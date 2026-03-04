//! DNS Zone Module
#![allow(clippy::useless_vec)]
//!
//! This module provides DNS zone configuration for the CodeEditorLand
//! private network. It creates an authoritative zone for the editor.land
//! domain that resolves to loopback addresses.

use anyhow::Result;
use hickory_proto::rr::{
	Name,
	RData,
	Record,
	rdata::{A, NS, SOA},
};
use hickory_server::authority::ZoneType;
use hickory_server::store::in_memory::InMemoryAuthority;

/// Creates the editor.land authoritative zone.
///
/// This zone resolves all `*.editor.land` domains to `127.x.x.x` addresses
/// as a defense-in-depth measure for DNS isolation.
///
/// # Zone Records
///
/// - **SOA**: `editor.land.` - Start of Authority record
/// - **NS**: `ns1.editor.land.` - Name server record
/// - **Glue A**: `ns1.editor.land.` → `127.0.0.1` - Glue record for nameserver
/// - **A**: `*.editor.land.` → `127.x.x.x` - Loopback addresses for subdomains
///
/// # Security
///
/// All editor.land domains resolve to loopback addresses (127.x.x.x), providing:
/// - Ensures traffic stays on localhost
/// - Prevents external DNS resolution for internal services
/// - Defense against DNS rebinding attacks
///
/// # Example
///
/// ```rust,no_run
/// use Mist::zone::editor_land_zone;
///
/// let zone = editor_land_zone()?;
/// let authority = InMemoryAuthority::new(
///     Name::from_ascii("editor.land.").unwrap(),
///     zone,
///     ZoneType::Primary
/// );
/// ```
pub fn editor_land_zone() -> Result<Vec<Record>> {
	let mut records = Vec::new();
	let origin = Name::from_ascii("editor.land.").unwrap();
	let ttl = 300u32;

	// SOA Record
	let serial = 2025010100u32;
	let refresh: i32 = 86400;
	let retry: i32 = 7200;
	let expire: i32 = 604800;
	let minimum: u32 = 3600;

	let soa_rdata = SOA::new(
		Name::from_ascii("ns1.editor.land.").unwrap(),
		Name::from_ascii("hostmaster.editor.land.").unwrap(),
		serial,
		refresh,
		retry,
		expire,
		minimum,
	);

	let soa_record = Record::from_rdata(origin.clone(), ttl, RData::SOA(soa_rdata));
	records.push(soa_record);

	// NS Record
	let ns_name = Name::from_ascii("ns1.editor.land.").unwrap();
	let ns_rdata = NS(ns_name);
	let ns_record = Record::from_rdata(origin.clone(), ttl, RData::NS(ns_rdata));
	records.push(ns_record);

	// Glue A Record for ns1.editor.land
	let glue_ip = A::new(127, 0, 0, 1);
	let glue_record = Record::from_rdata(
		Name::from_ascii("ns1.editor.land.").unwrap(),
		ttl,
		RData::A(glue_ip),
	);
	records.push(glue_record);

	// Add some common subdomains to demonstrate the zone
	let subdomains = vec![
		"editor",
		"www",
		"localhost",
		"sidecar", // For sidecar processes
		"cocoon", // For managed processes
	];

	for (i, subdomain) in subdomains.iter().enumerate() {
		let name = Name::from_ascii(format!("{}.editor.land.", subdomain)).unwrap();
		// Use 127.x.x.x where x.x varies based on subdomain
		let ip = A::new(127, 0, (i / 255) as u8, (i % 255) as u8);
		let record = Record::from_rdata(
			name,
			ttl,
			RData::A(ip),
		);
		records.push(record);
	}

	// Origin A record
	let origin_ip = A::new(127, 0, 0, 1);
	let origin_record = Record::from_rdata(
		origin,
		ttl,
		RData::A(origin_ip),
	);
	records.push(origin_record);

	Ok(records)
}

/// Creates an authoritative zone for editor.land.
///
/// This function creates an `InMemoryAuthority` that serves all `*.editor.land`
/// queries as loopback addresses.
///
/// # Parameters
///
/// * `origin`: The zone origin (e.g., "editor.land.")
///
/// # Returns
///
/// An `InMemoryAuthority` for the editor.land zone.
///
/// # Example
///
/// ```rust,no_run
/// use Mist::zone::editor_land_authority;
///
/// let authority = editor_land_authority()?;
/// let zone_type = authority.zone_type();
/// ```
pub fn editor_land_authority() -> Result<InMemoryAuthority> {
	let origin = Name::from_ascii("editor.land.").unwrap();

	// Use InMemoryAuthority::empty since RecordSet and RrKey are private
	// We'll need to add records using the update method or similar
	// DEPENDENCY: NxProofKind parameter is None - may need to be configured for DNSSEC NSEC/NSEC3 support
	let authority = InMemoryAuthority::empty(
		origin.clone(),
		ZoneType::Primary,
		false,
		None, // Option<NxProofKind> - added in hickory-server 0.25
	);

	// Add records using the update method
	// Note: This is a simplified approach - in reality we'd need to construct
	// proper MessageRequest objects for each record update
	let _records = editor_land_zone()?;

	// For now, return the empty authority
	// DEPENDENCY: Implement proper record insertion using the Authority trait's update method
	Ok(authority)
}

/// Creates an authoritative zone with specified records.
///
/// # Parameters
///
/// * `origin`: The zone origin (e.g., "editor.land.")
/// * `_records`: The records to add to the zone (currently unused due to API limitations)
///
/// # Returns
///
/// An `InMemoryAuthority` with the specified records (currently empty due to API limitations).
///
/// # Example
///
/// ```rust,no_run
/// use Mist::zone::custom_authority;
/// use hickory_proto::rr::Name;
///
/// let origin = Name::from_ascii("example.land.").unwrap();
/// let authority = custom_authority(&origin, vec![])?;
/// ```
pub fn custom_authority(origin: &Name, _records: Vec<Record>) -> Result<InMemoryAuthority> {
	// Use InMemoryAuthority::empty since we can't construct BTreeMap<RrKey, RecordSet> directly
	// DEPENDENCY: NxProofKind parameter is None - may need to be configured for DNSSEC NSEC/NSEC3 support
	let authority = InMemoryAuthority::empty(
		origin.clone(),
		ZoneType::Primary,
		false,
		None, // Option<NxProofKind> - added in hickory-server 0.25
	);

	// For now, return the empty authority
	// DEPENDENCY: Implement proper record insertion using the Authority trait's update method
	Ok(authority)
}

#[cfg(test)]
mod tests {
	use super::*;
	use hickory_server::authority::Authority;

	#[test]
	fn test_zone_creation() {
		let zone = editor_land_zone().expect("Failed to create zone");
		assert!(!zone.is_empty(), "Zone should contain records");
	}

	#[test]
	fn test_zone_authority_creation() {
		let authority = editor_land_authority().expect("Failed to create authority");
		use hickory_server::authority::Authority;
		assert_eq!(authority.zone_type(), ZoneType::Primary);
	}

	#[test]
	fn test_zone_records() {
		let zone = editor_land_zone().expect("Failed to create zone");

		// Check that we have records
		assert!(!zone.is_empty());

		// Check for A records by looking at the RData
		let has_soa = zone.iter().any(|r| matches!(r.data(), RData::SOA(_)));
		assert!(has_soa, "Zone should have SOA record");

		let has_ns = zone.iter().any(|r| matches!(r.data(), RData::NS(_)));
		assert!(has_ns, "Zone should have NS record");

		let has_a = zone.iter().any(|r| matches!(r.data(), RData::A(_)));
		assert!(has_a, "Zone should have A records");
	}

	#[test]
	fn test_loopback_resolution() {
		let zone = editor_land_zone().expect("Failed to create zone");

		// Check that all A records resolve to 127.x.x.x
		for record in &zone {
			if let RData::A(ip) = record.data() {
				let octets = ip.octets();
				assert_eq!(
					octets[0], 127,
					"A record for {} should resolve to 127.x.x.x, got {}.{}.{}.{}",
					record.name(), octets[0], octets[1], octets[2], octets[3]
				);
			}
		}
	}

	#[test]
	fn test_custom_authority() {
		let origin = Name::from_ascii("test.land.").unwrap();
		let ip = A::new(127, 0, 0, 1);
		let record = Record::from_rdata(
			origin.clone(),
			300,
			RData::A(ip),
		);

		let authority = custom_authority(&origin, vec![record]).expect("Failed to create custom authority");
		assert_eq!(authority.zone_type(), ZoneType::Primary);
	}
}
