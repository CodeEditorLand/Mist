# Mist: DNS Isolation Server 🌫️

This document describes `Mist`, a local DNS server that provides network
isolation for `Land`'s sidecar processes:

- Runs an authoritative DNS server for the `land.playform.cloud` zone
- Resolves all subdomains to `127.0.0.1`
- Implements forward allowlisting for controlled external domain access

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [DNS Zone Configuration](#dns-zone-configuration)
4. [Forward Allowlisting](#forward-allowlisting)
5. [DNSSEC](#dnssec)
6. [WebSocket Transport](#websocket-transport)
7. [Startup Sequence](#startup-sequence)
8. [Related Documentation](#related-documentation)

---

```mermaid
graph TB
    subgraph Mist["Mist DNS Isolation Server"]
        SRV["Server.rs<br/>UDP + TCP<br/>port 5380"]
        ZONE["Zone.rs<br/>land.playform.cloud zone<br/>*.land.playform.cloud -> 127.0.0.1"]
        RES["Resolver.rs<br/>external DNS<br/>forwarding"]
        FSEC["ForwardSecurity.rs<br/>DNSSEC signing<br/>ECDSA P-256"]
        WS["WebSocket.rs<br/>Sky<->Cocoon<br/>transport"]

        SRV --> ZONE
        SRV --> RES
        SRV --> FSEC
        SRV --> WS
        ZONE -->|"authoritative"| FSEC
        RES -->|"allowlisted"| UPSTREAM["Upstream DNS"]
    end

    SIDECARS["Sidecars<br/>Cocoon / Air / Grove"] -->|"DNS queries"| SRV
    ZONE -->|"NXDOMAIN"| BLOCKED["Blocked domains"]
```

## Overview 📋

`Mist` runs a local `Hickory DNS` server authoritative for the
`land.playform.cloud` zone on loopback (port 5380):

- Provides network isolation for sidecar processes (`Cocoon`, `Air`, `Grove`)
- Prevents them from resolving arbitrary external hosts without explicit
  allowlisting

| Attribute    | Value                                               |
| ------------ | --------------------------------------------------- |
| Language     | `Rust` (edition 2024)                               |
| Crate type   | Library + Binary                                    |
| DNS library  | `hickory-server`, `hickory-proto`, `hickory-client` |
| Port         | 5380 (UDP + TCP)                                    |
| Dependencies | `Common`, `ring`, `tokio`, `reqwest`                |
| Consumed by  | `Air`, `Mountain`, `SideCar`                        |

---

## Architecture 🏗️

```
+----------------------------------------------------------+
|                        Mist                               |
|                                                           |
|  +------------------+  +------------------+               |
|  | Server.rs        |  | Zone.rs          |               |
|  | UDP + TCP DNS    |  | land.playform.cloud zone |               |
|  | listener         |  | resolution logic |               |
|  +------------------+  +------------------+               |
|                                                           |
|  +------------------+  +------------------+               |
|  | Resolver.rs      |  | ForwardSecurity  |               |
|  | External DNS     |  | .rs              |               |
|  | forwarding       |  | DNSSEC signing   |               |
|  +------------------+  +------------------+               |
|                                                           |
|  +------------------+                                     |
|  | WebSocket.rs     |                                     |
|  | WebSocket        |                                     |
|  | transport for    |                                     |
|  | Sky<->Cocoon     |                                     |
|  +------------------+                                     |
+----------------------------------------------------------+
```

### Module Map 🗺️

| Path                        | Purpose                                                        |
| --------------------------- | -------------------------------------------------------------- |
| `Source/Server.rs`          | UDP and TCP DNS listener, query dispatch                       |
| `Source/Zone.rs`            | `land.playform.cloud` zone configuration and record generation |
| `Source/Resolver.rs`        | External DNS forwarding for allowlisted domains                |
| `Source/ForwardSecurity.rs` | DNSSEC signing with ECDSA P-256                                |
| `Source/WebSocket.rs`       | WebSocket transport for `Sky`<->`Cocoon` communication         |
| `Source/lib.rs`             | Library root                                                   |

---

## DNS Zone Configuration 🌐

`Mist` serves the `land.playform.cloud` zone with the following configuration:

```
land.playform.cloud.  IN SOA  localhost. root.land.playform.cloud. (
    2026010100 ; serial
    3600       ; refresh
    900        ; retry
    86400      ; expire
    60         ; minimum TTL
)

*.land.playform.cloud.  IN A  127.0.0.1
```

All `*.land.playform.cloud` subdomains resolve to `127.0.0.1`:

- Ensures sidecar processes communicate only over localhost
- Prevents any sidecar process from exfiltrating data through DNS
- Provides a first line of defense against compromised extension code

### Resolution Rules 📋

| Query Pattern           | Response                | Behavior                        |
| ----------------------- | ----------------------- | ------------------------------- |
| `*.land.playform.cloud` | `A 127.0.0.1`           | Authoritative answer from zone  |
| Allowlisted domain      | Forward to upstream DNS | Pass-through to system resolver |
| All other domains       | `NXDOMAIN`              | Refused                         |

---

## Forward Allowlisting 📝

`Mist` maintains a configurable allowlist of trusted external domains that
sidecar processes may resolve:

| Domain                         | Purpose                        | Status              |
| ------------------------------ | ------------------------------ | ------------------- |
| `marketplace.visualstudio.com` | Extension downloads            | Allowlisted         |
| `update.land.playform.cloud`   | Application update server      | Allowlisted         |
| `api.posthog.com`              | Telemetry (when enabled)       | Allowlisted         |
| `www.google-analytics.com`     | Usage analytics (when enabled) | Allowlisted         |
| All unlisted domains           | Blocked                        | `NXDOMAIN` response |

The allowlist is loaded from a configuration file at startup and can be updated
at runtime.

---

## DNSSEC 🔐

`Mist` supports DNSSEC with ECDSA P-256 signing for the `land.playform.cloud`
zone:

| Aspect         | Detail                               |
| -------------- | ------------------------------------ |
| Algorithm      | ECDSA P-256 (algorithm 13)           |
| Key generation | First-run automatic generation       |
| Key storage    | Application data directory           |
| RRSIG records  | Automatically generated on zone load |
| Validation     | N/A (authoritative server)           |

Signing keys are generated on first run and cached for subsequent starts. Zone
data is signed with RRSIG records before responding to queries that request
DNSSEC.

---

## WebSocket Transport 🔌

In addition to DNS serving, `Mist` provides a WebSocket transport layer for
`Sky`<->`Cocoon` communication:

- Used for real-time data streaming between the UI layer and the extension host
- Used when `gRPC` is unavailable or inappropriate for the communication pattern

```
Sky UI (WebView)
    |
    | WebSocket
    v
Mist WebSocket.rs
    |
    | Message routing
    v
Cocoon (Node.js extension host)
```

---

## Startup Sequence 🚀

```
1. Mountain or Air spawns Mist binary
   - Port 5380 is bound (UDP + TCP)
   - System DNS configuration may be updated to point to loopback

2. Mist opens UDP and TCP listeners on port 5380
   - Hickory DNS server initializes
   - land.playform.cloud zone is loaded from configuration
   - DNSSEC signing keys are loaded or generated

3. DNS resolution begins
   - *.land.playform.cloud queries answered authoritatively
   - Allowlisted domains forwarded to system resolver
   - All other domains return NXDOMAIN

4. WebSocket server starts (optional)
   - Bound to configured WebSocket port
   - Accepts connections from Sky and Cocoon
```

---

## Related Documentation 📚

- [Air](https://github.com/CodeEditorLand/Air/tree/Current/Documentation/GitHub/Architecture.md) -
  Background daemon (DNS consumer)
- [Mountain](https://github.com/CodeEditorLand/Mountain/tree/Current/Documentation/GitHub/Architecture.md) -
  Main backend (DNS consumer)
- [SideCar](https://github.com/CodeEditorLand/SideCar/tree/Current/Documentation/GitHub/Architecture.md) -
  Vendored runtimes (DNS consumer)
- [RustInfrastructure](https://github.com/CodeEditorLand/Land/tree/Current/Documentation/GitHub/RustInfrastructure.md) -
  `Rust` backend components

---

**Project Maintainers:** Source Open
([Source/Open@Land.PlayForm.Cloud](mailto:Source/Open@Land.PlayForm.Cloud)) |
[GitHub Repository](https://github.com/CodeEditorLand/Mist) |
[Report an Issue](https://github.com/CodeEditorLand/Mist/issues)
