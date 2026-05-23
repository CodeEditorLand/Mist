//! # DNS Forward Security
//!
//! Allowlist-based security wrapper for DNS forwarding.
//! Prevents sidecars from reaching arbitrary external hosts via DNS.
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

use anyhow::{Result, anyhow};
use hickory_proto::rr::Name;

/// Returns the default DNS forward allowlist.
///
/// Domains in the allowlist may be forwarded to upstream DNS servers.
/// All other domains receive `REFUSED`.
pub fn DefaultForwardAllowlist() -> impl Iterator<Item = Result<Name>> {
	vec![Name::from_ascii("update.editor.land.")]
		.into_iter()
		.map(|R| R.map_err(|E| anyhow!("Failed to parse domain name: {}", E)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn TestAllowlistGeneration() {
		let Allowlist:Vec<Name> = DefaultForwardAllowlist().filter_map(|R| R.ok()).collect();

		assert!(!Allowlist.is_empty(), "Allowlist should not be empty");
	}
}
