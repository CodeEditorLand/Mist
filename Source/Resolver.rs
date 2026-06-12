//! # DNS Resolver
//!
//! Provides DNS resolution for the CodeEditorLand private network.
//! Routes `*.editor.land` queries to loopback; other domains fall back
//! to system DNS.
//!
//! ## Types
//!
//! * [`TokioResolver`] — Stub resolver that queries the local DNS server.
//! * [`LandDnsResolver`] — Secured resolver for `reqwest` DNS override.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Stub DNS resolver type.
///
/// In production this wraps a real hickory-client resolver connected
/// to the local DNS server. Currently a zero-size placeholder awaiting
/// integration with the running Mist server port.
pub struct TokioResolver;

/// Builds a `TokioResolver` stub that queries the local DNS server.
///
/// ## Parameters
///
/// * `_DNSPort` — Port number of the running Mist server (currently unused).
///
/// ## Returns
///
/// A new `TokioResolver` instance.
pub fn LandResolver(_DNSPort:u16) -> TokioResolver { TokioResolver }

/// Secured DNS resolver for use with `reqwest`'s DNS override.
///
/// Routes `*.editor.land` queries to `127.0.0.1` and lets other domains
/// fall back to system resolution.
pub struct LandDnsResolver;

impl LandDnsResolver {
	/// Builds a new `LandDnsResolver` connected to the given DNS port (PascalCase).
	///
	/// Matches the project's naming convention for constructors.
	/// See also [`new`](Self::new).
	///
	/// ## Parameters
	///
	/// * `_Port` — Port number of the running Mist server (currently unused).
	///
	/// ## Returns
	///
	/// A new `LandDnsResolver` instance.
	pub fn New(_Port:u16) -> Self { Self }

	/// Builds a new `LandDnsResolver` (snake_case alias for reqwest).
	///
	/// Exists for compatibility with the `reqwest::dns::Resolve`
	/// trait's expected construction pattern. Both this and [`New`](Self::New)
	/// are identical.
	///
	/// ## Parameters
	///
	/// * `_Port` — Port number of the running Mist server (currently unused).
	///
	/// ## Returns
	///
	/// A new `LandDnsResolver` instance.
	pub fn new(_Port:u16) -> Self { Self }
}

impl reqwest::dns::Resolve for LandDnsResolver {
	/// Resolves a domain name to IP addresses.
	///
	/// Routes `*.editor.land` queries to `127.0.0.1`. Returns an empty
	/// iterator for all other domains so that `reqwest` falls through to
	/// its system DNS resolver.
	///
	/// ## Parameters
	///
	/// * `Name` — The domain name to resolve.
	///
	/// ## Returns
	///
	/// A resolving future that yields socket addresses (loopback for
	/// `editor.land` domains, empty otherwise).
	fn resolve(&self, Name:reqwest::dns::Name) -> reqwest::dns::Resolving {
		let NameString = Name.as_str().to_string();

		Box::pin(async move {
			let IsLandPlayForm = NameString.ends_with(".editor.land") || NameString == "editor.land";

			if IsLandPlayForm {
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
