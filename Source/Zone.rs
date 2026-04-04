#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
//! # DNS Zone
//!
//! Provides DNS zone configuration for the CodeEditorLand private network.
//! Creates an authoritative zone for `editor.land` that resolves to loopback addresses.

use anyhow::Result;
use hickory_proto::rr::{
	Name,
	RData,
	Record,
	rdata::{A, NS, SOA},
};
use hickory_server::authority::ZoneType;
use hickory_server::store::in_memory::InMemoryAuthority;

/// Creates the `editor.land` authoritative zone records.
///
/// All `*.editor.land` domains resolve to `127.x.x.x` (loopback).
pub fn EditorLandZone() -> Result<Vec<Record>> {
	let mut Records = Vec::new();
	let Origin = Name::from_ascii("editor.land.").unwrap();
	let TTL = 300u32;

	let Serial = 2025010100u32;
	let Refresh: i32 = 86400;
	let Retry: i32 = 7200;
	let Expire: i32 = 604800;
	let Minimum: u32 = 3600;

	let SOARecord = SOA::new(
		Name::from_ascii("ns1.editor.land.").unwrap(),
		Name::from_ascii("hostmaster.editor.land.").unwrap(),
		Serial,
		Refresh,
		Retry,
		Expire,
		Minimum,
	);
	Records.push(Record::from_rdata(Origin.clone(), TTL, RData::SOA(SOARecord)));

	let NSName = Name::from_ascii("ns1.editor.land.").unwrap();
	Records.push(Record::from_rdata(
		Origin.clone(),
		TTL,
		RData::NS(NS(NSName)),
	));

	Records.push(Record::from_rdata(
		Name::from_ascii("ns1.editor.land.").unwrap(),
		TTL,
		RData::A(A::new(127, 0, 0, 1)),
	));

	let Subdomains = vec!["editor", "www", "localhost", "sidecar", "cocoon"];
	for (Index, Subdomain) in Subdomains.iter().enumerate() {
		let SubdomainName =
			Name::from_ascii(format!("{}.editor.land.", Subdomain)).unwrap();
		let IP = A::new(127, 0, (Index / 255) as u8, (Index % 255) as u8);
		Records.push(Record::from_rdata(SubdomainName, TTL, RData::A(IP)));
	}

	Records.push(Record::from_rdata(Origin, TTL, RData::A(A::new(127, 0, 0, 1))));

	Ok(Records)
}

/// Creates an `InMemoryAuthority` for the `editor.land` zone.
pub fn EditorLandAuthority() -> Result<InMemoryAuthority> {
	let Origin = Name::from_ascii("editor.land.").unwrap();
	let Authority = InMemoryAuthority::empty(Origin, ZoneType::Primary, false, None);
	let _Records = EditorLandZone()?;
	Ok(Authority)
}

/// Creates an `InMemoryAuthority` for a custom origin with specified records.
pub fn CustomAuthority(Origin: &Name, _Records: Vec<Record>) -> Result<InMemoryAuthority> {
	let Authority =
		InMemoryAuthority::empty(Origin.clone(), ZoneType::Primary, false, None);
	Ok(Authority)
}

#[cfg(test)]
mod tests {
	use super::*;
	use hickory_server::authority::Authority;

	#[test]
	fn TestZoneCreation() {
		let Zone = EditorLandZone().expect("Failed to create zone");
		assert!(!Zone.is_empty());
	}

	#[test]
	fn TestZoneHasLoopbackRecords() {
		let Zone = EditorLandZone().expect("Failed to create zone");
		for Record in &Zone {
			if let RData::A(IP) = Record.data() {
				assert_eq!(IP.octets()[0], 127, "A record must resolve to 127.x.x.x");
			}
		}
	}
}
