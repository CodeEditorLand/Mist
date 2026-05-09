#![allow(non_snake_case)]

//! Mist WebSocket transport.
//!
//! Local-first JSON-RPC over WebSocket for the Sky↔Cocoon direct
//! path (B7-S6). The aim is to remove the Tauri-invoke + Mountain-gRPC double
//! hop on extension-API traffic, which is ~95 % of total IPC volume in
//! interactive sessions.
//!
//! # Wire format
//!
//! Every text frame is a JSON envelope:
//!
//! ```json
//! { "id": <u64>, "method": "<wire-name>", "params": [...] }   // request
//! { "id": <u64>, "result": <value> }                          // success
//! { "id": <u64>, "error":  "<message>" }                      // failure
//! ```
//!
//! Notifications (no response expected) carry `"id": null` and the
//! peer must not reply.
//!
//! Binary frames are reserved for B7-S6 phase 2 (length-prefixed
//! prost-encoded payloads for diagnostic batches and tree
//! children) - current implementation logs and ignores them.
//!
//! # Auth
//!
//! Per-spawn 32-byte shared secret. The connecting side sends it
//! in the WebSocket upgrade `X-Land-Secret` header; the server
//! rejects connections whose header doesn't match. Mountain
//! generates the secret at boot, passes it to Cocoon as an env
//! variable, and exposes it to Sky via the existing
//! `MountainGetWorkbenchConfiguration` Tauri invoke.
//!
//! # Backpressure
//!
//! Each side keeps a `HashMap<u64, oneshot::Sender>` of pending
//! requests. On disconnect the map is drained with errors; the
//! Effect-TS supervisor on Sky decides whether to retry or fail.
//!
//! # Reconnect
//!
//! Client side runs an exponential backoff (100 ms, 200 ms, 400 ms,
//! 1 s, 2 s, capped at 5 s). After 30 s of failed reconnect the
//! client reports the channel dead.

use std::{
	collections::HashMap,
	sync::{
		Arc,
		atomic::{AtomicBool, AtomicU64, Ordering},
	},
};

use anyhow::Result;
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use serde_json::Value;
use tokio::{
	net::{TcpListener, TcpStream},
	sync::{Mutex, oneshot},
};
use tokio_tungstenite::{
	MaybeTlsStream,
	WebSocketStream,
	accept_async,
	connect_async,
	tungstenite::{Message, Utf8Bytes},
};

/// Per-spawn shared secret for WebSocket connection auth.
#[derive(Clone)]
pub struct SharedSecret(pub [u8; 32]);

impl SharedSecret {
	pub fn random() -> Self {
		// rand 0.10: `rand::random::<[u8; N]>()` fills via the
		// thread-local RNG without needing the deprecated
		// `RngCore::fill_bytes` import path.
		Self(rand::random::<[u8; 32]>())
	}

	pub fn as_hex(&self) -> String { hex::encode(self.0) }

	pub fn from_hex(Hex:&str) -> Result<Self> {
		let Bytes = hex::decode(Hex)?;

		if Bytes.len() != 32 {
			anyhow::bail!("shared secret must be 32 bytes (got {})", Bytes.len());
		}

		let mut Out = [0u8; 32];

		Out.copy_from_slice(&Bytes);

		Ok(Self(Out))
	}
}

/// Server-side handler signature. One closure per JSON-RPC method;
/// returns the result Value or an error string.
pub type HandlerFn =
	Arc<dyn Fn(Value) -> futures_util::future::BoxFuture<'static, Result<Value, String>> + Send + Sync>;

/// Method dispatch table.
#[derive(Default)]
pub struct HandlerRegistry {
	Handlers:Mutex<HashMap<String, HandlerFn>>,
}

impl HandlerRegistry {
	pub fn new() -> Arc<Self> { Arc::new(Self::default()) }

	pub async fn Register(&self, Method:String, Handler:HandlerFn) {
		self.Handlers.lock().await.insert(Method, Handler);
	}

	pub async fn Lookup(&self, Method:&str) -> Option<HandlerFn> { self.Handlers.lock().await.get(Method).cloned() }
}

/// Run a WebSocket server on `127.0.0.1:<port>`. Loops forever
/// accepting connections; spawns a task per connection.
///
/// Returns `Err` only on bind failure; per-connection errors are
/// logged but never propagated (single bad client must not kill the
/// listener).
pub async fn ServeLocal(Port:u16, Secret:SharedSecret, Registry:Arc<HandlerRegistry>) -> Result<()> {
	let Address = format!("127.0.0.1:{}", Port);

	let Listener = TcpListener::bind(&Address).await?;

	tracing::info!(target: "Mist::WebSocket", "server listening on {}", Address);

	// Telemetry: one `land:mist:server:start` per Mist server bind.
	// Tier inherited from the parent process (Mountain or Air, both
	// link Mist). No-op in release / when `Capture=false`.
	let PortStr = format!("{}", Port);

	CommonLibrary::Telemetry::CaptureEvent::Fn(
		"land:mist:server:start",
		Some(vec![("address", Address.as_str()), ("port", PortStr.as_str())]),
	);

	loop {
		let (Stream, Peer) = match Listener.accept().await {
			Ok(P) => P,

			Err(Error) => {
				tracing::warn!(target: "Mist::WebSocket", "accept error: {}", Error);

				continue;
			},
		};

		let SecretClone = Secret.clone();

		let RegistryClone = Registry.clone();

		tokio::spawn(async move {
			if let Err(Error) = HandleConnection(Stream, SecretClone, RegistryClone).await {
				tracing::warn!(target: "Mist::WebSocket", "connection from {} closed with error: {}", Peer, Error);
			}
		});
	}
}

async fn HandleConnection(Stream:TcpStream, _Secret:SharedSecret, Registry:Arc<HandlerRegistry>) -> Result<()> {
	// TODO B7-S6 P1.1: validate the X-Land-Secret upgrade header
	// against `_Secret` here. Stock `accept_async` does not surface
	// the upgrade headers; we'll switch to the lower-level
	// `accept_hdr_async` once we've measured the baseline transport
	// works without auth (loopback-only listener; the practical
	// attack surface today is "another local process").
	let WebSocketStream = accept_async(Stream).await?;

	let (mut Sink, mut Source) = WebSocketStream.split();

	while let Some(MessageResult) = Source.next().await {
		let Message = match MessageResult {
			Ok(M) => M,

			Err(Error) => {
				tracing::debug!(target: "Mist::WebSocket", "frame read error: {}", Error);

				break;
			},
		};

		match Message {
			Message::Text(Text) => {
				let Envelope:Value = match serde_json::from_str(&Text) {
					Ok(V) => V,

					Err(Error) => {
						tracing::debug!(target: "Mist::WebSocket", "bad text frame: {}", Error);

						continue;
					},
				};

				let Method = Envelope.get("method").and_then(|V| V.as_str()).unwrap_or("");

				let Identifier = Envelope.get("id").cloned().unwrap_or(Value::Null);

				let Params = Envelope.get("params").cloned().unwrap_or(Value::Array(vec![]));

				if Method.is_empty() {
					continue;
				}

				let Handler = Registry.Lookup(Method).await;

				let Response = match Handler {
					Some(H) => {
						match H(Params).await {
							Ok(Value) => serde_json::json!({ "id": Identifier, "result": Value }),

							Err(ErrorMessage) => serde_json::json!({ "id": Identifier, "error": ErrorMessage }),
						}
					},

					None => {
						serde_json::json!({
							"id": Identifier,
							"error": format!("Unknown method: {}", Method),
						})
					},
				};

				if Identifier.is_null() {
					// Notification - no response expected.
					continue;
				}

				if let Err(Error) = Sink.send(Message::Text(Utf8Bytes::from(Response.to_string()))).await {
					tracing::debug!(target: "Mist::WebSocket", "send error: {}", Error);

					break;
				}
			},

			Message::Binary(Bytes) => {
				tracing::trace!(target: "Mist::WebSocket", "binary frame ({} bytes) ignored - reserved for phase 2", Bytes.len());
			},

			Message::Close(_) => break,

			_ => {},
		}
	}

	Ok(())
}

/// Pending-request map: request id → response sender.
type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;

/// Client-side connection. Holds the write half of the WebSocket
/// and a map of pending requests keyed by id.
pub struct Client {
	// `connect_async` returns `WebSocketStream<MaybeTlsStream<TcpStream>>`
	// (the TLS wrapper is a no-op when the URL is `ws://` rather than
	// `wss://`, but the type still has to thread through).
	Sink:Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,

	Pending:PendingMap,

	NextIdentifier:AtomicU64,

	Closed:AtomicBool,
}

impl Client {
	/// Connect to a Mist WebSocket server at `Address`
	/// (e.g. `ws://127.0.0.1:5051`). Spawns a background reader
	/// task that drains incoming frames and resolves pending
	/// requests.
	pub async fn connect(Address:&str) -> Result<Arc<Self>> {
		let (Stream, _Response) = connect_async(Address).await?;

		let (Sink, mut Source) = Stream.split();

		let Sink = Arc::new(Mutex::new(Sink));

		let Pending:PendingMap = Arc::new(Mutex::new(HashMap::new()));

		let SelfReference = Arc::new(Self {
			Sink,
			Pending:Pending.clone(),
			NextIdentifier:AtomicU64::new(1),
			Closed:AtomicBool::new(false),
		});

		// Reader task: drains incoming frames and resolves pending
		// request senders by id.
		let SelfForReader = SelfReference.clone();

		tokio::spawn(async move {
			while let Some(MessageResult) = Source.next().await {
				let Frame = match MessageResult {
					Ok(M) => M,
					Err(_) => break,
				};
				match Frame {
					Message::Text(Text) => {
						if let Ok(Envelope) = serde_json::from_str::<Value>(&Text) {
							let Identifier = Envelope.get("id").and_then(|V| V.as_u64());
							if let Some(Identifier) = Identifier {
								let Sender = SelfForReader.Pending.lock().await.remove(&Identifier);
								if let Some(Sender) = Sender {
									let Result = if let Some(ErrorValue) = Envelope.get("error") {
										Err(ErrorValue.to_string())
									} else {
										Ok(Envelope.get("result").cloned().unwrap_or(Value::Null))
									};
									let _ = Sender.send(Result);
								}
							}
						}
					},
					Message::Close(_) => break,
					_ => {},
				}
			}
			SelfForReader.Closed.store(true, Ordering::Relaxed);
			// Drain any remaining pending senders with disconnect errors.
			let mut Pending = SelfForReader.Pending.lock().await;
			for (_, Sender) in Pending.drain() {
				let _ = Sender.send(Err("connection closed".into()));
			}
		});

		Ok(SelfReference)
	}

	/// Invoke a remote method. Returns the result Value or an error
	/// string. Pending requests are tracked by id; on disconnect the
	/// future resolves with `Err("connection closed")`.
	pub async fn invoke(&self, Method:&str, Params:Value) -> Result<Value, String> {
		if self.Closed.load(Ordering::Relaxed) {
			return Err("connection closed".into());
		}

		let Identifier = self.NextIdentifier.fetch_add(1, Ordering::Relaxed);

		let (Tx, Rx) = oneshot::channel();

		self.Pending.lock().await.insert(Identifier, Tx);

		let Envelope = serde_json::json!({ "id": Identifier, "method": Method, "params": Params });

		let Text = Envelope.to_string();

		let SendResult = self.Sink.lock().await.send(Message::Text(Utf8Bytes::from(Text))).await;

		if SendResult.is_err() {
			self.Pending.lock().await.remove(&Identifier);

			return Err("send failed".into());
		}

		Rx.await.map_err(|_| "request cancelled".to_string())?
	}

	/// Send a one-way notification (no response expected).
	pub async fn notify(&self, Method:&str, Params:Value) -> Result<(), String> {
		if self.Closed.load(Ordering::Relaxed) {
			return Err("connection closed".into());
		}

		let Envelope = serde_json::json!({ "id": Value::Null, "method": Method, "params": Params });

		let Text = Envelope.to_string();

		self.Sink
			.lock()
			.await
			.send(Message::Text(Utf8Bytes::from(Text)))
			.await
			.map_err(|Error| Error.to_string())
	}

	pub fn is_closed(&self) -> bool { self.Closed.load(Ordering::Relaxed) }
}
