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
//! children) — current implementation logs and ignores them.
//!
//! # Auth
//!
//! Per-spawn 32-byte shared secret. Native clients send it in the
//! WebSocket upgrade `X-Land-Secret` header; browser clients (Sky)
//! cannot set custom upgrade headers, so the server also accepts
//! the hex secret as a `?secret=<hex>` URL query parameter or as a
//! `Sec-WebSocket-Protocol` subprotocol entry (echoed back in the
//! accept response, as RFC 6455 requires). Connections presenting
//! none of the three are rejected with `403 Forbidden`. Mountain
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
	sync::{Mutex, RwLock, oneshot},
};
use tokio_tungstenite::{
	MaybeTlsStream,
	WebSocketStream,
	accept_async,
	accept_hdr_async,
	connect_async,
	tungstenite::{
		Message,
		Utf8Bytes,
		client::IntoClientRequest,
		handshake::server::{ErrorResponse, Request, Response},
		http::{HeaderValue, StatusCode},
	},
};

/// Per-spawn shared secret for WebSocket connection authentication.
///
/// A cryptographically random 32-byte value used to authenticate
/// WebSocket upgrade requests from native clients. Exchange happens
/// out-of-band (environment variable from Mountain to Cocoon,
/// Tauri invoke from Mountain to Sky).
#[derive(Clone)]
pub struct SharedSecret(pub [u8; 32]);

impl SharedSecret {
	/// Generates a cryptographically random 32-byte shared secret.
	///
	/// Uses the thread-local RNG from `rand` 0.10 via `rand::random`.
	///
	/// ## Returns
	///
	/// A new `SharedSecret` with 32 random bytes.
	pub fn random() -> Self {
		// rand 0.10: `rand::random::<[u8; N]>()` fills via the
		// thread-local RNG without needing the deprecated
		// `RngCore::fill_bytes` import path.
		Self(rand::random::<[u8; 32]>())
	}

	/// Returns the secret as a hex-encoded string.
	///
	/// Each byte is encoded as two hexadecimal characters, producing a
	/// 64-character string. Useful for transmitting the secret over HTTP
	/// headers or environment variables.
	///
	/// ## Returns
	///
	/// A 64-character hex string.
	pub fn as_hex(&self) -> String { hex::encode(self.0) }

	/// Parses a hex-encoded string back into a `SharedSecret`.
	///
	/// The input must be exactly 64 hexadecimal characters (32 bytes).
	/// Returns an error if the length is wrong or the string contains
	/// invalid hex characters.
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

/// Fallback handler signature. Receives the method name alongside the
/// params so a single closure can forward any unregistered method into
/// an existing dispatcher.
pub type DefaultHandlerFn =
	Arc<dyn Fn(String, Value) -> futures_util::future::BoxFuture<'static, Result<Value, String>> + Send + Sync>;

/// Method dispatch table.
///
/// Lookups are read-dominated (one per inbound frame) so the maps sit
/// behind `RwLock`, not `Mutex` — concurrent connections never serialize
/// on dispatch.
#[derive(Default)]
pub struct HandlerRegistry {
	Handlers:RwLock<HashMap<String, HandlerFn>>,

	DefaultHandler:RwLock<Option<DefaultHandlerFn>>,
}

impl HandlerRegistry {
	/// Builds a new, empty `HandlerRegistry` wrapped in an `Arc`.
	///
	/// The registry starts with no methods registered. Use
	/// [`Register`](Self::Register) to add handlers.
	pub fn new() -> Arc<Self> { Arc::new(Self::default()) }

	/// Registers a handler function for the given method name.
	///
	/// When a JSON-RPC request arrives with `method` matching `Method`,
	/// the `Handler` closure is invoked with the params `Value`.
	/// Replaces any previously registered handler for the same name.
	pub async fn Register(&self, Method:String, Handler:HandlerFn) {
		self.Handlers.write().await.insert(Method, Handler);
	}

	/// Registers the fallback handler invoked for any method that has
	/// no per-method registration. Replaces any previous fallback.
	pub async fn RegisterDefault(&self, Handler:DefaultHandlerFn) {
		*self.DefaultHandler.write().await = Some(Handler);
	}

	/// Looks up a handler function by method name.
	///
	/// Returns `None` if no handler has been registered for the given
	/// method.
	pub async fn Lookup(&self, Method:&str) -> Option<HandlerFn> { self.Handlers.read().await.get(Method).cloned() }

	/// Returns the fallback handler, if one has been registered.
	pub async fn LookupDefault(&self) -> Option<DefaultHandlerFn> { self.DefaultHandler.read().await.clone() }
}

/// Runs a WebSocket server on `127.0.0.1:<port>`. Loops forever
/// accepting connections; spawns a task per connection.
///
/// `Secret:Some(_)` enforces upgrade-time auth (header, query
/// parameter, or subprotocol — see the module docs); `Secret:None`
/// accepts every loopback connection unchecked.
///
/// # Parameters
///
/// * `Port` — The local port to bind the WebSocket listener to.
/// * `Secret` — Optional shared secret for upgrade-time authentication.
/// * `Registry` — The method dispatch table for JSON-RPC handlers.
///
/// # Returns
///
/// `Err` only on bind failure; per-connection errors are logged but
/// never propagated (a single bad client must not kill the listener).
pub async fn ServeLocal(Port:u16, Secret:Option<SharedSecret>, Registry:Arc<HandlerRegistry>) -> Result<()> {
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

/// Compares a presented credential against the expected hex secret
/// in constant time.
///
/// The byte loop always runs over the full expected length so a
/// mismatch never leaks its position; a length mismatch also fails
/// via the same accumulator.
fn CredentialMatches(Candidate:&str, ExpectedHex:&str) -> bool {
	let CandidateBytes = Candidate.as_bytes();

	let ExpectedBytes = ExpectedHex.as_bytes();

	let mut Difference = CandidateBytes.len() ^ ExpectedBytes.len();

	for (Index, Expected) in ExpectedBytes.iter().enumerate() {
		Difference |= usize::from(CandidateBytes.get(Index).copied().unwrap_or(0) ^ Expected);
	}

	Difference == 0
}

/// Authorizes a WebSocket upgrade request by checking the shared
/// secret.
///
/// Accepts the request when the hex secret is presented via the
/// `X-Land-Secret` header, a `?secret=<hex>` query parameter, or a
/// `Sec-WebSocket-Protocol` subprotocol entry (browser clients).
/// Matched subprotocols are echoed back, as RFC 6455 requires for
/// the browser to keep the connection open.
fn AuthorizeUpgrade(
	RequestValue:&Request,
	mut ResponseValue:Response,
	ExpectedHex:&str,
) -> Result<Response, ErrorResponse> {
	let HeaderMatch = RequestValue
		.headers()
		.get("X-Land-Secret")
		.and_then(|V| V.to_str().ok())
		.map(|V| CredentialMatches(V, ExpectedHex))
		.unwrap_or(false);

	let QueryMatch = RequestValue
		.uri()
		.query()
		.map(|Query| {
			Query
				.split('&')
				.any(|Pair| Pair.strip_prefix("secret=").map(|V| CredentialMatches(V, ExpectedHex)).unwrap_or(false))
		})
		.unwrap_or(false);

	let ProtocolMatch = RequestValue
		.headers()
		.get("Sec-WebSocket-Protocol")
		.and_then(|V| V.to_str().ok())
		.map(|List| List.split(',').any(|Entry| CredentialMatches(Entry.trim(), ExpectedHex)))
		.unwrap_or(false);

	if !(HeaderMatch || QueryMatch || ProtocolMatch) {
		tracing::warn!(target: "Mist::WebSocket", "upgrade rejected: missing or invalid shared secret");

		let mut Rejection = ErrorResponse::new(Some("Forbidden: invalid or missing X-Land-Secret".to_string()));

		*Rejection.status_mut() = StatusCode::FORBIDDEN;

		return Err(Rejection);
	}

	if ProtocolMatch {
		if let Ok(Echo) = HeaderValue::from_str(ExpectedHex) {
			ResponseValue.headers_mut().insert("Sec-WebSocket-Protocol", Echo);
		}
	}

	Ok(ResponseValue)
}

async fn HandleConnection(Stream:TcpStream, Secret:Option<SharedSecret>, Registry:Arc<HandlerRegistry>) -> Result<()> {
	let WebSocketStream = match Secret {
		Some(Secret) => {
			let ExpectedHex = Secret.as_hex();

			accept_hdr_async(Stream, move |RequestValue:&Request, ResponseValue:Response| {
				AuthorizeUpgrade(RequestValue, ResponseValue, &ExpectedHex)
			})
			.await?
		},

		None => accept_async(Stream).await?,
	};

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
						match Registry.LookupDefault().await {
							Some(Default) => {
								match Default(Method.to_string(), Params).await {
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
						}
					},
				};

				if Identifier.is_null() {
					// Notification — no response expected.
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

/// Maps request identifiers to their response senders.
type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;

/// Client-side WebSocket connection.
///
/// Holds the write half of the WebSocket and a map of pending
/// requests keyed by identifier.
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
	/// Connects to a Mist WebSocket server at `Address`
	/// (e.g. `ws://127.0.0.1:5051`). Spawns a background reader
	/// task that drains incoming frames and resolves pending
	/// requests.
	///
	/// # Parameters
	///
	/// * `Address` — The WebSocket server URL (ws:// or wss:// scheme).
	///
	/// # Returns
	///
	/// A new `Client` instance wrapped in `Arc`.
	pub async fn connect(Address:&str) -> Result<Arc<Self>> {
		let (Stream, _Response) = connect_async(Address).await?;

		Ok(Self::FromStream(Stream))
	}

	/// Connects with the per-spawn shared secret attached as the
	/// `X-Land-Secret` upgrade header. Native clients (Grove, Cocoon)
	/// use this against a secret-enforcing [`ServeLocal`] server;
	/// browser clients use the query-parameter/subprotocol forms
	/// instead since they cannot set upgrade headers.
	///
	/// # Parameters
	///
	/// * `Address` — The WebSocket server URL (ws:// or wss:// scheme).
	/// * `Secret` — The shared secret for upgrade-time authentication.
	///
	/// # Returns
	///
	/// A new `Client` instance wrapped in `Arc`.
	pub async fn ConnectWithSecret(Address:&str, Secret:&SharedSecret) -> Result<Arc<Self>> {
		let mut RequestValue = Address.into_client_request()?;

		RequestValue.headers_mut().insert("X-Land-Secret", HeaderValue::from_str(&Secret.as_hex())?);

		let (Stream, _Response) = connect_async(RequestValue).await?;

		Ok(Self::FromStream(Stream))
	}

	/// Wraps an established WebSocket stream in a `Client`: splits the
	/// stream, spawns the reader task, and wires the pending-request
	/// map.
	fn FromStream(Stream:WebSocketStream<MaybeTlsStream<TcpStream>>) -> Arc<Self> {
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

		SelfReference
	}

	/// Invokes a remote method.
	///
	/// Returns the result Value or an error string. Pending requests
	/// are tracked by identifier; on disconnect the future resolves
	/// with `Err("connection closed")`.
	///
	/// # Parameters
	///
	/// * `Method` — The JSON-RPC method name.
	/// * `Params` — The JSON parameter value.
	///
	/// # Returns
	///
	/// The result `Value` on success, or an error `String`.
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

	/// Sends a one-way notification (no response expected).
	///
	/// # Parameters
	///
	/// * `Method` — The JSON-RPC method name.
	/// * `Params` — The JSON parameter value.
	///
	/// # Returns
	///
	/// `Ok(())` on success, or an error `String`.
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

	/// Returns `true` if the WebSocket connection has been closed.
	///
	/// Once closed, further [`invoke`](Self::invoke) and
	/// [`notify`](Self::notify) calls return
	/// `Err("connection closed")` immediately. A new `Client` must be
	/// created via [`connect`](Self::connect) to re-establish the
	/// channel.
	pub fn is_closed(&self) -> bool { self.Closed.load(Ordering::Relaxed) }
}
