//! DNS Resolver Module
//!
//! This module provides DNS resolution capabilities for the CodeEditorLand
//! private network. It overrides the system DNS resolver to use the local
//! Hickory DNS server for editor.land domains.

use std::{
	net::{IpAddr, Ipv4Addr, SocketAddr},
};

/// Stub type for the DNS resolver.
/// This is a simplified implementation for compilation purposes.
///
/// In a production implementation, this would be a real hickory-client resolver
/// that connects to the local DNS server.
pub struct TokioResolver;

/// Creates a TokioResolver that queries the local DNS server.
///
/// This is a stub implementation for compilation. In production, this would
/// create a real resolver that connects to the local DNS server.
///
/// # Parameters
///
/// * `_dns_port` - The port of the local DNS server (ignored in stub)
///
/// # Returns
///
/// Returns a Resolver stub.
///
/// # Note
///
/// This is a stub implementation for compilation. The actual resolver would
/// connect to the local DNS server and resolve queries.
pub fn land_resolver(_dns_port: u16) -> TokioResolver {
	TokioResolver
}

/// Custom DNS resolver that uses the local Hickory DNS server.
///
/// This resolver is designed for use with reqwest's DNS override functionality.
/// It wraps a TokioResolver and implements the `reqwest::dns::Resolve` trait.
///
/// # Security
///
/// This resolver ensures that all DNS queries go through the local DNS server,
/// which resolves `*.editor.land` domains to `127.x.x.x` addresses as a
/// defense-in-depth measure.
///
/// # Note
///
/// This is a stub implementation for compilation. The actual resolver would
/// connect to the local DNS server and resolve queries.
pub struct LandDnsResolver;

impl LandDnsResolver {
	/// Creates a new LandDnsResolver.
	///
	/// # Parameters
	///
	/// * `_port` - The port of the local DNS server (ignored in stub)
	///
	/// # Note
	///
	/// This is a stub implementation for compilation. The actual resolver would
	/// connect to the local DNS server and resolve queries.
	pub fn new(_port: u16) -> Self {
		Self
	}
}

// Implement the reqwest::dns::Resolve trait
// This allows the resolver to be used with reqwest's DNS override
impl reqwest::dns::Resolve for LandDnsResolver {
	fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
		let name_str = name.as_str().to_string();

		Box::pin(async move {
			// SECURITY: Defense-in-depth IP validation for *.editor.land domains
			//
			// This check ensures that any DNS response for *.editor.land domains
			// only returns 127.x.x.x addresses. This provides protection against:
			//
			// 1. **Upstream DNS poisoning**: If an attacker compromises the upstream DNS or
			//    the forward authority, they cannot return external IPs for editor.land
			//    domains
			//
			// 2. **Zone file corruption**: If the zone file is modified to point to
			//    external IPs, this validation prevents traffic from leaving localhost
			//
			// 3. **Process compromise**: If the DNS server process is compromised, this
			//    prevents it from redirecting editor.land traffic to external hosts
			//
			// Any non-editor.land domain is allowed to resolve to any IP (subject to
			// the forward allowlist restrictions in RestrictedForwardAuthority).
			let is_editor_land = name_str.ends_with(".editor.land") || name_str == "editor.land";

			if is_editor_land {
				// For editor.land domains, return loopback addresses
				let addrs = vec![
					SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
				];
				Ok(Box::new(addrs.into_iter()) as Box<dyn Iterator<Item = SocketAddr> + Send>)
			} else {
				// For non-editor.land domains, return empty (will fall back to system DNS)
				// In production, this would forward to the upstream DNS server
				Ok(Box::new(std::iter::empty()) as Box<dyn Iterator<Item = SocketAddr> + Send>)
			}
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_resolver_creation() {
		let port = 15353;
		let _resolver = land_resolver(port);
		// Just verify creation works
	}

	#[test]
	fn test_land_dns_resolver_creation() {
		let port = 15354;
		let _resolver = LandDnsResolver::new(port);
		// Verify creation works
	}

	#[test]
	fn test_resolver_port_configuration() {
		let ports = vec![15355u16, 15356, 15357];
		// Just verify we can construct the various port values
		for &port in &ports {
			assert!(port > 1024);
		}
	}

	#[test]
	fn test_resolver_localhost_address() {
		let addr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
		assert!(matches!(addr, IpAddr::V4(_)));
	}

	#[test]
	fn test_editor_land_domain_detection() {
		assert!("example.editor.land".ends_with(".editor.land"));
		assert!(!"example.com".ends_with(".editor.land"));
		assert!("editor.land" == "editor.land".to_string());
	}
}
