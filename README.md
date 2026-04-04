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

# **Mist**&#x2001;🌫️

> **Development environments that communicate over the public internet expose services to unnecessary risk. DNS resolution for local services goes through external resolvers, leaking information about the development setup.**

_"Nothing leaks to the public internet. A clean network boundary between the editor and the outside world."_

[![License: CC0-1.0](https://img.shields.io/badge/License-CC0_1.0-lightgrey.svg)](../../LICENSE)
[![Rust Version](https://img.shields.io/badge/Rust-1.95+-blue.svg)](https://www.rust-lang.org/)
[![Hickory DNS Version](https://img.shields.io/badge/Hickory_v0.24-blue.svg)](https://github.com/hickory-dns/hickory-dns)

Mist creates a fully sandboxed DNS zone that resolves every `*.editor.land` domain to `127.0.0.1`. All Land services communicate through this local layer. Nothing leaks to the public internet.

---

## What It Does&#x2001;🔐

- **Local DNS sandbox.** Every `*.editor.land` domain resolves to `127.0.0.1`.
- **Network isolation.** All Land services communicate locally, never through external DNS.
- **Zero public exposure.** Development services are invisible to the public internet.

---

## Development&#x2001;🛠️

Mist is a component of the Land workspace. Follow the
[Land Repository](https://github.com/CodeEditorLand/Land) instructions to
build and run.

---

## License&#x2001;⚖️

CC0 1.0 Universal. Public domain. No restrictions.
[LICENSE](https://github.com/CodeEditorLand/Mist/tree/Current/LICENSE)

---

## See Also

- [Mist Documentation](https://editor.land/Doc/mist)
- [Architecture Overview](https://editor.land/Doc/architecture)
- [Why Rust](https://editor.land/Doc/why-rust)
- [Mountain](https://github.com/CodeEditorLand/Mountain)


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
