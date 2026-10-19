# Phase 5 — Device adapter framework

Implementation date: 19 October 2026. Phase 5 adds backend contracts and passive adapter evaluation. Physical management transports remain deferred to Phase 7; the complete stateful simulator remains Phase 6. Acceptance and executed commands are recorded separately in `PHASE_5_VERIFICATION.md`.

## Audit and implementation map

The existing npm/Cargo workspaces and `apps/desktop/` location are retained. No migration or data-directory change is necessary. The resumed worktree contained the initial Phase 5 edits; those were inspected and hardened in place. Prior UI cleanup and the device-card grid remain intact. No reset, checkout, history rewrite, application-data deletion or device configuration action was performed.

| Existing implementation | Extension and ownership |
| --- | --- |
| `src-tauri/src/discovery/mod.rs`: `assemble`, `DiscoverySnapshot::reconcile` | Stable SHA-256 logical IDs; USB parent/container correlation, network observations, presence and discovery revisions remain authoritative for Windows observations only. |
| `discovery/windows.rs`, `discovery/service.rs`: `run_worker`, `scan_and_reply` | Existing SetupAPI/IP Helper capture, notification/debounce and bounded refresh retained. New session invalidation observes committed discovery revisions without issuing device requests. |
| `discovery/identity.rs`: `resolve` | Adds evidence-aware `is_host_hardware`. A host PCIe Wi-Fi or Ethernet interface is distinct from its reachable gateway. External USB network candidates remain candidates. |
| `profiles/mod.rs`, `profiles/engine.rs`: `Registry`, `gather_evidence`, `evaluate` | Existing three-way assessments, readiness requirements, independent feature decisions, strong authenticated evidence, revision/expiry and firmware constraints are reused. Opaque identifiers retain literal matching. |
| `commands.rs`: existing compatibility/manual-router commands | New passive adapter commands resolve targets from backend discovery or saved rows, gather existing evidence and reject changed discovery before returning. No raw endpoint, credential or profile object is accepted as authority. |
| `connection.rs`: existing endpoint probes | Existing separately confirmed, bounded HEAD checks remain unchanged. An HTTP response never becomes authenticated adapter evidence. Phase 5 checks do not invoke physical endpoint probes. |
| `storage.rs`: `Repository::open`, `identity_observations`, `routers` | Schema 5, normal `%LOCALAPPDATA%\\OBTAINER`, settings, firmware and saved identity rows retained. Adapter evaluation is read-only; sessions and secrets have no new tables or migrations. |
| `library/`, current native-test ownership infrastructure | Existing isolated paths, marker/lock checks, generated harmless binaries and guarded cleanup are reused. Mock adapter is compiled with `cfg(test)` only, including when native-test feature is enabled. |
| `lib/backend.ts`, `types/profiles.ts` | Adapter responses receive independent strict Zod validation and reuse assessment/readiness schemas. Backend errors remain separate from compatibility results. |
| `features/Compatibility.tsx`, `features/ManualRouters.tsx` | Small reusable adapter-status panel appears on compatibility selection and on explicit saved-endpoint request. Six destinations and page-help controls remain unchanged. |

Paths in this table are relative to `apps/desktop/` unless otherwise marked. The earlier baseline reports are historical evidence, not new results. Initial scaffolding preceded a full pre-change test run; final verification executes retained regressions against the completed source. A read-only normal-data fingerprint was captured before this continuation's native launches.

## Core contracts

`src-tauri/src/adapters/` contains working registry, policy, authentication, lifecycle and test implementations:

- `types.rs`: immutable adapter metadata, typed operations, authentication and connection states, passive query request, per-operation availability and per-feature assessment/readiness. APN, radio/BSS/client limits, separate upstream/LAN IPv4/IPv6 DNS, USSD capabilities and typed future change plans are defined without executable payloads.
- `contracts.rs`: core `DeviceAdapter`, optional `ReadOnlyConnectionAdapter`, backend-owned `AdapterConnection`, independent feature readers, optional planning/backup/apply/verify/restore and USSD session interfaces. Missing optional interfaces return explicit unsupported results through the service. No placeholder reports successful writes.
- `registry.rs`: validated registration, immutable descriptor snapshot, duplicate rejection, evidence matching and deterministic ranking. Production registers only the declared OpenWrt contract. No working physical OpenWrt adapter is claimed.
- `policy.rs`: scoped `AuthorizationGrant` and deny-by-default `authorize`. Grant has no Deserialize implementation, so frontend data cannot impersonate backend authorization.
- `auth.rs`, `lifecycle.rs`: non-serializable in-memory secrets; real deadline/cancellation checks, authenticated handshake and identity validation, two-session/attempt resource limit, expiry/revision invalidation and bounded cleanup.
- `errors.rs`: stable structured error categories, safe message, operation, retryability and suggested action. Internal lock/transport details do not flow into public errors.
- `mock.rs`, `tests.rs`: deterministic test-only connector and actual session/resource checks, not a physical adapter or the Phase 6 simulator.

`Registry::register` validates IDs, numeric descriptor versions, bounded strings/collections, duplicate operations/transports and implementation/connector consistency. Production registration rejects test-only descriptors instead of merely hiding them in UI. Registry content cannot carry shell/modem/SSH execution strings.

## Matching and capability reporting

Sequence: Windows observations → existing identity/profile evaluation → independent adapter constraints → authorization → trusted authenticated connection → capability/identity reads. The last three active stages have only isolated unit-test implementations in Phase 5.

Descriptors declare supported transports, profile IDs and/or firmware families, authentication, operations, dependencies and limitations. Matching checks the backend report's device ID, current discovery revision, registry digest, profile version/content hash, expiry, conflicts and exact authoritative hardware/firmware result. Partial matches need meaningful matching fields and compatible profile constraints; the presence of an unmatched reference profile in the registry is insufficient.

A unique exact adapter takes precedence over weaker candidates. Two exact matches or multiple partial-only candidates produce ambiguity. No active connection is inferred from any match. Missing/stale/conflicting identity, wrong hardware/firmware and untrusted user labels remain conservative. Profile assessments can describe documented support while adapter read/write/verify/rollback permissions remain false. Those concepts use the existing assessment/readiness enums rather than a second incompatible support model.

An explicitly selected saved endpoint is evaluated separately from the host NIC used to reach it. Until authenticated endpoint identity exists, its address/label/history produces no adapter selection and no synthetic discovery device. No network request occurs while evaluating or displaying adapter status.

## Authorization, authentication and sessions

The backend-only session binding includes device ID, adapter ID/version, exact profile ID/version/content hash, evidence digest, discovery revision, endpoint, Windows interface, endpoint identity pin, expiry and simulation provenance. Authorization grants bind to that entire value plus intended operations and a maximum 60-second lifetime. Changed endpoint, interface, hardware/firmware/profile evidence or discovery invalidates authority. An authentication flag sent by React is never used by this framework.

Authentication methods cover password, SSH key, challenge response, session token and documented vendor protocols. Credentials are not accepted through adapter IPC. `Secret` has no serialization or cloning, redacts Debug and clears its owned bytes using volatile writes on drop. Temporary copies outside this container cannot be protected by the container; future transports must avoid secret copies, command arguments, debug output and plaintext persistence. No credential persistence is implemented. Future Windows-protected storage requires a separate reviewed implementation.

A connector must validate its protocol's endpoint trust (SSH host key, HTTPS certificate or documented modem identity), authenticate and return a read-only connection. `SessionManager::connect` validates returned manufacturer/model, exact hardware revision, firmware constraints and simulation provenance before publishing connected state. Returned identity does not promote discovery or write authoritative evidence to SQLite. Test sessions explicitly report simulated provenance.

Deadlines are monotonic and capped at 60 seconds overall. Stage limits are connection 10 s, authentication 8 s, identity 4 s, capabilities 6 s, ordinary read 5 s and disconnection 2 s; a stage never extends its parent deadline. Cancellation is cooperative and checked before/after work and between connector IO calls. Only two sessions or in-flight connection attempts can hold slots. RAII releases slots/resources on failure, cancellation and unwinding. All reads recheck scoped authorization; cancellation or stale results cannot publish a session/result. Discovery commits invalidate prior revisions. Shutdown closes sessions and prevents new publication; expiry is reaped on access/new connection or explicit backend maintenance.

Physical modification, restore, firmware acquisition/installation, active USSD and firmware execution remain hard-denied independently of descriptor claims. Production has no connector and cannot report connected read-only. Actual SSH, HTTPS or modem authentication, automatic trust installation, credential guessing and TLS bypass are not implemented.

## IPC and UI

New approved commands are `list_available_adapters` and `get_device_adapter_matches`. They are registered in `lib.rs`, the Tauri app manifest in `build.rs`, generated command permissions and `capabilities/main.json`. Query input is a strict `AdapterQuery` with canonical SHA-256 target ID and `discovered`/`saved_endpoint` target kind. Unknown fields, stale references and absent saved targets fail safely. No connect, credential, arbitrary command, write, USSD request or extraction endpoint is exposed.

`src/types/adapters.ts` validates descriptors, matches, requirements, states, capability reports and errors. Passive results that claim an established connection, test-only physical adapter or writable feature reject. `src/features/AdapterStatus.tsx` has independent loading/error/retry/expiry behavior, hides stale results and ignores late responses after target/revision change. Detailed requirements remain expandable. Adapter failure does not suppress the existing four-feature matrix. Extraction and customization controls remain disabled.

## Threat model and qualification limits

| Risk | Enforced boundary and test |
| --- | --- |
| Spoofed name, USB IDs or HTTP success | Existing profile evidence rules; no authoritative adapter from labels alone. |
| Adapter confusion | Deterministic exact precedence; ambiguous exact matches select nothing. |
| Host hardware as router | Host PCI/ACPI/software NICs, HID/storage and unrelated devices reject independently of a spoofed category. Gateway reachability does not identify the host NIC as a router. |
| Stale approval/session | Entire session binding, expiry, profile/registry version and discovery revision rechecked; changed discovery invalidates sessions and late publication. |
| Endpoint substitution | Binding compares endpoint/interface/pin and trusted profile evidence; connector must verify the observed endpoint before handshake success. Test pin substitution rejects. |
| Credential leakage | No adapter secret serialization, database field, process argument or raw error/log path. In-memory byte clearing and redacted Debug are tested. |
| Direct IPC bypass | No active operation endpoints; malformed request and attempted write/execution commands reject in native verification. Rust policy rejects all forbidden operations even with a matching descriptor/grant. |
| Hung operations/resource exhaustion | Bounded deadlines, cooperative cancellation, two resource slots, expiry/shutdown and RAII cleanup tested with deterministic faults. |
| Test leakage | Mock module uses `cfg(test)` only. Production registry rejects test-only adapters; native snapshot remains `windows_observed`, `simulatedAdapters:false`. |

Uncooperative future native protocol code cannot be forcibly made cancellable by a trait. Each real connector must use bounded IO, close handles and check cancellation; this is an adapter qualification requirement. No physical transport has been qualified in Phase 5. Idle session scheduling, persisted configuration transactions, encrypted secret-bearing backups, chargeable USSD approval and actual recovery execution remain owning-phase work.

## Adding a future adapter

Implement immutable validated metadata and optional reviewed connector/readers. Add exact hardware/firmware evidence through the existing profile engine, explicit endpoint trust verification, bounded IO/cancellation and resource Drop cleanup. Register in the backend registry, never a screen. Prove denied unsupported operations, conflicting/stale evidence, secret privacy, authentication and disconnection behavior first with isolated fixtures, then obtain the separately required authorization for physical read-only qualification. A profile or descriptor declaration never enables feature writes. Phase 8 must supply durable plans, scoped approval, verified backups and recovery before configuration changes can run.
