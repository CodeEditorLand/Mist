#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
//! # DNS Server
//!
//! Builds and serves the private DNS catalog for CodeEditorLand.
//! Binds exclusively to loopback (`127.0.0.1`) to prevent LAN exposure.

use std::{
	net::{IpAddr, Ipv4Addr, SocketAddr},
	sync::Arc,
};

use anyhow::Result;
// hickory-server 0.26 reorganisation:
//   authority::*                       → zone_handler::*
//   authority::Authority (trait)       → zone_handler::ZoneHandler
//   authority::Catalog / ZoneType      → zone_handler::Catalog / ZoneType
//   store::in_memory::InMemoryAuthority → store::in_memory::InMemoryZoneHandler
//   server::ServerFuture               → Server (re-exported at crate root)
//   InMemoryAuthority::empty(_,_,bool,_) → InMemoryZoneHandler::empty(_,_,AxfrPolicy,_)
// The behaviour is unchanged; only names moved.
use hickory_server::{
	Server,
	net::runtime::TokioRuntimeProvider,
	store::in_memory::InMemoryZoneHandler,
	zone_handler::{AxfrPolicy, Catalog, ZoneType},
};
use tokio::net::UdpSocket;

/// Buffer capacity for outgoing DNS TCP responses per connection. 65 535 is
/// the upper bound a single DNS message can reach over TCP (the 16-bit
/// length prefix cap from RFC 1035 §4.2.2). Picking the cap avoids any
/// truncation for zone-transfer or large TXT responses while staying well
/// within memory for the dozen-or-so concurrent connections a local
/// `editor.land` catalog ever sees.
const DNS_TCP_RESPONSE_BUFFER_SIZE:usize = 65_535;

/// Builds a DNS catalog for the CodeEditorLand private network.
///
/// Creates a catalog with an authoritative zone for `editor.land` that
/// resolves all queries locally to loopback addresses.
pub fn BuildCatalog(_DNSPort:u16) -> Result<Catalog> {
	let mut Catalog = Catalog::new();

	let EditorLandOrigin = hickory_proto::rr::Name::from_ascii("editor.land.").unwrap();

	// `AxfrPolicy::Deny` replaces the old `false` bool that disabled AXFR.
	// The trailing `None` is `Option<NxProofKind>` and remains dnssec-ring-gated.
	// Turbofish pins the runtime provider so inference has a concrete type
	// (the handler is generic over `P: RuntimeProvider`; there's no
	// inference anchor without either an `.await`-driven callsite or an
	// explicit parameter here).
	let Authority = InMemoryZoneHandler::<TokioRuntimeProvider>::empty(
		EditorLandOrigin.clone(),
		ZoneType::Primary,
		AxfrPolicy::Deny,
		None,
	);

	let EditorLandLower = hickory_proto::rr::LowerName::from(&EditorLandOrigin);
	let AuthorityArc = Arc::new(Authority);
	Catalog.upsert(EditorLandLower, vec![AuthorityArc]);

	Ok(Catalog)
}

/// Serves DNS queries on the specified loopback port (async).
///
/// Binds to `127.0.0.1:{Port}` for both UDP and TCP. Validates that the
/// socket is bound to a loopback address before accepting connections.
pub async fn Serve(Catalog:Catalog, Port:u16) -> Result<()> {
	let Address:SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), Port);

	let BindingIP = Address.ip();
	match BindingIP {
		IpAddr::V4(IP) => {
			if !IP.is_loopback() {
				return Err(anyhow::anyhow!(
					"SECURITY: DNS server attempted to bind to non-loopback address: {}. Only 127.x.x.x addresses are \
					 allowed.",
					IP
				));
			}
		},
		IpAddr::V6(IP) if IP.is_loopback() => {},
		_ => {
			return Err(anyhow::anyhow!(
				"SECURITY: DNS server attempted to bind to invalid address: {}. Only loopback addresses are allowed.",
				BindingIP
			));
		},
	}

	tracing::info!("Binding DNS server to loopback address: {}", Address);

	let UDPSocket = UdpSocket::bind(Address)
		.await
		.map_err(|E| anyhow::anyhow!("SECURITY: Failed to bind DNS server to {}: {}.", Address, E))?;

	let BoundAddress = UDPSocket
		.local_addr()
		.map_err(|E| anyhow::anyhow!("SECURITY: Failed to retrieve bound socket address: {}", E))?;

	if !BoundAddress.ip().is_loopback() {
		return Err(anyhow::anyhow!(
			"SECURITY: UDP socket bound to non-loopback address: {}.",
			BoundAddress.ip()
		));
	}

	// `Server` supersedes `ServerFuture`; constructor + register_* + block_until_done
	// signatures are the same so the rest of this body is unchanged.
	let mut Server = Server::new(Catalog);
	Server.register_socket(UDPSocket);

	let TCPListener = tokio::net::TcpListener::bind(Address)
		.await
		.map_err(|E| anyhow::anyhow!("SECURITY: Failed to bind TCP listener to {}: {}", Address, E))?;

	let TCPBoundAddress = TCPListener
		.local_addr()
		.map_err(|E| anyhow::anyhow!("SECURITY: Failed to retrieve TCP listener bound address: {}", E))?;

	if !TCPBoundAddress.ip().is_loopback() {
		return Err(anyhow::anyhow!(
			"SECURITY: TCP listener bound to non-loopback address: {}.",
			TCPBoundAddress.ip()
		));
	}

	Server.register_listener(
		TCPListener,
		std::time::Duration::from_secs(5),
		DNS_TCP_RESPONSE_BUFFER_SIZE,
	);

	tracing::info!("DNS server bound to loopback: UDP={}, TCP={}", BoundAddress, TCPBoundAddress);

	match Server.block_until_done().await {
		Ok(_) => {
			tracing::info!("DNS server shutdown gracefully");
			Ok(())
		},
		Err(E) => {
			let ErrorMessage = format!("DNS server error: {:?}", E);
			tracing::error!("{}", ErrorMessage);
			Err(anyhow::anyhow!(ErrorMessage))
		},
	}
}

/// Serves DNS queries synchronously (blocking convenience wrapper).
pub fn ServeSync(Catalog:Catalog, Port:u16) -> Result<()> {
	let Runtime = tokio::runtime::Runtime::new()?;
	Runtime.block_on(Serve(Catalog, Port))?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use hickory_proto::rr::Name;

	use super::*;

	#[test]
	fn TestBuildCatalog() { let _Catalog = BuildCatalog(5353).expect("Failed to build catalog"); }

	#[test]
	fn TestSocketAddressIsLoopback() {
		let Address:SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5353);
		assert!(Address.ip().is_loopback());
	}
}
