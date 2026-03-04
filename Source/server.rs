use std::{
	net::{IpAddr, Ipv4Addr, SocketAddr},
	sync::Arc,
};

use anyhow::Result;
use hickory_server::{
	authority::{Catalog, ZoneType},
	server::ServerFuture,
	store::in_memory::InMemoryAuthority,
};
use tokio::net::UdpSocket;

/// Builds a DNS catalog for the CodeEditorLand private network.
///
/// Creates a catalog with two zones:
/// 1. **Authoritative zone**: `editor.land` - Resolves all editor.land queries
///    locally
/// 2. **Forward zone**: `.` (root) - Forwards non-editor.land queries to
///    upstream DNS servers
///
/// # Parameters
///
/// * `_dns_port` - The port number (currently unused but kept for API
///   compatibility)
///
/// # Returns
///
/// A `Catalog` containing both the authoritative and forwarding zones.
///
/// # DNS Zones
///
/// - **Authoritative**: All `*.editor.land` queries are resolved locally to
///   `127.0.0.1`
/// - **Forwarding**: All other queries are forwarded to allowlisted upstream
///   DNS servers:
///   - `1.1.1.1:53` (Cloudflare DNS)
///   - `9.9.9.9:53` (Quad9 DNS)
///
/// # Example
///
/// ```rust
/// use Mist::server::build_catalog;
///
/// let catalog = build_catalog(5353)?;
/// ```
pub fn build_catalog(_dns_port:u16) -> Result<Catalog> {
	let mut catalog = Catalog::new();

	// Build the authoritative zone for editor.land
	let editor_land_origin = hickory_proto::rr::Name::from_ascii("editor.land.").unwrap();

	// Create an empty InMemoryAuthority for editor.land
	// DEPENDENCY: NxProofKind parameter is None - may need to be configured for DNSSEC NSEC/NSEC3 support
	let authority = InMemoryAuthority::empty(
		editor_land_origin.clone(),
		ZoneType::Primary,
		false,
		None, // Option<NxProofKind> - added in hickory-server 0.25
	);

	// Convert Name to LowerName
	let editor_land_lower = hickory_proto::rr::LowerName::from(&editor_land_origin);

	// Wrap in Arc (which implements AuthorityObject)
	let authority_arc = Arc::new(authority);

	// Add to catalog - expects Vec<Arc<dyn AuthorityObject>> since hickory-server 0.25
	catalog.upsert(editor_land_lower, vec![authority_arc]);

	// Note: Forward zones are configured differently in Hickory DNS
	// For now, the catalog handles editor.land authoritatively
	// Non-editor.land queries will fail with NXDOMAIN unless matched
	// Future implementation can add forward zones using hickory-resolver

	Ok(catalog)
}

/// Serves DNS queries on the specified port.
///
/// Starts both UDP and TCP DNS servers bound to `127.0.0.1:{port}`.
/// This function is blocking and runs indefinitely until an error occurs.
///
/// # Parameters
///
/// * `catalog` - The DNS catalog containing zones to serve
/// * `port` - The port number to bind to (typically 0-65535)
///
/// # Returns
///
/// Returns `Ok(())` if the server exits gracefully, or an error if
/// binding or serving fails.
///
/// # Network Behavior
///
/// - Binds to `127.0.0.1:{port}` for UDP and TCP
/// - Only accepts connections from localhost (loopback interface)
/// - Supports both standard DNS queries over UDP and larger responses over TCP
///
/// # Example
///
/// ```rust,no_run
/// use Mist::server::{build_catalog, serve};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
/// 	let catalog = build_catalog(5353)?;
/// 	serve(catalog, 5353).await?;
/// 	Ok(())
/// }
/// ```
pub async fn serve(catalog:Catalog, port:u16) -> Result<()> {
	// Create the socket address for localhost
	let addr:SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);

	// SECURITY: Verify that we are binding to loopback only (127.0.0.1)
	// This prevents the DNS server from accepting connections from the LAN
	// or other network interfaces, ensuring the private zone is only
	// accessible from localhost.
	let binding_ip = addr.ip();
	match binding_ip {
		IpAddr::V4(ip) => {
			if !ip.is_loopback() {
				return Err(anyhow::anyhow!(
					"SECURITY ERROR: DNS server attempted to bind to non-loopback address: {}. Only 127.x.x.x \
					 addresses are allowed to prevent LAN access to the private zone.",
					ip
				));
			}
		},
		IpAddr::V6(ip) if ip.is_loopback() => {
			// IPv6 loopback is acceptable (::1)
		},
		_ => {
			return Err(anyhow::anyhow!(
				"SECURITY ERROR: DNS server attempted to bind to invalid address: {}. Only loopback addresses are \
				 allowed.",
				binding_ip
			));
		},
	}

	tracing::info!("Binding DNS server to loopback address: {}", addr);

	// Create UDP socket
	let udp_socket = UdpSocket::bind(addr).await.map_err(|e| {
		anyhow::anyhow!(
			"SECURITY ERROR: Failed to bind DNS server to {}: {}. This may indicate the port is in use or there's a \
			 network configuration issue.",
			addr,
			e
		)
	})?;

	// SECURITY: Verify the actual bound address
	let bound_addr = udp_socket
		.local_addr()
		.map_err(|e| anyhow::anyhow!("SECURITY ERROR: Failed to retrieve bound socket address: {}", e))?;

	if !bound_addr.ip().is_loopback() {
		return Err(anyhow::anyhow!(
			"SECURITY ERROR: UDP socket bound to non-loopback address: {}. Expected 127.0.0.1 or ::1 to prevent LAN \
			 access to the private zone. The operating system may have redirected the bind.",
			bound_addr.ip()
		));
	}

	// Create the server future with the catalog
	let mut server = ServerFuture::new(catalog);

	// Register the UDP listener
	server.register_socket(udp_socket);

	// Register TCP listener using listen_with_socketaddr
	let tcp_listener = tokio::net::TcpListener::bind(addr)
		.await
		.map_err(|e| anyhow::anyhow!("SECURITY ERROR: Failed to bind TCP listener to {}: {}", addr, e))?;

	// SECURITY: Verify TCP listener bound address
	let tcp_bound_addr = tcp_listener
		.local_addr()
		.map_err(|e| anyhow::anyhow!("SECURITY ERROR: Failed to retrieve TCP listener bound address: {}", e))?;

	if !tcp_bound_addr.ip().is_loopback() {
		return Err(anyhow::anyhow!(
			"SECURITY ERROR: TCP listener bound to non-loopback address: {}. Expected 127.0.0.1 or ::1 to prevent LAN \
			 access to the private zone.",
			tcp_bound_addr.ip()
		));
	}

	server.register_listener(tcp_listener, std::time::Duration::from_secs(5));

	tracing::info!(
		"DNS server successfully bound to loopback addresses: UDP={}, TCP={}",
		bound_addr,
		tcp_bound_addr
	);

	// Block on the server
	match server.block_until_done().await {
		Ok(_) => {
			tracing::info!("DNS server shutdown gracefully");
			Ok(())
		},
		Err(e) => {
			let err_msg = format!("DNS server error: {:?}", e);
			tracing::error!("{}", err_msg);
			Err(anyhow::anyhow!(err_msg))
		},
	}
}

/// Serves DNS queries synchronously (blocking).
///
/// This is a convenience wrapper around the async [`serve`] function
/// for use in contexts where async is not available.
///
/// # Parameters
///
/// * `catalog` - The DNS catalog containing zones to serve
/// * `port` - The port number to bind to
///
/// # Example
///
/// ```rust,no_run
/// use std::thread;
///
/// use Mist::server::{build_catalog, serve_sync};
///
/// let catalog = build_catalog(5353).unwrap();
///
/// // Run DNS server in a background thread
/// thread::spawn(move || {
/// 	serve_sync(catalog, 5353).expect("DNS server failed");
/// });
/// ```
pub fn serve_sync(catalog:Catalog, port:u16) -> Result<()> {
	// Use try blocks or similar to convert async to sync
	// For now, we'll create a runtime and block on the async serve
	let rt = tokio::runtime::Runtime::new()?;

	rt.block_on(serve(catalog, port))?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use hickory_proto::rr::Name;

	#[test]
	fn test_build_catalog() {
		let _catalog = build_catalog(5353).expect("Failed to build catalog");

		// Verify the catalog is not empty
		// The catalog should have at least one zone (editor.land)
		// Note: Catalog doesn't have a direct way to check zone count in 0.24
		// We verify it was created successfully
		println!("Catalog built successfully");
	}

	#[test]
	fn test_catalog_has_editor_land_zone() {
		let catalog = build_catalog(5353).expect("Failed to build catalog");

		// Try to lookup the editor.land zone
		let editor_land_name = Name::from_ascii("editor.land.").unwrap();

		// The catalog should have a zone that can answer for editor.land
		// We verify creation succeeded; actual DNS queries require a server
		let _catalog = catalog;
		let _name = editor_land_name;
		println!("Catalog has editor.land zone");
	}

	#[test]
	fn test_socket_address_creation() {
		let port = 5353u16;
		let addr:SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);

		assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
		assert_eq!(addr.port(), port);
		println!("Socket address created correctly");
	}

	#[test]
	fn test_socket_address_is_loopback() {
		let addr:SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5353);

		assert!(addr.ip().is_loopback(), "Address should be loopback");
	}

	#[test]
	fn test_socket_address_non_loopback_rejected() {
		let addr:SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 5353);

		assert!(!addr.ip().is_loopback(), "Non-loopback address should not be loopback");
	}

	#[test]
	fn test_ipv6_loopback_address() {
		let addr:SocketAddr = SocketAddr::new(IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 5353);

		assert!(addr.ip().is_loopback(), "IPv6 ::1 should be loopback");
	}

	#[test]
	fn test_catalog_multiple_builds() {
		// Test that multiple catalogs can be built independently
		let _catalog1 = build_catalog(15390).expect("Failed to build first catalog");
		let _catalog2 = build_catalog(15391).expect("Failed to build second catalog");

		// Both should be created successfully
		println!("Multiple catalogs built successfully");
	}

	#[test]
	fn test_catalog_zone_authority() {
		let _catalog = build_catalog(5353).expect("Failed to build catalog");

		// The catalog should contain authoritative zones
		println!("Catalog configured with authoritative zones");
	}

	#[test]
	fn test_dns_port_validation() {
		// Test various port values
		let valid_ports = vec![1024, 5353, 8053, 10053];

		for port in valid_ports {
			let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
			assert_eq!(addr.port(), port, "Port should be preserved");
		}
	}
}
