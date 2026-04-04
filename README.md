<table>
	<tr>
		<td align="left" valign="middle">
			<h3 align="left"> Mist</h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left">
				🌫️
			</h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left"> + </h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left">
				<a href="https://Editor.Land" target="_blank">
					<picture>
						<source media="(prefers-color-scheme: dark)" srcset="https://PlayForm.Cloud/Dark/Image/GitHub/Land.svg">
						<source media="(prefers-color-scheme: light)" srcset="https://PlayForm.Cloud/Image/GitHub/Land.svg">
						<img width="28" alt="Land Logo" src="https://PlayForm.Cloud/Image/GitHub/Land.svg">
					</picture>
				</a>
			</h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left">
				<a href="https://Editor.Land" target="_blank">
					Land
				</a>
			</h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left"> 🏞️</h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left"> + </h3>
		</td>
		<td align="left" valign="middle" width="190">
			<h3 align="left">
				<a href="https://Hickory-DNS.rs" target="_blank">
					<picture>
						<source media="(prefers-color-scheme: dark)" srcset="https://PlayForm.Cloud/Dark/Image/GitHub/Made/HickoryDNS.svg">
						<source media="(prefers-color-scheme: light)" srcset="https://PlayForm.Cloud/Image/GitHub/Made/HickoryDNS.svg">
						<img width="160" alt="Made With Hickory DNS" src="https://PlayForm.Cloud/Image/GitHub/Made/HickoryDNS.svg">
					</picture>
				</a>
			</h3>
		</td>
	</tr>
</table>

---

# **Mist** 🌫️

DNS Isolation for the editor.land Private Network

[![License: CC0-1.0](https://img.shields.io/badge/License-CC0_1.0-lightgrey.svg)](../../LICENSE)
[![Rust Version](https://img.shields.io/badge/Rust-1.95+-blue.svg)](https://www.rust-lang.org/)
[![Hickory DNS Version](https://img.shields.io/badge/Hickory_v0.24-blue.svg)](https://github.com/hickory-dns/hickory-dns)

**Mist** provides DNS isolation and private network resolution for the Land Code
Editor. It creates a secure DNS sandbox that resolves all `*.editor.land`
domains locally to `127.0.0.1`, ensuring all private network communication stays
local.

**What Mist gives you:**

1. **Zero-config private networking.** All `*.editor.land` domains resolve to
   `127.0.0.1`. Components find each other automatically, no `/etc/hosts`
   editing needed.
2. **Extension isolation.** Cocoon's Node.js process uses Mist's DNS. Extensions
   can only resolve domains on the forward allowlist. No phone-home to arbitrary
   hosts.
3. **Signed DNS responses.** ECDSA P-256 DNSSEC on the `editor.land` zone.
   Cryptographic proof that DNS answers are authentic.
4. **Dynamic port allocation.** `portpicker` finds available ports automatically.
   No conflicts, no manual configuration.

---

## Key Features 🌫️

- **Hickory DNS Server:** Built on the high-performance Hickory DNS library
  (formerly Trust-DNS), providing a robust, async DNS server implementation.
- **Authoritative Zone:** Operates as an authoritative DNS server for
  `editor.land`, resolving all subdomains (`*.editor.land`) to `127.0.0.1` for
  secure local communication.
- **Forward Security:** Implements a strict allowlist for external DNS queries,
  preventing sidecars from reaching unauthorized external hosts by default.
- **DNSSEC Support:** Signs the authoritative zone with ECDSA P-256 keys,
  providing cryptographic integrity and authenticity for DNS responses.
- **Dynamic Port Selection:** Automatically selects an available port if the
  preferred port (5380) is unavailable, ensuring robust startup behavior.
- **Async Runtime:** Built on Tokio for efficient, non-blocking DNS query
  handling.
- **Cross-Platform:** Works on macOS, Linux, and Windows with consistent
  behavior.

---

## Architecture 🏗️

**Mist** follows a layered architecture.

DNS queries from applications flow through the catalog, which handles zone
lookups and forward allowlist enforcement.

```
┌─────────────────────────────────────────────────────────────────┐
│                     Applications (Wind, Cocoon)                 │
│                        (DNS Queries)                            │
└────────────────────────────────────┬────────────────────────────┘
                                     │
                                     ▼
┌────────────────────────────────────────────────────────────────┐
│                     Mist DNS Server (127.0.0.1:PORT)           │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                   DNS Catalog                            │  │
│  │  ┌────────────────────┐  ┌──────────────────────┐        │  │
│  │  │ Editor.land Zone   │  │ Forward Allowlist    │        │  │
│  │  │ (Authoritative)    │  │ (Restricted Access)  │        │  │
│  │  │ *.editor.land →    │  │ update.editor.land   │        │  │
│  │  │ 127.0.0.1          │  │                      │        │  │
│  │  └────────────────────┘  └──────────────────────┘        │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                │
│  Hickory DNS Server Core (UDP + TCP)                           │
│  - Request parsing and response construction                   │
│  - Zone lookup and record matching                             │
│  - DNSSEC signature verification                               │
└────────────────────────────────────────────────────────────────┘
```

### Components

- **`lib.rs`**: Main library entry point, exports public API and manages the DNS
  server state.
- **`server.rs`**: DNS server implementation using Hickory, handles UDP/TCP
  listeners and catalog management.
- **`zone.rs`**: DNS zone configuration for `editor.land`, including record
  definitions and authority creation.
- **`resolver.rs`**: DNS resolver for use by other components, provides
  interface to the local DNS server.
- **`forward_security.rs`**: Forward allowlist management, restricts which
  external domains can be resolved.
- **`tests/integration.rs`**: Comprehensive integration tests for DNS server
  functionality.

---

## Usage 🔧

### Starting the DNS Server

Start the DNS server on a specific port (or 0 for auto-selection):

```rust
use Mist::start;

// Start on preferred port 5380
let port = Mist::start(5380)?;

// Or let the system select an available port
let port = Mist::start(0)?;

println!("DNS server running on 127.0.0.1:{}", port);
```

### Getting the DNS Server Port

Retrieve the current DNS server port:

```rust
use Mist::dns_port;

let port = dns_port();
println!("DNS server is on port: {}", port);
```

### Creating a DNS Resolver

Create a resolver that uses the local DNS server:

```rust
use Mist::resolver::{land_resolver, LandDnsResolver};

// Simple resolver
let port = Mist::dns_port();
let resolver = land_resolver(port);

// Or with explicit interface
let resolver = LandDnsResolver::new(port);
```

### Building a DNS Catalog

Build a DNS catalog with authoritative zones:

```rust
use Mist::server::build_catalog;

let catalog = build_catalog(5380)?;
```

---

## DNS Zone Configuration 📋

### Authoritative Zone: `editor.land`

All subdomains of `editor.land` resolve to `127.0.0.1`:

- `code.editor.land` → `127.0.0.1`
- `api.editor.land` → `127.0.0.1`
- `*.editor.land` → `127.0.0.1`

### Forward Allowlist

Only allowlisted external domains can be resolved:

- `update.editor.land` - For application updates

All other external queries are refused by default.

### DNSSEC

The `editor.land` zone is signed with ECDSA P-256 keys:

- DNSKEY records provide the public signing key
- RRSIG records provide cryptographic signatures
- Clients can verify the authenticity of DNS responses

---

## Dependencies 📦

**Mist** depends on the following crates:

- **`hickory-server`** (`0.24`): DNS server implementation
- **`hickory-proto`** (`0.24`): DNS protocol implementation
- **`hickory-client`** (`0.24`): DNS client for resolvers
- **`ring`** (`0.17`): Cryptographic signing for DNSSEC
- **`tokio`** (`1.49`): Async runtime
- **`anyhow`** (`1.0`): Error handling
- **`tracing`** (`0.1`): Logging and instrumentation
- **`once_cell`** (`1.21`): Thread-safe lazy initialization
- **`portpicker`** (`0.1.1`): Random port selection
- **`async-trait`** (`0.1`): Async trait support
- **`reqwest`** (`0.13`): HTTP client with DNS integration

---

## Building & Testing 🔨

### Building

Build the library:

```bash
cargo build --release
```

### Running Tests

Run all tests:

```bash
cargo test
```

Run integration tests:

```bash
cargo test --test integration
```

Run with logging:

```bash
RUST_LOG=debug cargo test
```

---

## Security Considerations 🔒

**Mist** implements several security features:

1. **Private Network Isolation:** All `editor.land` domains resolve to
   localhost, preventing any external network access for private services.
2. **Forward Allowlist:** External DNS queries are restricted to a trusted
   allowlist, preventing sidecars from accessing arbitrary external hosts.
3. **DNSSEC:** Zone signing provides cryptographic assurance of DNS responses,
   preventing DNS spoofing attacks.
4. **Loopback Binding:** The DNS server only binds to `127.0.0.1`, preventing
   external access to the private DNS server.

---

## Integration with Land 🔗

**Mist** is integrated into the Land ecosystem:

- **Mountain:** Starts the DNS server during initialization and exposes the port
  to other components via the `DnsPort` managed state.
- **Air:** Uses the DNS server for secure HTTP requests, configuring HTTP
  clients to use the local DNS resolver.
- **SideCar:** Spawns Node.js sidecars with DNS override configuration, ensuring
  all queries go through the local server.
- **Cocoon:** The Node.js extension host resolves `editor.land` domains via the
  local DNS server for gRPC communication with Mountain.

---

## License ⚖️

This project is licensed under Creative Commons CC0.

See the LICENSE file for details.

---

## Changelog 📜

Stay updated with our progress! See
[`CHANGELOG.md`](https://github.com/CodeEditorLand/Mist/tree/Current/) for a
history of changes specific to **Mist**.

---


## See Also

- [Architecture Overview](https://editor.land/Doc/architecture)
- [Mountain](https://github.com/CodeEditorLand/Mountain)
- [Vine](https://github.com/CodeEditorLand/Vine)
- [Air](https://github.com/CodeEditorLand/Air)

## Funding & Acknowledgements 🙏🏻

Code Editor Land is funded through the NGI0 Commons Fund, established by NLnet
with financial support from the European Commission's Next Generation Internet
programme, under grant agreement No. 101135429.

The project is operated by PlayForm, based in Sofia, Bulgaria.

PlayForm acts as the open-source steward for Code Editor Land under the NGI0
Commons Fund grant.

<table>
	<thead>
		<tr>
			<th align="left"><strong>Land</strong></th>
			<th align="left"><strong>PlayForm</strong></th>
			<th align="left"><strong>NLnet</strong></th>
			<th align="left"><strong>NGI0 Commons Fund</strong></th>
		</tr>
	</thead>
	<tbody>
		<tr>
			<td align="left" valign="middle">
				<a href="https://Editor.Land">
					<img width="60" src="https://raw.githubusercontent.com/CodeEditorLand/Asset/refs/heads/Current/Logo/Land.svg" alt="Land">
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://PlayForm.Cloud">
					<img width="76" src="https://raw.githubusercontent.com/PlayForm/Asset/refs/heads/Current/Logo/PlayForm.svg" alt="PlayForm">
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://NLnet.NL">
					<img width="240" src="https://NLnet.NL/logo/banner.svg" alt="NLnet">
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://NLnet.NL/commonsfund">
					<img width="240" src="https://NLnet.NL/image/logos/NGI0CommonsFund_tag_black_mono.svg" alt="NGI0 Commons Fund">
				</a>
			</td>
		</tr>
	</tbody>
</table>

---

**Project Maintainers**: Source Open
([Source/Open@Editor.Land](mailto:Source/Open@Editor.Land)) |
[GitHub Repository](https://github.com/CodeEditorLand/Mist) |
[Report an Issue](https://github.com/CodeEditorLand/Mist/issues) |
[Security Policy](https://github.com/CodeEditorLand/Mist/security/policy)
