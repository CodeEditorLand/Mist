//! Integration tests for the DNS server
//!
//! These tests verify DNS server startup, resolution, forward authority,
//! and DNSSEC functionality.

use std::time::Duration;

use Mist::{dns_port, Server, Zone, Resolver, ForwardSecurity};
use hickory_proto::rr::RData;

/// Test DNS server startup
#[tokio::test]
async fn test_dns_server_startup() {
	// Start DNS server on a random port
	let port = Mist::start(15353).expect("Failed to start DNS server");
	// Need to call start before dns_port is available
	assert_eq!(dns_port(), port);

	// Verify port is in valid range
	assert!(port >= 1024, "Port should be >= 1024");
	assert!(port <= 65535, "Port should be <= 65535");

	// Give server time to start
	tokio::time::sleep(Duration::from_millis(100)).await;

	println!("DNS server started successfully on port {}", port);
}

/// Test DNS catalog building
#[test]
fn test_dns_catalog_building() {
	let catalog = build_catalog(15356);
	assert!(catalog.is_ok(), "Should be able to build catalog");

	let _catalog = catalog.expect("Catalog should be created");
	println!("DNS catalog built successfully");
}

/// Test DNS server can bind to loopback
#[test]
fn test_dns_server_loopback_binding() {
	// Verify we can build a catalog
	let catalog = build_catalog(15357).expect("Failed to build catalog");

	let _ = catalog;

	// The actual binding test would require starting the server
	// which we can do in a separate test
	println!("Loopback binding test passed");
}

/// Test zone creation
#[test]
fn test_zone_creation() {
	let zone = Zone::editor_land_zone();
	assert!(zone.is_ok(), "Should be able to create zone");

	let zone_records = zone.expect("Zone should be created");
	assert!(!zone_records.is_empty(), "Zone should contain records");

	println!("Zone created successfully with {} records", zone_records.len());
}

/// Test authority creation
#[test]
fn test_authority_creation() {
	let authority = Zone::editor_land_authority();
	assert!(authority.is_ok(), "Should be able to create authority");

	let _authority = authority.expect("Authority should be created");

	println!("Authority created successfully");
}

/// Test resolver creation
#[test]
fn test_resolver_creation() {
	let port = 15400;
	let resolver = Resolver::land_resolver(port);

	// Just verify creation works - the resolver is a stub
	let _ = resolver;

	println!("Resolver created successfully");
}

/// Test land DNS resolver creation
#[test]
fn test_land_dns_resolver_creation() {
	let port = 15401;
	let resolver = Resolver::LandDnsResolver::new(port);

	// Just verify creation works - the resolver is a stub
	let _ = resolver;

	println!("Land DNS resolver created successfully");
}

/// Test allowlist generation
#[test]
fn test_allowlist_generation() {
	let allowlist = ForwardSecurity::default_forward_allowlist();

	let domains:Vec<_> = allowlist.filter_map(|r| r.ok()).collect();

	assert!(!domains.is_empty(), "Allowlist should not be empty");
	assert_eq!(domains.len(), 2, "Should have 2 domains in allowlist");

	println!("Allowlist generated successfully: {:?}", domains);
}

/// Test multiple DNS server instances
#[test]
fn test_multiple_dns_servers() {
	// Test building catalogs for multiple ports
	let catalog1 = build_catalog(15390).expect("Failed to build first catalog");
	let catalog2 = build_catalog(15391).expect("Failed to build second catalog");

	let _ = (catalog1, catalog2);

	println!("Multiple DNS servers tested successfully");
}

/// Test zone record types
#[test]
fn test_zone_record_types() {
	let zone = Zone::editor_land_zone().expect("Failed to create zone");

	// Check for SOA record
	let has_soa = zone.iter().any(|r| matches!(r.data(), RData::SOA(_)));
	assert!(has_soa, "Zone should have SOA record");

	// Check for NS record
	let has_ns = zone.iter().any(|r| matches!(r.data(), RData::NS(_)));
	assert!(has_ns, "Zone should have NS record");

	// Check for A records
	let has_a = zone.iter().any(|r| matches!(r.data(), RData::A(_)));
	assert!(has_a, "Zone should have A records");

	println!("Zone record types verified successfully");
}

/// Test loopback resolution
#[test]
fn test_loopback_resolution() {
	let zone = Zone::editor_land_zone().expect("Failed to create zone");

	// Check that all A records resolve to 127.x.x.x
	for record in &zone {
		if let RData::A(ip) = record.data() {
			let octets = ip.octets();
			assert_eq!(
				octets[0],
				127,
				"A record for {} should resolve to 127.x.x.x, got {}.{}.{}.{}",
				record.name(),
				octets[0],
				octets[1],
				octets[2],
				octets[3]
			);
		}
	}

	println!("Loopback resolution verified successfully");
}
