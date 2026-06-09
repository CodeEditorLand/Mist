# Changelog - Mist

Mist is our DNS isolation element - the Hickory-DNS-backed authoritative server
that resolves `*.editor.land` to loopback so the editor's private network
traffic never leaks to an external resolver. This file records what we built in
our voice, version by version. Format adapted from
[Keep a Changelog](https://keepachangelog.com/).

## [v2.1] - Full Workbench Lift (April 2026)

We tightened Mist's API surface and brought it in line with the project's naming
rules during the workbench-lift cycle.

### Added

- **CHANGELOG documenting version history** (`58cc381`, 2026-04-17).
- **README expansion** with comprehensive technical documentation (`86604bf`,
  2026-04-05) and a benefit-focused rewrite pass (`44e1fd8`, `7317fff`,
  2026-04-04). ASCII architecture diagram fixed (`764059f`, 2026-04-06).
- **Crate-level rustdoc** rewritten benefit-first in `Library.rs` (`a98d940`,
  2026-04-04).

### Changed

- **Hickory-server upgraded to 0.26 API** (`160d79e`, 2026-04-22).
- **Library renamed `mist` → `Mist`** (`b681ca8`, 2026-04-18) for PascalCase
  compliance: `Cargo.toml` lib name, the clippy-allow attribute relocated to the
  module doc, and integration tests updated to use direct module paths
  (`Server`, `Zone`, `Resolver`, `ForwardSecurity`) instead of the flattened
  `Mist::` re-exports.
- **Integration-test import paths refactored** end-to-end (`9b1633d`,
  2026-04-18).
- **Non-snake_case identifiers allowed** at module scope (`94e88d1`, 2026-04-27)
  so the PascalCase rename doesn't trip clippy.
- **Consistent Rust formatting** applied to the DNS module (`42f97d1`,
  2026-04-11).

## [v2.0] - Editor Launch (DNS Isolation Introduced, Q1 2026)

The pivotal cycle. Mist was introduced as a brand-new element on **2026-02-27**
to give the fleet a private DNS surface.

### Added

- **DNS isolation element** for the Land private network (`6db973f`,
  2026-02-27). A complete DNS server built on Hickory DNS that creates a secure
  sandbox resolving `*.editor.land` exclusively to 127.0.0.1 - all private
  network communication stays local and cannot leak.
    - **`Server.rs`** - Hickory DNS server, UDP/TCP listeners bound exclusively
      to loopback with comprehensive security validation.
    - **`Zone.rs`** - authoritative zone config for `editor.land` with SOA, NS,
      A records all pointing at loopback.
    - **`Resolver.rs`** - `LandDnsResolver` implementing `reqwest::dns::Resolve`
      for HTTP-client integration with defense-in-depth IP validation.
    - **`ForwardSecurity.rs`** - forward allowlist restricting external DNS
      queries to trusted domains (`update.editor.land`, `cdn.crashlytics.com`).
    - **`Library.rs`** - public API: `Start()` and `DnsPort()`.
- **Integration**: Mountain starts the DNS server during init and exposes the
  port via `DnsPort` managed state; Air uses it for secure HTTP; SideCar spawns
  Node.js sidecars with DNS override configuration; Cocoon resolves
  `editor.land` for its gRPC link to Mountain through this server.

### Changed

- **TODOs reclassified as DEPENDENCY comments** for Hickory DNS (`50f8db4`,
  2026-03-04). Surviving TODOs now track upstream work, not local stalls.
- **Lint-attribute fixes** applied (`addf979`, 2026-03-04).
- **Redundant `#[must_use]` removed from `Start()`** (`6b5183f`, 2026-03-04).

## [v1.x] - Pre-DNS Scaffold (April 2025 - January 2026)

Mist existed as a placeholder repository through this entire window. Five total
commits, all unlabelled scaffolding pushes (`a351282` 2025-04-16, `24fbde8`
2025-06-02, `de354f6` 2025-06-09 README deletion, `49bd47e` 2025-09-12,
`e9428e0` 2026-01-26). The DNS implementation arrived in v2.0 with the
**2026-02-27** introduction above.

## [v0.0] - Project Inception

Repository created April 2025 as a placeholder for what would eventually become
the DNS isolation element. The architectural slot existed in the fleet diagrams;
the substance landed nine months later when we needed a way to keep
`editor.land` traffic on loopback.
