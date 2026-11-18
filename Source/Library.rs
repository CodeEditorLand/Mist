#![allow(
	non_snake_case,
	non_camel_case_types,
	non_upper_case_globals,
	dead_code,
	unused_imports,
	unused_variables,
	unused_assignments,
	clippy::tabs_in_doc_comments,
	clippy::unnecessary_lazy_evaluations
)]
//! # Mist: Private DNS for Local-First Networking
//!
//! Mist gives Land its own private DNS so editor components can find each
//! other on `*.editor.land` without touching the public internet. All
//! queries resolve to `127.0.0.1`. No external DNS leaks, no configuration
//! needed.
//!
//! ## Features
//!
//! - **Private DNS Zone**: Authoritative zone for `*.editor.land` domains
//! - **Local Resolution**: All editor.land queries resolve to `127.0.0.1`.
//! - **Dynamic Port Allocation**: Automatically finds available ports using
//!   portpicker
//! - **Async/Sync Support**: Both async and blocking server implementations
//! - **WebSocket Transport**: Local-first JSON-RPC over WebSocket for direct
//!   Sky↔Cocoon IPC
//! - **Forward Security**: Allowlist-based DNS forwarding to prevent sidecars
//!   from reaching arbitrary external hosts
//!
//! ## Example
//!
//! ```rust,no_run
//! use Mist::start;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//! 	// Start the DNS server (tries port 5353 first, then finds an available one)
//! 	let port = start(5353)?;
//!
//! 	println!("DNS server running on port {}", port);
//!
//! 	// The server runs in the background
//! 	// Use DNS_PORT to get the port number elsewhere
//!
//! 	Ok(())
//! }
//! ```
//!
//! ## Modules
//!
//! - [`Server`]: DNS server builder and async/sync serving
//! - [`Zone`]: Authoritative zone configuration for `editor.land`
//! - [`Resolver`]: Stub and reqwest-compatible DNS resolvers
//! - [`ForwardSecurity`]: Allowlist-based DNS forward security
//! - [`WebSocket`]: JSON-RPC over WebSocket transport layer
//!
//! ## Links
//!
//! - [Repository](https://github.com/CodeEditorLand/Mist)
//! - [Architecture](Documentation/GitHub/Architecture.md)

use std::{
	net::{IpAddr, Ipv4Addr, SocketAddr},
	sync::mpsc,
	thread,
};

use anyhow::Result;
use hickory_server::Server as Hickory;
use once_cell::sync::OnceCell;
use tokio::net::{TcpListener as Listener, UdpSocket as Socket};

// Public module exports (PascalCase per project convention)
pub mod Server;

pub mod Zone;

pub mod Resolver;

pub mod ForwardSecurity;

// LAND-PATCH B7-S6 P1: WebSocket transport for the Sky↔Cocoon
pub mod WebSocket;

/// Global DNS port number.
///
/// Stores the port number that the DNS server is running on.
/// Set once when [`start`] is called and remains constant thereafter.
///
/// # Example
///
/// ```rust
/// use Mist::dns_port;
/// let port = dns_port();
/// ```
pub static DNS_PORT:OnceCell<u16> = OnceCell::new();

/// Returns the DNS port number.
///
/// Provides the port that the DNS server is listening on, or `0` if the
/// server has not been started yet.
///
/// ## Returns
///
/// The port number (0-65535), or `0` if the server hasn't been started.
///
/// ## Example
///
/// ```rust
/// use Mist::dns_port;
/// let port = dns_port();
/// if port > 0 {
/// 	println!("DNS server is running on port {}", port);
/// } else {
/// 	println!("DNS server has not been started");
/// }
/// ```
pub fn dns_port() -> u16 { *DNS_PORT.get().unwrap_or(&0) }

/// Starts the DNS server for the CodeEditorLand private network.
///
/// Performs the following steps:
/// 1. Binds the DNS sockets to `127.0.0.1:{preferred_port}`, or to an
///    OS-assigned ephemeral port when that address is unavailable or
///    `preferred_port` is `0`
/// 2. Builds the DNS catalog with the `editor.land` zone
/// 3. Spawns the DNS server as a background task and waits for it to report
///    the port it actually bound
/// 4. Sets the `DNS_PORT` global variable to that real port
/// 5. Returns the real bound port
///
/// The DNS server runs in the background and can be stopped by dropping
/// the application.
///
/// ## Parameters
///
/// * `preferred_port` - The preferred port number to use. If this port is
///   already in use, portpicker will find an alternative available port.
///
/// ## Returns
///
/// `Ok(port)` with the port number the server is listening on,
/// or an error if the server failed to start.
///
/// ## Example
///
/// ```rust,no_run
/// use Mist::start;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
/// 	// Start DNS server, preferring port 5353
/// 	let port = start(5353)?;
/// 	println!("DNS server started on port {}", port);
/// 	tokio::signal::ctrl_c().await?;
/// 	Ok(())
/// }
/// ```
pub fn start(preferred_port:u16) -> Result<u16> {
	// A second bind would leave two servers fighting over the one global
	// port. Refuse early so the caller can fall back instead of getting a
	// port that no server actually owns.
	match DNS_PORT.get() {
		Some(_) => {
			eprintln!("warn: [Mist] DNS server already started; refusing a second start()");

			return Err(anyhow::anyhow!("DNS server already started"));
		},

		None => {},
	}

	// The detached server reports the port it actually bound over this
	// channel. `start` blocks on the receive, so it can never advertise
	// success before the sockets are live, and a panic in the detached task
	// drops the sender, which we surface as an error instead of a false
	// success on an unbound server.
	let (Sender, Receiver) = mpsc::channel::<Result<u16>>();

	thread::spawn(move || {
		let Runtime = match tokio::runtime::Runtime::new() {
			Ok(Built) => Built,

			Err(Error) => {
				eprintln!("warn: [Mist] failed to build DNS runtime: {}", Error);
				let _ = Sender.send(Err(anyhow::anyhow!("failed to build DNS runtime: {}", Error)));

				return;
			},
		};

		Runtime.block_on(async move {
			let Loopback = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

			// Bind once and keep the socket: this is the authoritative bind.
			// There is no longer a throwaway probe that can disagree with it.
			let UDPSocket = match Socket::bind(SocketAddr::new(Loopback, preferred_port)).await {
				Ok(Bound) => Bound,

				Err(Error) => {
					// A requested port already taken by another process is a
					// normal non-happy path: warn and let the OS pick a free
					// ephemeral port instead of failing the whole host.
					eprintln!(
						"warn: [Mist] preferred DNS port {} unavailable ({}); selecting an ephemeral port",
						preferred_port,
						Error
					);

					match Socket::bind(SocketAddr::new(Loopback, 0)).await {
						Ok(Bound) => Bound,

						Err(Error) => {
							eprintln!("warn: [Mist] failed to bind DNS UDP socket: {}", Error);
							let _ = Sender.send(Err(anyhow::anyhow!("failed to bind DNS UDP socket: {}", Error)));

							return;
						},
					}
				},
			};

			let Port = match UDPSocket.local_addr() {
				Ok(Address) => Address.port(),

				Err(Error) => {
					eprintln!("warn: [Mist] failed to read bound DNS UDP port: {}", Error);
					let _ = Sender.send(Err(anyhow::anyhow!("failed to read bound DNS UDP port: {}", Error)));

					return;
				},
			};

			// A zero means the socket did not really bind; never report it as
			// a successful start.
			match Port {
				0 => {
					eprintln!("warn: [Mist] DNS socket reported port 0 after bind; refusing to advertise success");
					let _ = Sender.send(Err(anyhow::anyhow!("DNS socket bound to port 0")));

					return;
				},

				_ => {},
			}

			let Address = SocketAddr::new(Loopback, Port);

			// UDP and TCP use separate protocol namespaces, so the same
			// numeric port is valid for both, exactly as in Server::Serve.
			let TCPListener = match Listener::bind(Address).await {
				Ok(Bound) => Bound,

				Err(Error) => {
					eprintln!("warn: [Mist] failed to bind DNS TCP listener on {}: {}", Address, Error);
					let _ = Sender.send(Err(anyhow::anyhow!(
						"failed to bind DNS TCP listener on {}: {}",
						Address,
						Error
					)));

					return;
				},
			};

			let Catalog = match Server::BuildCatalog(Port) {
				Ok(Built) => Built,

				Err(Error) => {
					eprintln!("warn: [Mist] failed to build DNS catalog: {}", Error);
					let _ = Sender.send(Err(Error));

					return;
				},
			};

			let mut Service = Hickory::new(Catalog);
			Service.register_socket(UDPSocket);
			Service.register_listener(TCPListener, std::time::Duration::from_secs(5), 65_535);

			// Report the real bound port only now, after both sockets are
			// registered and the server is ready to run.
			match Sender.send(Ok(Port)) {
				Err(_) => eprintln!(
					"warn: [Mist] DNS start() receiver dropped before the bound port was delivered"
				),

				Ok(_) => {},
			}

			match Service.block_until_done().await {
				Err(Error) => eprintln!("warn: [Mist] DNS server stopped: {:?}", Error),

				Ok(_) => {},
			}
		});
	});

	// Block until the detached server has bound and reported its real port.
	// A closed channel means the task panicked before binding: never claim
	// success on an unbound server, and never let the panic reach the caller.
	match Receiver.recv() {
		Ok(Ok(BoundPort)) => match DNS_PORT.set(BoundPort) {
			Ok(_) => Ok(BoundPort),

			Err(_) => Err(anyhow::anyhow!("DNS port has already been set")),
		},

		Ok(Err(Error)) => {
			eprintln!("warn: [Mist] DNS server failed to bind: {}", Error);

			Err(Error)
		},

		Err(Error) => {
			eprintln!(
				"warn: [Mist] DNS server task ended before binding ({}); reporting failure",
				Error
			);

			Err(anyhow::anyhow!("DNS server task ended before binding: {}", Error))
		},
	}
}

#[cfg(test)]
mod tests {

	use super::*;

	// The DNS_PORT OnceCell is process-wide. Keeping the three assertions
	// that depend on its state in one test makes them independent of test
	// execution order (Rust runs tests concurrently in one process).
	#[test]
	fn test_dns_port_lifecycle() {
		assert_eq!(dns_port(), 0, "DNS_PORT must start unset");

		// Use a non-standard port for testing.
		let preferred_port = 15353;

		let port = start(preferred_port).expect("Failed to start DNS server");

		assert!(port >= 1024, "Port should be >= 1024");

		assert!(port <= 65535, "Port should be <= 65535");

		assert_eq!(dns_port(), port, "DNS_PORT should match returned port");

		// The global is set exactly once; a second start must be rejected.
		assert!(start(15355).is_err(), "Second start should fail");
	}

	#[test]
	fn test_build_catalog_api() {
		let catalog = Server::BuildCatalog(15356);

		assert!(catalog.is_ok(), "Should be able to build catalog");
	}

	#[test]
	fn test_build_zone_api() {
		let zone = Zone::EditorLandZone();

		assert!(zone.is_ok(), "Should be able to build zone");
	}
}
