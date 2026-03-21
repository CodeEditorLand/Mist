//! DNS Forward Security Module
//!
//! This module provides security wrappers for DNS forwarding, implementing
//! an allowlist-based approach to prevent sidecars from reaching arbitrary
//! external hosts via DNS.
//!
//! ## SECURITY ARCHITECTURE
//!
//! The RestrictedForwardAuthority implements a defense-in-depth measure:
//!
//! ```text
//! Query ──► Is *.editor.land? ──► Authoritative (Local)
//!            │ No
//!            ▼
//!     Is in Allowlist? ──► Forward to Upstream
//!            │ No
//!            ▼
//!        Return REFUSED
//! ```
//!
//! This prevents sidecars from using DNS to reach unapproved external
//! services, limiting the attack surface for compromised processes.
//!
//! # Note
//!
//! This module is currently a simplified stub. The Authority trait implementations
//! have been removed because Hickory DNS 0.25.x has significant API changes that
//! require complex lifetime handling. The main DNS functionality uses
//! `InMemoryAuthority` from hickory-server which implements Authority correctly.

use anyhow::{Result, anyhow};
use hickory_proto::rr::Name;

/// Creates a default forward authority allowlist.
///
/// Returns an iterator of domain names that are allowed to be forwarded.
///
/// # Default Allowlist
///
/// - `update.editor.land.` - For application updates
///
/// # Example
///
/// ```rust
/// use Mist::forward_security::default_forward_allowlist;
///
/// let allowlist:Vec<Name> = default_forward_allowlist().map(|n| n.unwrap()).collect();
/// ```
pub fn default_forward_allowlist() -> impl Iterator<Item = Result<Name>> {
	vec![
		Name::from_ascii("update.editor.land."),
	]
	.into_iter()
	.map(|r| r.map_err(|e| anyhow!("Failed to parse domain name: {}", e)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_allowlist_generation() {
		let allowlist: Vec<Name> = default_forward_allowlist()
			.filter_map(|r| r.ok())
			.collect();
		
		assert!(!allowlist.is_empty(), "Allowlist should not be empty");
		assert_eq!(allowlist.len(), 2, "Should have 2 domains in allowlist");
	}
}
