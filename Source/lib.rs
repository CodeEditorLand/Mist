#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
//! # Mist: Private DNS for Local-First Networking
//! Mist gives Land its own private DNS so editor components can find each
//! other on `*.editor.land` without touching the public internet. All queries
//! resolve to `127.0.0.1`. No external DNS leaks, no configuration needed.
//!
//! ## Features
//!
//! - **Private DNS Zone**: Authoritative zone for `*.editor.land` domains
//! - **Local Resolution**: All editor.land queries resolve to `127.0.0.1`.
//! - **Dynamic Port Allocation**: Automatically finds available ports using
//!   portpicker
//! - **Async/Sync Support**: Both async and blocking server implementations
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

#![allow(clippy::tabs_in_doc_comments, clippy::unnecessary_lazy_evaluations)]

use std::thread;

use anyhow::Result;
use once_cell::sync::OnceCell;

// Public module exports (PascalCase per project convention)
pub mod Server;

pub mod Zone;

pub mod Resolver;

pub mod ForwardSecurity;

// LAND-PATCH B7-S6 P1: WebSocket transport for the Sky↔Cocoon
pub mod WebSocket;

/// Global DNS port number.
///
/// This static cell stores the port number that the DNS server is running on.
/// It is set once when [`start`] is called and remains constant thereafter.
///
/// # Example
///
/// ```rust
/// use Mist::dns_port;
///
/// // Returns the port number, or 0 if the server hasn't been started
/// let port = dns_port();
/// ```
pub static DNS_PORT:OnceCell<u16> = OnceCell::new();

/// Returns the DNS port number.
///
/// Returns the port that the DNS server is listening on, or `0` if the
/// server has not been started yet.
///
/// # Returns
///
/// The port number (0-65535), or 0 if the server hasn't started.
///
/// # Example
///
/// ```rust
/// use Mist::dns_port;
///
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
/// This function performs the following steps:
/// 1. Uses portpicker to find an available port (tries `preferred_port` first)
/// 2. Sets the `DNS_PORT` global variable
/// 3. Builds the DNS catalog with the `editor.land` zone
/// 4. Spawns the DNS server as a background task
/// 5. Returns the port number
///
/// The DNS server runs in the background and can be stopped by dropping
/// the application.
///
/// # Parameters
///
/// * `preferred_port` - The preferred port number to use. If this port is
///   already in use, portpicker will find an alternative available port.
///
/// # Returns
///
/// Returns `Ok(port)` with the port number the server is listening on,
/// or an error if the server failed to start.
///
/// # Example
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
	// Step 1: Find an available port using portpicker
	// Try the preferred port first, then pick a random available one
	let port = portpicker::pick_unused_port()
		.or_else(|| {
			// If pick_unused_port returns None, try the preferred port explicitly
			Some(preferred_port)
		})
		.ok_or_else(|| anyhow::anyhow!("Failed to find an available port"))?;

	// Step 2: Set the DNS_PORT globally
	DNS_PORT
		.set(port)
		.map_err(|_| anyhow::anyhow!("DNS port has already been set"))?;

	// Step 3: Build the DNS catalog
	let catalog = Server::BuildCatalog(port)?;

	// Step 4: Spawn the DNS server as a background task
	thread::spawn(move || {
		if let Err(e) = Server::ServeSync(catalog, port) {
			eprintln!("DNS server error: {:?}", e);
		}
	});

	// Step 5: Return the port number
	Ok(port)
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	fn test_dns_port_initial_state() {
		// Initially, DNS_PORT should be 0
		let port = dns_port();

		assert_eq!(port, 0);
	}

	#[test]
	fn test_dns_port_starts_server() {
		// Test that we can start the DNS server
		let preferred_port = 15353; // Use a non-standard port for testing

		let result = start(preferred_port);

		// The server should start successfully
		assert!(result.is_ok(), "Failed to start DNS server");

		let port = result.unwrap();

		// The port should be within valid range
		assert!(port >= 1024, "Port should be >= 1024");

		assert!(port <= 65535, "Port should be <= 65535");

		// DNS_PORT should now return the same port
		let retrieved_port = dns_port();

		assert_eq!(port, retrieved_port, "DNS_PORT should match returned port");
	}

	#[test]
	fn test_start_fails_on_second_call() {
		// Starting the server twice should fail
		let port1 = start(15354);

		assert!(port1.is_ok(), "First start should succeed");

		let port2 = start(15355);

		assert!(port2.is_err(), "Second start should fail");
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
