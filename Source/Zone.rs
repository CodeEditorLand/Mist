//! # DNS Zone
//!
//! Provides DNS zone configuration for the CodeEditorLand private network.
//! Creates an authoritative zone for `land.playform.cloud` that resolves to
//! loopback addresses.

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

/// Creates the `land.playform.cloud` authoritative zone records.
///
/// All `*.land.playform.cloud` domains resolve to `127.x.x.x` (loopback).
pub fn EditorLandZone() -> Result<Vec<Record>> {
	let mut Records = Vec::new();

	let Origin = Name::from_ascii("land.playform.cloud.").unwrap();

	let TTL = 300u32;

	let Serial = 2025010100u32;

	let Refresh:i32 = 86400;

	let Retry:i32 = 7200;

	let Expire:i32 = 604800;

	let Minimum:u32 = 3600;

	let SOARecord = SOA::new(
		Name::from_ascii("ns1.land.playform.cloud.").unwrap(),
		Name::from_ascii("hostmaster.land.playform.cloud.").unwrap(),
		Serial,
		Refresh,
		Retry,
		Expire,
		Minimum,
	);

	Records.push(Record::from_rdata(Origin.clone(), TTL, RData::SOA(SOARecord)));

	let NSName = Name::from_ascii("ns1.land.playform.cloud.").unwrap();

	Records.push(Record::from_rdata(Origin.clone(), TTL, RData::NS(NS(NSName))));

	Records.push(Record::from_rdata(
		Name::from_ascii("ns1.land.playform.cloud.").unwrap(),
		TTL,
		RData::A(A::new(127, 0, 0, 1)),
	));

	let Subdomains = vec!["editor", "www", "localhost", "sidecar", "cocoon"];

	for (Index, Subdomain) in Subdomains.iter().enumerate() {
		let SubdomainName = Name::from_ascii(format!("{}.land.playform.cloud.", Subdomain)).unwrap();

		let IP = A::new(127, 0, (Index / 255) as u8, (Index % 255) as u8);

		Records.push(Record::from_rdata(SubdomainName, TTL, RData::A(IP)));
	}

	Records.push(Record::from_rdata(Origin, TTL, RData::A(A::new(127, 0, 0, 1))));

	Ok(Records)
}

/// Creates an `InMemoryZoneHandler` for the `land.playform.cloud` zone.
pub fn EditorLandAuthority() -> Result<InMemoryZoneHandler<TokioRuntimeProvider>> {
	let Origin = Name::from_ascii("land.playform.cloud.").unwrap();

	let Authority =
		InMemoryZoneHandler::<TokioRuntimeProvider>::empty(Origin, ZoneType::Primary, AxfrPolicy::Deny, None);

	let _Records = EditorLandZone()?;

	Ok(Authority)
}

/// Creates an `InMemoryZoneHandler` for a custom origin with specified records.
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
