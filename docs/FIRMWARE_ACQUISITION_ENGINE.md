# Firmware acquisition engine

## B1 design proposal - awaiting review, no B1 implementation

23 October 2026, Africa/Lagos. R1-R5 are accepted at `09481b9`. This section proposes B1 and previews B2/B4/B5; the existing Sprint A implementation described below remains in use. No strategy trait, vendor login, transfer worker, schema migration or new request path is implemented here. Code for B1-B6 waits for the reviewer's design approval, as required by the owner's R6-R10/B1 guideline. No Phase 8 or physical OpenWrt qualification is included.

### Proposed contracts and approval ownership

The following Rust is a design sketch, not compilable application code. IDs are closed backend types, not caller-supplied URLs, names or commands.

```rust
enum StrategyKind {
    OfficialImage,       // official_image
    DeviceExport,        // device_export
    StorageRead,         // storage_read
    ManufacturerPackage, // manufacturer_package
    ExternalLab,         // external_lab
}

trait AcquisitionStrategy: Send + Sync {
    fn id(&self) -> StrategyId;
    fn kind(&self) -> StrategyKind;
    fn applicability(&self, ctx: &EvaluationContext<'_>) -> Applicability;
    fn probe(
        &self,
        ctx: &mut ProbeContext<'_>,
        approval: ClaimedApproval,
    ) -> Result<ProbeReport, StrategyError>;
    fn plan(
        &self,
        ctx: &EvaluationContext<'_>,
        probe: &ProbeReport,
    ) -> Result<AcquisitionPlan, StrategyError>;
    // No execute, write, flash, reboot or arbitrary transport entry point.
}

struct Applicability {
    state: CapabilityState,
    evidence_ids: Vec<EvidenceId>,
    missing: Vec<Requirement>,
}
struct ProbeReport {
    strategy: StrategyId,
    state: CapabilityState,
    provenance: ProvenanceKind,
    expected_class: ArtifactClass,
    evidence_ids: Vec<EvidenceId>,
    missing: Vec<Requirement>,
    limitations: Vec<LimitationCode>,
}
struct AcquisitionPlan {
    strategy: StrategyId,
    provenance: ProvenanceKind,
    expected_class: ArtifactClass,
    binding_digest: BindingDigest,
    evidence_ids: Vec<EvidenceId>,
    missing: Vec<Requirement>,
    // No executable job/command or capability token.
}
```

`EvaluationContext` exposes only immutable, current backend identity/evidence, family candidates and qualification records. `ProbeContext` additionally holds cancellation/deadline/resource limits and a sealed request broker. Strategies cannot construct an HTTP/SSH client, access ambient credentials, mint trusted evidence, or turn a plan into a job. Broker methods accept a fixed family request ID and borrow the already claimed proof for its bounded request bundle. They validate scope before every dispatch and before publication. Secrets are separately borrowed from the existing `Secret` path only inside the approved authenticated broker; they are not a field in reports or evaluation contexts.

The current `approval::ClaimedApproval` is non-Clone, non-serializable and privately constructed. Its existing binding authorizes root/static metadata only. The future internal binding must explicitly add an `ApprovalScope` enum: `EndpointMetadata`, `FamilyIdentityRead`, `StrategyProbe { strategy, requests }`, and later a separately reviewed acquisition scope. `Approvals::claim` must check the expected scope before returning this same opaque proof by value. Old metadata receipts retain their exact current scope and 30-second expiry; they cannot be upgraded/wrapped into a strategy receipt. A strategy receives only a strategy-probe-scoped proof; the broker rejects the other scopes. No acquisition-scope proof is issued while no qualified executable strategy exists.

`applicability` and `plan` are pure local evaluation. Probing is an optional, separately approved read, not an acquisition. Unimplemented strategies return `not_implemented` without dispatch. Plan creation never changes `canObtain`; that needs a future reviewed executor, exact target/source qualification and its own approval. `external_lab` has no in-app transport at this gate.

| Existing static entry | Proposed stable ID / kind | Expected artifact class | Current executable status |
| --- | --- | --- | --- |
| OfficialImage | `official_image` / official_image | official_image_download | None; publisher registry empty |
| DocumentedExport | `documented_export` / device_export | device_firmware_export | None; no qualified binary export |
| AuthorizedStorageRead | `authorized_storage_read` / storage_read | physical_flash_dump | None; no qualified ranges/adapter |
| ManufacturerPackage | `manufacturer_package` / manufacturer_package | official_image_download, only after reviewed non-executing package validation | None; never run an updater |
| ExternalLaboratory | `external_laboratory` / external_lab | physical_flash_dump, only with actual coverage/provenance | None; separate expert workflow |

The five current wire IDs stay stable, preserving schema-1 report/frontend compatibility. A future additional registry entry is a module plus explicit compiled registration. It does not require editing model branches in registry evaluation. There is no dynamic DLL/script loading.

### Evidence flow and authority

Discovery -> immutable observed fingerprint -> candidate family -> scoped read approval -> bounded parser output -> central identity verifier -> per-requirement strategy evaluation -> nonexecutable plan. Each arrow carries evidence IDs, collection time, source, verification, reliability, endpoint/interface/generation and expiry. Stale/conflicting evidence is retained for diagnosis but cannot satisfy a current prerequisite.

Strategies may read current Windows observations, user reports, static root/version observations, authenticated management records and separately qualified publisher/storage records. They may emit sanitized observations and missing requirements. Only central source-specific verification can create a trusted verified record; neither the strategy nor the frontend accepts a caller-supplied `verified=true`.

An observed or user-reported value can never satisfy `Requirement::Verified(...)`, even if it textually equals a verified value. `/version_num.txt` remains observed, as do genuine HTML references, USB IDs, DHCP data and HTTP success. No count of weak signals upgrades it. A spoofed page containing real script tags can still appear as a family candidate; this never proves identity, permits authentication, or enables firmware extraction. Exact proprietary firmware comparisons remain exact opaque-string matches, with no numeric/date inference.

Authenticated reads can provide field-specific `authenticated_management` evidence after credential authentication, endpoint binding, qualified response parsing and independent consistency checks. An authenticated response that lacks manufacturer/model/revision does not verify those fields. Plain HTTP retains its explicit transport limitation below; verification means a qualified authenticated statement under that binding, not cryptographic endpoint assurance. Conflicts invalidate the session/plan instead of choosing a convenient identifier.

Proposed tests: observed equals trusted text but still fails a verified prerequisite; an HTTP-200/two-tag spoof remains candidate; missing exact revision blocks a revision-specific route; expired records, mixed endpoints, changed generation and conflicting identities invalidate plans. All routes remain nonexecutable without a qualified worker.

### Vendor modules and read allowlists

A profile selects a closed `ProtocolFamilyId` such as `asr_duster_xml`; it contains no URLs, query strings, scripts or commands. A family module owns `RequestSpec { id, method, canonical_path, response_types, max_bytes, parser, max_attempts, budget }`. Only registered IDs reach the broker. UI input can select a validated endpoint/interface, never a path or file name. Family candidates only suggest applicability; invoking a family identity read still requires its separate approval.

The proposed ASR family starts with the existing root (16 KiB) and conditional static version (4 KiB) contracts unchanged. Authenticated identity candidates below would be XML, capped at 16 KiB each, one request plus at most one bounded Digest challenge/retry, five seconds per request, and a 15-second bundle cap starting when the owner confirms and supplies credentials. These are proposed limits for review, not qualified hardware settings. Unexpected media types, oversized/truncated XML or malformed envelopes yield no identity evidence. Parsers reject DTDs/entities, duplicate identity fields and excessive depth; contents are never executed/rendered.

Broker rules, applied both to registry validation and each dispatch:

1. GET only. Reject POST/PUT/PATCH/DELETE, bodies, arbitrary query parameters, commands and form/session automation. `method=get` is necessary but is not proof of a read-only server operation.
2. Exact match of one canonical compiled path/query. Reject encoded separators, double encoding, traversal, userinfo, fragments and alternate hosts. Decode once for validation, reject residual encodings, and compare against the canonical specification.
3. Case-insensitive deny check over the decoded entire path/query for `set`, `upload`, `upgrade`, `reboot`, `restore`, `reset`, `factory`; conservative false exclusions remain blocked for review. Also deny action/command parameters even on a GET path. No escape hatch from profile or IPC.
4. No redirects at all for authenticated reads; therefore no cross-host redirect. For future HTTPS publisher reads, any reviewed same-host redirects remain bounded; cross-host redirects always require a different reviewed source/approval. Credentials never follow redirects.
5. No proxy, default credentials, browser cookies or credential guessing. Same endpoint/interface/source binding, time, response-size and cancellation checks apply at every step. Unknown/incomplete bindings fail closed.

Planned table-driven tests inspect every registered request and reject each forbidden substring, HTTP method, caller path, encoded bypass, cross-host redirect, unexpected media type and oversize body. A vendor GET that performs a reset is prohibited even if its JavaScript calls it a read. The captured scripts actually contain such calls; do not import their generic helper wholesale.

### Model-independent selection

Runtime selection uses protocol family plus qualified identity/evidence predicates; no `if model == "MG11"` or brand fallback. Model/revision values remain data in exact qualification records outside generic acquisition code. Planned regression recursively scans runtime `.rs` under `src/acquisition` for the current hardware-matrix model/brand strings, including case-folded forms; test fixture strings are isolated/excluded from runtime scanning. A second test runs equal evidence through two different user labels and asserts equal applicability. New target names must be added to the scan's external fixture list. This is a planned test, not implemented in B1 design.

### Strategy approval, errors and invalidation

The backend first resolves the current target, builds a preview of exact request IDs/limits and obtains a resource lease. Only then may it claim the native-confirmed receipt, preserving the R5 BUSY behavior. A strategy-probe receipt binds device selection, canonical endpoint, interface ID and source address, current network-binding digest, identity evidence IDs/reliability, discovery generation, strategy ID/version, operation, request bundle, maximum bytes and expiry. If identity is still unknown, that fact is bound explicitly and only identity/bootstrap reads can be proposed. Protected host hardware, ambiguous targets, stale grants and identity conflicts remain denied.

Human confirmation has a proposed five-minute preview expiry. After confirmation, a separate short dispatch lease starts; a stale preview cannot be silently refreshed. Credentials are entered only after the final target/scope is displayed and are erased on completion/cancel/expiry. These strategy limits do not change the current endpoint metadata receipt. A later firmware transfer needs a new approval bound to the actual immutable plan and destination/class; a probe receipt cannot authorize transfer.

On gateway, source address, DHCP server, interface, USB correlation or identity change: cancel in-flight work, revoke all related proofs/session authority, discard stale successful publication and require fresh approval. A routine identical discovery snapshot does not invalidate a session. Unsupported optional metadata is a limitation; transport/identity/trust failure invalidates authority. No automatic retry after cancellation, identity mismatch, lockout or 5xx.

| Proposed code | Meaning / UI recovery |
| --- | --- |
| APPROVAL_REQUIRED / SCOPE_MISMATCH | No matching proof; preview and approve the exact operation |
| APPROVAL_EXPIRED / ALREADY_CLAIMED | One-shot lease unavailable; request a new confirmation |
| BUSY | No receipt consumed; retry when prior operation releases its guard |
| ENDPOINT_CHANGED / STALE_EVIDENCE / IDENTITY_CONFLICT | Cancel/revoke; fresh observation and approval required |
| PROTECTED_TARGET / AMBIGUOUS_TARGET / ENDPOINT_UNTRUSTED | Deny; no probe/transfer |
| AUTHENTICATION_FAILED / LOCKOUT_DETECTED | Stop; erase Secret; no credential retries/guessing |
| CANCELLED / TIMEOUT | Release sockets/guards/staging ownership; no stale result |
| REQUEST_DENIED / RESPONSE_TOO_LARGE / INVALID_RESPONSE / REMOTE_5XX | Stop this bundle; no authority emitted |
| NOT_IMPLEMENTED / REQUIREMENTS_MISSING / UNSUPPORTED | Neutral unavailable route with exact missing facts |

These map onto existing adapter cancellation/deadline/error conventions, rather than creating a second session manager. Diagnostics contain stable codes/counts and redacted evidence references; never raw XML, authentication headers, device identifiers or secrets. Proposed tests cover single-use/scope/endpoint substitution, busy preservation, cancellation/timeout cleanup, post-read binding changes and stale publication.

### Artifact classes and migration preview (B2)

| Class | Allowed content | Firmware Library / future pipeline eligibility |
| --- | --- | --- |
| official_image_download | Qualified publisher firmware or a validated firmware payload from a reviewed package | Firmware candidate; exact-device and provenance checks remain independent |
| device_firmware_export | Qualified actual firmware export, not settings | Firmware candidate with coverage/type evidence |
| physical_flash_dump | Actual bounded flash bytes and range/coverage manifest | Firmware candidate; partial dump is never a complete backup |
| configuration_backup | Settings/configuration archive | Never firmware, extraction, rebuild or flash input |
| metadata_report | Sanitized observations/diagnostic report | Never firmware or a backup of its bytes |

Class, provenance and acquisition occurrence are separate from blob SHA-256. Content-addressed deduplication may share identical blob bytes but can never promote a configuration/metadata occurrence into a firmware entry or merge its authority. Filename, extension, MIME type and user label cannot choose a firmware class. Only the qualified typed producer may request one of the three firmware classes; byte/hash validation alone is insufficient. Configuration/metadata results go to their own filtered records/views and never enter `firmware_artifacts` or firmware counts.

A migration is required before B2 production code: schema 5's `artifact_imports.source_category` CHECK in `migrations/004_firmware_library.sql` admits only the five legacy categories; Library DB/UI also assume firmware records. Proposed schema 6 adds a neutral occurrence/class/provenance envelope referencing the same Library-owned blob storage, and enforces firmware-only insertion through typed producer/DB constraints. Keep old source categories and all legacy rows/IDs untouched; their new class stays unknown/legacy until independently classified, never guessed from `authorized_device_acquisition`. Configuration/metadata envelopes have no firmware-table link. The exact SQL is a B2 review deliverable, not part of B1.

Migration plan: isolated fixture DB first; exclusive coordinated Library admission, consistent pre-migration SQLite backup, transaction, integrity/FK and row/hash checks, then user_version bump. Failures before commit roll back the transaction; bytes/storage locations do not move. After populated schema 6, schema-5 binaries must refuse it. Downgrade requires a verified pre-migration backup plus explicit handling/export of any later records; never drop new classes or coerce them into firmware for compatibility. Test existing data byte/row preservation, interrupted migration rollback, duplicate blobs across classes, rejection of configuration-as-firmware, and old-version refusal. No migration is executed now.

### Plain-HTTP endpoint binding (B5 preview)

Show **"not cryptographically verified"** beside any plain-HTTP identity/session. Bind the interface's stable Windows ID/LUID, local source address/prefix, selected gateway, DHCP-server address, USB-to-NIC correlation, request endpoint, evidence IDs and session generation. The placeholder `00-CC-CC-CC-CC-CC`, any other MAC or matching subnet alone is never identity. The existing discovery model does not yet expose DHCP server identity; missing information stays unknown and may block qualification, never synthesized from DNS or gateway values.

Recheck the entire binding and minimal approved identity fields before and after every session/read bundle. USB replug, route/interface/source/DHCP change or conflicting identity cancels and revokes; do not silently follow another gateway. Current same-subnet collision block remains until a source/interface-bound transport is independently qualified. WinHTTP behavior is not presented as strong source binding. Future socket binding needs IPv4/IPv6, route, TLS, cancellation and collision tests before use. Plain HTTP remains susceptible to interception despite these consistency checks; authenticated Digest is not a substitute for a cryptographic host identity.

### Authenticated-read proposal and device-JavaScript citations (B4 preview)

Sources were read as text from the owner's existing local `scratchpad/mg11` capture on 23 October; no device request, JavaScript execution or credential entry was performed for this design. Paths below are the device's `/js/...` paths and one-based original line numbers. The shared helper at `/js/base/ajax_calls.js:349-364` builds `xml_action.cgi?method=get&module=duster&file=` plus the name, explicitly uses HTTP GET and `getAuthHeader("GET")`. `/js/base/utils.js:245-246` maps `getData` to that helper.

| Exact `file` name | Device's own call site | Evidence / proposed disposition |
| --- | --- | --- |
| `status1` | `/js/panel/internet/internet_connection.js:1853`, `callProductXML("status1")`; helper `/js/base/ajax_calls.js:355-361` | Candidate for a reviewed minimal identity/status read only. Nearby line 1854 consumes IMEI: parser must discard subscriber/device-unique fields from reports. This citation proves the GET call, not safe server semantics or sufficient exact identity. Not authorized yet. |
| `wan` | `/js/panel/internet/pin_puk.js:18`, `callProductXML("wan")`; same GET helper | Documented name, deferred from initial identity allowlist: SIM/APN metadata is unnecessary and may be sensitive. |
| `Engineer_parameter` | `/js/panel/dashboard.js:48`, `getData("Engineer_parameter")`; `/js/base/utils.js:245-246`, then same GET helper | Documented name, excluded from initial allowlist pending semantic review of engineering data. No engineering/control request is proposed. |

No other file name is proposed, inferred from the vendor, or borrowed from another device. In particular, the software-upgrade page uses a variable `xmlName`; its `version_num` display does not identify a safe literal identity filename. The exact read yielding manufacturer/model/hardware revision is still an open question. Do not invent `device_info`/`version` endpoints or treat `status1` as sufficient before an approved response parser is qualified. Static `/version_num.txt` remains the already implemented unauthenticated observed-only path and is not a CGI filename.

Capture SHA-256 anchors: `ajax_calls.js` `6432420bc1aefcb7990b7180664bf0be720e434ae952093f1f6a1fde9fd524e1`; `utils.js` `862497a77554c3d67da5fbed611bf13d9df9bc126bf2a7af882c997cb89c6f68`; `internet_connection.js` `84f54df2fd9e91ec34fe9a43c1f7d368d0776367244d39293c9b8df4364e3dee`; `pin_puk.js` `e05c11c3e1520f46c280018a53b9ab4290d2201110bd7893fa78dc45899643ac`; `dashboard.js` `f57e620d871ec877920d1a04cef4a7e41eb2db0af20c77bd457b967c40ae381d`. Captured text is evidence for design, not executable code to vendor into the app. The current capture resides at the owner-supplied guideline's sibling `scratchpad/mg11` location; no full vendor scripts are newly committed.

First authenticated read procedure, after design and request-specific review:

1. Owner powers/connects target and confirms endpoint/interface. Review source citations and the single proposed GET's server semantics/expected safe fields; if read-only behavior cannot be established, stop before credentials/dispatch.
2. Native confirmation names the exact identity request, privacy scope, plain-HTTP limitation and limits. Credentials are owner-supplied at runtime through `adapters/lifecycle.rs::Secret` only; never default, derived from IMEI, guessed, stored, printed or exported. Do not reuse the vendor page's credential derivation or browser login flow.
3. Owner supervises a single request with at most its approved Digest challenge/retry. Validate the challenge origin and URI: captured `/js/base/utils.js:142,159` references `/cgi/xml_action.cgi`, while the request helper uses `/xml_action.cgi`. This discrepancy must be resolved from the actual approved challenge before a client is written; no alternate-path probing.
4. Stop immediately on any sign of a write, any 5xx, lockout/unauthorized text, redirect, unexpected content, changed binding, or a request not demonstrably GET-only. Erase Secret, revoke pending grants and publish a sanitized limitation/error, not a verified identity.
5. Qualified successful parsing may verify only returned identity fields under the bound session and source policy. Recheck binding/identity before publishing; otherwise discard. Record owner observations with exact words/date/source and web-display-only scope. No extraction/export or internal-storage preservation claim follows from this read.

### Risks and open questions for review

- Is `status1` a safe and sufficient identity source? Current citation proves a GET and an IMEI consumer, not manufacturer/model/revision fields or side-effect-free server semantics. Default: do not dispatch until request-specific review.
- Which literal GET yields the displayed software version and exact retail identity? Variable-based page code is insufficient. Missing fields remain unknown; no inference from `REL_MRVL...`, `MF69...` or `LT800`.
- How does the actual Digest challenge reconcile `/cgi/xml_action.cgi` and `/xml_action.cgi`? No login adapter until resolved; no guessed fallback URI.
- What policy, if any, accepts authenticated identity over noncryptographic HTTP for a specific device? Keep the explicit limitation and separate field verification; never allow firmware read/write based only on transport binding.
- How will DHCP-server evidence be exposed and what exact source-binding transport will be qualified? Current missing field/collision guard remains authoritative until B5 review/testing.
- The proposed schema-6 neutral envelope and classification constraints require a separate B2 SQL/rollback review. Do not contaminate firmware history with configuration/report records.
- Registry composition, qualification records and trusted publisher signing need separate reviewed persistence/version rules. No dynamic plugins, official-image URLs or acquisition executor ship from this proposal.

Design review decision requested on these contracts and unresolved B4/B5 gates. No B1-B6 implementation begins before approval. B7 registry and B8 worker remain unimplemented/design-only future work; all extraction, physical writes, flashing and live USSD stay disabled.

## Existing Sprint A implementation

Sprint A implements Rust-managed route discovery, not firmware transfer. `acquisition/mod.rs` owns typed device fingerprints, identity/evidence assessments, vendor-family candidates, five independent strategy candidates and 16 capability assessments. `acquisition/ipc.rs` re-resolves current inventory, uses existing bounded connection leases, rejects stale results and caches at most 64 recent reports. Diagnostics are generated from backend state with raw identifiers, endpoint addresses, reported identity text and peer response text excluded.

Strategies: exact official image, documented device export, qualified authorized storage read, authentic manufacturer package, external laboratory acquisition. No qualified artifact catalog or executable acquisition worker is registered in Sprint A. Every strategy remains unknown with specific missing requirements. This means missing implementation/evidence, not confirmed hardware incompatibility.

Existing Library owns all future bytes, jobs, hashing, content-addressed storage, restart handling, backups and restoration. Sprint B must add provenance and acquisition streaming there. No second vault or database is created. Exact model/revision/firmware/region/image type and source trust are independent prerequisites. Official images, device exports, partition dumps, full backups and configuration archives must retain different provenance.

No acquisition authorization, job, artifact or successful extraction is invented in Sprint A. Those executable contracts belong to reviewed Sprint B/C implementations. Source mismatch, redirects, untrusted manifests, checksum failures, interrupted transfers and disk-full qualification remain Sprint B gates. Independent analysis/build/installation/recovery gates remain closed.

## Sprint A close-out approval and routing

Endpoint metadata needs a backend-issued receipt following the Phase 5 bound, expiring, single-use approval pattern. A native Windows confirmation names device, interface, normalized endpoint and advanced scope. The receipt expires 30 seconds after confirmation, is consumed on an attempted claim, and cannot transfer across device, discovery revision, interface, endpoint or advanced flag. No approval means local evidence evaluation only. Default endpoints must be numeric gateways of the selected active physical interface; DHCP server discovery is not currently exposed, so a DHCP-only address requires explicit advanced approval. Advanced addresses still require private, on-link scope and the existing protected-host and route checks. This receipt permits only root metadata and a recognized-family static version file; it grants no authentication, session reads or acquisition.

Two independent static root references are needed for candidate `asr_duster_xml`; E-LINK is a brand hint only. Only then may the same approval collect `GET /version_num.txt`. Version text is endpoint-response/observed/observed evidence in the bounded cached report, never trusted firmware identity or a strategy prerequisite. Owner-reported firmware stays separately displayed. The root response cap is an informational fact, not a warning. Other metadata requests, vendor login and authenticated identity remain Sprint B work.

Overlapping active physical networks return `subnet_conflict` before any TCP/HTTP dispatch, naming the other interface in the UI. Diagnostics omit that name. Windows routing remains unchanged. A later client could bind a Winsock socket with `IP_UNICAST_IF` (and corresponding IPv6 handling), then qualify source selection, TLS, deadlines, cancellation and endpoint binding under collision. This is design only: the current WinHTTP client must fail closed and cannot claim such binding.

Device detection is `observed`. Capabilities without an executable code path are `not_implemented`, neutral and nonexecutable. Unknown identity and acquisition-source applicability remain `unknown`; one stage never enables another. Five summary stages precede collapsed remaining capabilities. See [close-out evidence](PHASE_7_5_SPRINT_A_CLOSEOUT.md).
