# Stage 4 design: plain-HTTP endpoint consistency binding

Historical proposal below is accepted with the reviewer decisions at the end. Implementation and actual tests are recorded separately in [Stage 4 verification](PHASE_7_5_STAGE_4_BINDING_VERIFICATION.md); proposal-era blocked/planned statements are not the current release status.

23 October 2026, Africa/Lagos. PROPOSED; documentation only. This binding is a consistency check, not endpoint authentication. Required UI label: **not cryptographically verified**. No socket, discovery field, authentication or B5 code is implemented by this document.

## Evidence register and Windows inputs

R = confirmed by repository source at accepted base ee3e53d; O = official documentation retrieved 23 October 2026; P = proposed policy; U = unqualified/unknown. No community claim is treated as fact.

| Input | Evidence and proposed collection |
| --- | --- |
| Interface identity | R: `apps/desktop/src-tauri/src/discovery/windows.rs:427` reads AdapterName, `:464`/`:485` LUID; `src/discovery/mod.rs:36` NetworkAdapter exposes id/luid plus indices. O: [IP_ADAPTER_ADDRESSES](https://learn.microsoft.com/en-us/windows/win32/api/iptypes/ns-iptypes-ip_adapter_addresses_lh) documents Luid and adapter metadata; [XP-compatible remarks](https://learn.microsoft.com/en-us/windows/win32/api/iptypes/ns-iptypes-ip_adapter_addresses_xp) warn indices may change. P: use validated interface GUID plus current-boot LUID; index is dispatch metadata, never identity. Across reboot discard session, do not persist LUID as hardware identity. |
| Source address/prefix | R: same collector populates addresses and prefixes; connection/windows.rs uses route/source checks. P: choose the actual route-selected local source, not first list element; encode IP family, binary address, IPv6 scope/interface and prefix length. Unknown/ambiguous source blocks binding. |
| Selected gateway/route | R: NetworkAdapter.default_routes and gateways expose observations, not manufacturer identity. P: bind selected numeric endpoint, gateway and route's interface/source; default gateway may differ from explicitly approved on-link endpoint. Capture both without treating gateway as the endpoint by inference. |
| DHCP-server address | O: [GetAdaptersAddresses](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses) and the [LH structure](https://learn.microsoft.com/en-us/windows/win32/api/iptypes/ns-iptypes-ip_adapter_addresses_lh) document Dhcpv4Server and DHCP flags; members were added in Vista-era structures. P: read only when returned Length includes that field, DHCPv4-enabled flag is set and SOCKET_ADDRESS is valid IPv4 with adequate length. U: whether any particular RNDIS driver exposes a useful address. Static addressing, disabled DHCP, zero/absent/invalid data remain **unknown, not guessed**. Do not substitute DNS or gateway for DHCP server. No DHCP discovery packet is sent. |
| USB correlation | R: discovery/mod.rs:251–340 groups Windows nodes/container ancestry and attached adapters. P: retain exact observed association references plus confidence; no VID/PID/model label alone creates a physical identity. Wi-Fi/Ethernet without a correlated USB device records explicit absence. |
| MAC | P: neighbor values, including reviewer-reported MG11 placeholder `00-CC-CC-CC-CC-CC`, never identify hardware. No new ARP/network probe is authorized. |

The DHCP API fields are available in relevant modern SDK layouts, not a guarantee of a DHCP lease or truthful server identity. Implementation must validate native lengths/pointers and qualify its Windows build/driver combinations; unknown DHCP does not silently become a verified prerequisite. DNS is Windows-reported DNS configuration only, distinct from gateway/DHCP evidence.

## Proposed in-memory digest

P: versioned canonical binary record, fixed field order, integers unsigned big-endian, each byte/string prefixed by u32 length, UTF-8 without locale transformations. Optional values have a one-byte present/unknown discriminator; no ambiguous empty/null equivalence. GUID normalized to 16 bytes; LUID u64; IPs use family + 4/16 bytes, IPv6 scope u32. Prefixes/correlation sets sorted by canonical bytes and deduplicated; no arbitrary Windows enumeration order.

P: SHA-256 over domain `OBTAINER/plain-http-binding/v1` plus device discovery key, boot/session generation, interface GUID/LUID, selected normalized numeric endpoint (scheme/host/port), selected source/prefix, selected route gateway, DHCP status/address and USB correlation evidence IDs/association. Evidence expiry, creation time and session reference accompany the digest outside canonical identity fields. It lives in memory only; no SQLite/settings/identity row, no nonce or credentials. Unknown input is encoded, never filled from another adapter. Digest equality proves only equality of captured observations.

## Rechecks, invalidation and integration

P: read immutable snapshot and actual route immediately before issuing a session approval, before each request bundle and after its single read before publication. Existing scope proofs bind endpoint/interface/revision; add binding generation/digest only through a separately reviewed contract. AuthChallenge observation stays observed; neither digest nor plain-HTTP authenticated fields authorizes write/flash/install. Each authenticated field preserves transport=plain_http, session, scope, collection/expiry and permitted-use references.

P: removal/replug, changed interface GUID/LUID, selected source/prefix/route/gateway, known DHCP value change or known↔unknown transition, USB-correlation change, endpoint substitution, expired evidence, explicit cancel/reset and identity conflict invalidate binding, proofs and session. Generic discovery revisions alone do not: compare canonical relevant facts and invalidate only on material change. R: discovery/service.rs:258–284 retains authoritative revision/age after failed/cancelled scans; repeated identical successful scans must retain sessions. Failed refresh cannot refresh expired evidence. Recheck cannot eliminate between-check plain-HTTP spoofing; UI must retain the noncryptographic label.

## Same-subnet collision

R: current transport fails closed when competing active adapters can reach the selected private endpoint; supervised earlier evidence recorded MG11 Wi-Fi/USB collision separately from USB-only success. P: keep subnet_conflict and ask owner to disconnect the competing link. No automatic adapter disabling, route edits or exception for a hardware model.

O: Microsoft [IP_UNICAST_IF](https://learn.microsoft.com/en-us/windows/win32/winsock/ipproto-ip-socket-options) selects an outgoing IPv4 interface (network-order index); [IPV6_UNICAST_IF](https://learn.microsoft.com/en-us/windows/win32/winsock/ipproto-ipv6-socket-options) is the IPv6 option. P: a future direct interface-bound socket must also bind local source, verify selected route, safely resolve/pin address and enforce HTTP/TLS/cancellation bounds. Applying a socket option does not prove the current WinHTTP socket uses it. That transport remains unimplemented/unqualified, and the current collision block must not be relaxed before separate tests/review.

## Planned tests (not run)

All use synthetic snapshots, fake route/DHCP providers, temporary app profiles and owned loopback peers; no owner's DB, vault, device or host route table is changed.

| Test | Required assertion |
| --- | --- |
| Canonical vectors, reordered adapters/addresses and Unicode labels | Stable digest for same facts; labels do not participate; distinct null/empty/family/scopes cannot collide by encoding. |
| Each bound field changes independently, replug and restart | Session/proofs expire, no stale success; restart cannot load an old in-memory binding. |
| Identical rescan / failed or cancelled refresh | No spurious logout; no freshness extension after failure. |
| Missing/truncated DHCP native data, static config, known→unknown | Safe bounded parsing, unknown marker; no guessed DHCP from DNS/gateway. |
| Wi-Fi + USB same subnet / endpoint substitution / identity conflict | Fail closed with zero approved-read dispatch. |
| Change between before/after snapshots, timeout and cancellation | Read output not published; guard resources released; subsequent fresh approval works. |
| Hypothetical bound socket on IPv4/IPv6 multihomed fixture | Captured outgoing interface/source exact; route change, interface disable/index reuse fails safely; no fallback to other adapter. Required before future release. |
| Diagnostics/UI | Exact noncryptographic label, no MAC/label→identity upgrade; transport qualifier survives serialization. |

## Open questions

| ID | Proposed answer |
| --- | --- |
| Q1 | Accept GUID + boot-scoped LUID and in-memory digest, excluding display labels and routine revision? Yes. |
| Q2 | Missing DHCP stays unknown; which later read prerequisites permit it? Proposed read-only recognition may display unknown, but no operation requiring DHCP evidence passes. |
| Q3 | Known↔unknown DHCP/correlation transition invalidates existing session? Yes; no silent downgrade. |
| Q4 | Interface-bound socket qualification should be a later transport release; keep current collision rejection now. Proposed yes. |

Scope: design only. Files changed: this document and its PHASE_STATUS.md entry. Checks: source reads, official-document review/citation and section coverage, `git diff --check`; no build/test execution or owner-data access for this document. Still blocked after approval: B5 implementation until separate release, authenticated hardware/client, Stage 3/5/6 code and Phase 8. Handoff: `git diff phase-7-5-stage-2a-auth-design..HEAD -- docs/PHASE_7_5_STAGE_4_BINDING_DESIGN.md docs/PHASE_STATUS.md` on phase-7-5-design-track.


## Reviewer decisions, 24 October

Design accepted. These decisions supersede the proposals and open questions above.

- **Q1 yes** (GUID plus boot-scoped LUID, in-memory digest, labels and routine revision excluded). **Q2 DECIDED:** an unknown DHCP server does not block recognition or the authenticated identity read; it is displayed as "unknown" and encoded as unknown in the digest. No future operation may require DHCP evidence without its own review. **Q3 yes**: known to unknown and unknown to known transitions invalidate the binding. **Q4 yes**: interface-bound socket stays a later release and the collision block stays.
- Add the DHCP-server field to discovery only through `GetAdaptersAddresses` as documented (valid length, `IP_ADAPTER_DHCP_ENABLED`, a valid IPv4 `SOCKET_ADDRESS`); never from DNS or gateway.
