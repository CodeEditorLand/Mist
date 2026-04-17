# Changelog

All notable changes to the Mist element are documented in this file. Format:
[Keep a Changelog](https://keepachangelog.com/).

Mist is the DNS sandbox — a security-hardened local DNS resolver that provides
isolation for extension network requests, preventing direct host DNS queries and
enabling per-extension domain allowlists.

## [v2.1] — Q2 2026: Documentation + Formatting

### Changed

- Rust formatting standardized across all DNS module files
- `DeepDive.md` table formatting consistency improved
- External links updated in Rust submodule README

## [v2.0] — Q1 2026: DNS Module Maturation

### Added

- 9 Rust source modules implementing DNS resolver core
- Integration with Air daemon's `HTTP/client.rs` as DNS provider
- Comprehensive architecture documentation in `Documentation/`

### Changed

- Cargo workspace integration as `Mist` member of Land root workspace
- PascalCase naming enforced throughout source tree

## [v1.2] — Q3-Q4 2025: Foundation Build

### Added

- Initial Rust scaffolding with Cargo.toml
- Documentation directory with architecture planning
- Security policy (SECURITY.md) and code of conduct
- GitHub Actions CI/CD integration

## [v1.1] — Q2 2025: Project Inception

### Added

- Repository created April 2025 as part of NLnet NGI0 Commons Fund initiative
- Register as CodeEditorLand/Mist GitHub repository
- Initial architecture planning documents
- License (CC0) and code of conduct
