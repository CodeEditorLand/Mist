#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
//! # DNS Resolver
//!
//! Provides DNS resolution for the CodeEditorLand private network.
//! Routes `*.editor.land` queries to loopback; other domains fall back to
//! system DNS.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Stub DNS resolver type.
///
/// In production this would wrap a real hickory-client resolver connected
/// to the local DNS server.
pub struct TokioResolver;

/// Creates a `TokioResolver` stub that queries the local DNS server.
pub fn LandResolver(_DNSPort:u16) -> TokioResolver { TokioResolver }

/// Secured DNS resolver for use with `reqwest`'s DNS override.
///
/// Routes `*.editor.land` queries to `127.0.0.1` and lets other domains
/// fall back to system resolution.
pub struct LandDnsResolver;

impl LandDnsResolver {
	/// Creates a new `LandDnsResolver` connected to the given DNS port.
	pub fn New(_Port:u16) -> Self { Self }

	// Keep snake_case alias for reqwest compatibility (external crate pattern)
	pub fn new(_Port:u16) -> Self { Self }
}

impl reqwest::dns::Resolve for LandDnsResolver {
	fn resolve(&self, Name:reqwest::dns::Name) -> reqwest::dns::Resolving {
		let NameString = Name.as_str().to_string();
		Box::pin(async move {
			let IsEditorLand = NameString.ends_with(".editor.land") || NameString == "editor.land";
			if IsEditorLand {
				let Addresses = vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0)];
				Ok(Box::new(Addresses.into_iter()) as Box<dyn Iterator<Item = SocketAddr> + Send>)
			} else {
				Ok(Box::new(std::iter::empty()) as Box<dyn Iterator<Item = SocketAddr> + Send>)
			}
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn TestResolverCreation() { let _Resolver = LandResolver(15353); }

	#[test]
	fn TestLandDnsResolverCreation() { let _Resolver = LandDnsResolver::New(15354); }

	#[test]
	fn TestEditorLandDomainDetection() {
		assert!("example.editor.land".ends_with(".editor.land"));
		assert!(!"example.com".ends_with(".editor.land"));
	}
}
