//! # DNS Zone
//!
//! Provides DNS zone configuration for the CodeEditorLand private network.
//! Creates an authoritative zone for `editor.land` that resolves to
//! loopback addresses.
//!
//! ## Functions
//!
//! * [`EditorLandZone`] — Generates the zone records (SOA, NS, A).
//! * [`EditorLandAuthority`] — Builds an `InMemoryZoneHandler` for `editor.land`.
//! * [`CustomAuthority`] — Builds a handler for an arbitrary DNS origin.

use anyhow::Result;
use hickory_proto::rr::{
	Name,
	RData,
	Record,
	rdata::{A, NS, SOA},
};
// hickory-server 0.26: see the block comment in `Server.rs` for the full
// rename table (`authority::*` → `zone_handler::*`, `InMemoryAuthority` →
// `InMemoryZoneHandler`, AXFR bool → `AxfrPolicy`). The handler became
// generic over `P: RuntimeProvider`; we pin it to `TokioRuntimeProvider` so
// return types are concrete and callers don't have to thread the parameter.
use hickory_server::{
	net::runtime::TokioRuntimeProvider,
	store::in_memory::InMemoryZoneHandler,
	zone_handler::{AxfrPolicy, ZoneType},
};

/// Builds the `editor.land` authoritative zone records.
///
/// All `*.editor.land` domains resolve to `127.x.x.x` (loopback).
///
/// ## Returns
///
/// A vector of DNS records (SOA, NS, A) for the `editor.land` zone.
pub fn EditorLandZone() -> Result<Vec<Record>> {
	let mut Records = Vec::new();

	let Origin = Name::from_ascii("editor.land.").unwrap();

	let TTL = 300u32;

	let Serial = 2025010100u32;

	let Refresh:i32 = 86400;

	let Retry:i32 = 7200;

	let Expire:i32 = 604800;

	let Minimum:u32 = 3600;

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

	Records.push(Record::from_rdata(Origin.clone(), TTL, RData::NS(NS(NSName))));

	Records.push(Record::from_rdata(
		Name::from_ascii("ns1.editor.land.").unwrap(),
		TTL,
		RData::A(A::new(127, 0, 0, 1)),
	));

	let Subdomains = vec!["editor", "www", "localhost", "sidecar", "cocoon"];

	for (Index, Subdomain) in Subdomains.iter().enumerate() {
		let SubdomainName = Name::from_ascii(format!("{}.editor.land.", Subdomain)).unwrap();

		let IP = A::new(127, 0, (Index / 255) as u8, (Index % 255) as u8);

		Records.push(Record::from_rdata(SubdomainName, TTL, RData::A(IP)));
	}

	Records.push(Record::from_rdata(Origin, TTL, RData::A(A::new(127, 0, 0, 1))));

	Ok(Records)
}

/// Builds an `InMemoryZoneHandler` for the `editor.land` zone.
///
/// Creates a primary zone authority that serves the `editor.land` domain
/// with AXFR transfers disabled.
///
/// ## Returns
///
/// An `InMemoryZoneHandler` configured as primary for `editor.land`.
pub fn EditorLandAuthority() -> Result<InMemoryZoneHandler<TokioRuntimeProvider>> {
	let Origin = Name::from_ascii("editor.land.").unwrap();

	let Authority =
		InMemoryZoneHandler::<TokioRuntimeProvider>::empty(Origin, ZoneType::Primary, AxfrPolicy::Deny, None);

	let _Records = EditorLandZone()?;

	Ok(Authority)
}

/// Builds an `InMemoryZoneHandler` for a custom origin with specified records.
///
/// ## Parameters
///
/// * `Origin` — The DNS origin name (e.g. `example.com.`).
/// * `_Records` — DNS records for the zone (currently unused; the handler
///   is created empty regardless).
///
/// ## Returns
///
/// An `InMemoryZoneHandler` configured as primary for the given origin.
pub fn CustomAuthority(Origin:&Name, _Records:Vec<Record>) -> Result<InMemoryZoneHandler<TokioRuntimeProvider>> {
	let Authority =
		InMemoryZoneHandler::<TokioRuntimeProvider>::empty(Origin.clone(), ZoneType::Primary, AxfrPolicy::Deny, None);

	Ok(Authority)
}

#[cfg(test)]
mod tests {

	use hickory_server::zone_handler::ZoneHandler;

	use super::*;

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
