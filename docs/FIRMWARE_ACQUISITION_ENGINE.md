# Firmware acquisition engine

## Current Stage 2A: credential-free challenge observation

23 October 2026: Stage 1 and the Stage 2A design are accepted. The released software adds exactly one separately approved request, `GET /login.cgi` without query, body, credentials, Authorization or Cookie. The closed ASR-family table now has four entries: root, static version, static menu and Challenge. Login, StatusRead and Logout remain unregistered. Earlier table counts below describe historical gates.

`AuthChallenge { request: Challenge }` is an opaque, single-use approval bound to device, discovery revision, interface and normalized endpoint, expiring 30 seconds after its own native confirmation. Existing metadata/menu/strategy scopes cannot substitute for it or vice versa. A fresh same-endpoint/interface root observation must already recognize the candidate family; otherwise no challenge is sent. The shared connection lease, cancellation and route/subnet guards apply before dispatch and publication.

Challenge has one attempt and a five-second network budget. WinHTTP headers are bounded to 8 KiB overall and 2 KiB for WWW-Authenticate; the application returns before any body-read API. WinHTTP/OS internal buffering is not claimed to be zero. Only a single HTTP-401 Digest challenge with well-formed quoted parameters is usable. Output contains bounded printable realm, token-list qop, optional algorithm, status and nonce/Set-Cookie/other-scheme presence flags. The hex nonce is inspected only through borrowed transient bytes; raw header buffers are erased on drop. No nonce, cookie value, body, other header value, credential or session is retained. Conservative rejection includes non-ASCII headers, escaped quoted values, folded/malformed headers and ambiguous schemes.

The additive schema-1 `authChallenge` line is endpoint_response / observed / observed, labelled `authentication challenge observed; unverified`. Diagnostics revalidate its bounds. A failed observation exposes only a bounded code, status and flags; false flags on rejected input do not establish authoritative absence. No compatibility stage, identity evidence or permission advances; `canObtain` remains false. [Software results](PHASE_7_5_STAGE_2A_CHALLENGE_VERIFICATION.md). Part 3 hardware is HELD until code acceptance and a separate release. Login, StatusRead, Logout, credential entry/Secret use, B2, B5, B7, B8 and Phase 8 remain blocked.

## Current B4 pre-step: separately approved static menu read

23 October 2026: G1 and B4 plan amendments are accepted; reviewer merged local main to `1b7fbce`. The separate B4 software branch implements only `GET /xml/ui_mifi.xml`. A fresh earlier root report must contain candidate `asr_duster_xml` for the same device/revision/interface/normalized endpoint. A separate native Yes/No dialog issues `StaticMenuMetadata { request: StaticMenu }`, expiring after 30 seconds. Root/version and strategy receipts cannot claim it or be satisfied by it. Claim consumes the receipt; an atomic transport gate permits only one attempt, including failed/cancelled attempts.

The closed family table now contains exactly three reads: root (16 KiB), static version (4 KiB), static menu (16 KiB). Menu dispatch validates the literal table entry and reuses existing route/subnet/host guards, pinned-address WinHTTP, no-proxy/no-cookie/no-credentials policy, cancellation and current-lease publication. Menu has one attempt, at most five seconds including preflight, XML media type only, no redirects or follow-up requests. Neither discovered filename nor caller input can supply another request path.

DTD/entity declarations and all entity references are refused before parsing. Non-expanding `quick-xml` reads UTF-8; conservative ASCII XML names, namespace refusal, strict declaration/envelope checks, 16 KiB size and depth 64 limits apply. Only exactly one eligible Tab/Menu entry with case-exact `implFunction="objdashboard"` may yield its `xmlName`, limited to 1–64 ASCII letters/digits/underscore/hyphen. Invalid or ambiguous input yields no filename. Raw body is transient, never logged, cached or exported.

Optional schema-1 `staticMenu` carries bounded status and endpoint-response/observed/observed filename evidence, labelled unverified. It lives only in the existing expiring in-memory report cache, outside identity/profile evidence, and is excluded from diagnostics. UI adds one observation line under Management API and a separately approved Read static menu action. No stage or acquisition permission advances; `canObtain` stays false. [Software verification](PHASE_7_5_B4_PRESTEP_VERIFICATION.md). Physical read remains held until acceptance and a separate hardware guideline. B2, authenticated B4, B5, B7, B8 and Phase 8 remain blocked. Older design sections below retain their review context; current implemented authority is described here.

## B1 design and local implementation - ACCEPTED

23 October 2026, Africa/Lagos. R6-R10 are accepted; S1/S2 and revised B1 design were accepted. The owner explicitly approved local B1/B3/B6 and model-scan implementation. B1 strategies remain non-executable stubs without I/O. The later, separately authorized static-menu path is described above. Schema migration/artifact classes, authenticated client, transport binding, publisher registry and analysis worker remain separately blocked. No Phase 8 or physical OpenWrt qualification is included.

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

Authenticated reads can provide field-specific `authenticated_management` evidence after credential authentication, session binding, qualified response parsing and independent consistency checks. Reviewer policy decision, 23 October: plain-HTTP records are labelled `transport: plain_http` and may support recognition, compatibility display and read-only probes. Their permitted uses exclude writes, flashing, installation and all irreversible steps; they never alone authorize those operations. A future irreversible step would additionally require a fresh identity read in the same session, physical USB correlation, owner confirmation of the label, B5 rechecks and separately reviewed operation-specific authority. None is enabled in this phase. An authenticated response lacking manufacturer/model/revision does not verify those fields. Conflicts invalidate the session/plan instead of choosing a convenient identifier.

Proposed tests: observed equals trusted text but still fails a verified prerequisite; an HTTP-200/two-tag spoof remains candidate; missing exact revision blocks a revision-specific route; expired records, mixed endpoints, changed generation and conflicting identities invalidate plans. All routes remain nonexecutable without a qualified worker.

### Vendor modules and read allowlists

A profile selects a closed `ProtocolFamilyId` such as `asr_duster_xml`; it contains no URLs, query strings, scripts or commands. A family module owns `RequestSpec { id, method, canonical_path, response_types, max_bytes, parser, max_attempts, budget }`. Only registered IDs reach the broker. UI input can select a validated endpoint/interface, never a path or file name. Family candidates only suggest applicability; invoking a family identity read still requires its separate approval.

The original B3 table registered root `GET /` (16 KiB) and static version `GET /version_num.txt` (4 KiB). The approved pre-step adds only `GET /xml/ui_mifi.xml` (16 KiB), with its separate scoped dispatcher above. Validator still rejects caller paths/queries and all percent-encoded path/query input, along with traversal, credentials, fragments, alternate hosts, redirects, bodies, non-GET methods and every deny-list term. Authenticated B4 remains a separately blocked multi-step challenge/login/read/logout flow. Every future authentication/read/logout spec requires its own bounds and review; menu observation grants no such request. Unexpected media types, oversized/truncated XML or malformed envelopes yield no identity authority. No raw body is retained or rendered.

Broker rules, applied both to registry validation and each dispatch:

1. Current read skeleton: GET only; reject POST/PUT/PATCH/DELETE, bodies, arbitrary query parameters, commands and session operations. `method=get` is not proof of read-only server semantics. Future B4 requires explicit separately reviewed session-state specs for login/logout; their wire GET verb does not make them read-only. No generic form/session automation is permitted.
2. Exact match of one canonical compiled path/query. Reject encoded separators, double encoding, traversal, userinfo, fragments and alternate hosts. Decode once for validation, reject residual encodings, and compare against the canonical specification.
3. Case-insensitive deny check over the decoded entire path/query for `set`, `upload`, `upgrade`, `reboot`, `restore`, `reset`, `factory`, `download`, `delete`, `format`, `flash`; conservative false exclusions remain blocked for review. The current B3 skeleton also denies every action/command parameter. Future B4 may admit only the exact named Digest-login/logout specs below with closed parameter keys and their own state-operation approval; no arbitrary `Action` exception, profile or IPC escape hatch.
4. No redirects at all for authenticated reads; therefore no cross-host redirect. For future HTTPS publisher reads, any reviewed same-host redirects remain bounded; cross-host redirects always require a different reviewed source/approval. Credentials never follow redirects.
5. No proxy, default credentials, browser cookies or credential guessing. Same endpoint/interface/source binding, time, response-size and cancellation checks apply at every step. Unknown/incomplete bindings fail closed.

Local B3 tests cover both registered specs and reject non-GET/body/path/query/encoded/traversal/userinfo/fragment/alternate-host/redirect shapes plus all eleven deny-list words. A compile-fail doctest verifies callers cannot construct `RequestSpec`; lookup confirms login, logout and `xml_action.cgi` are not registered. The validator has no dispatcher, response handling, authentication, or request execution. Future B4 must test response media/size/parser limits and must not import the device's generic helpers wholesale. A vendor GET that performs a reset remains prohibited even if its JavaScript calls it a read.

### Model-independent selection

Runtime selection uses protocol family plus qualified identity/evidence predicates; no `if model == "MG11"` or brand fallback. Model/revision values remain data in exact qualification records outside generic acquisition code. The implemented regression recursively scans runtime `.rs` under `src/acquisition` for the case-folded model/brand strings in `acquisition/fixtures/hardware_model_names.txt`; test modules and fixtures are excluded. A second test runs equal evidence through two user labels and asserts equal applicability. Add every new matrix target name to that fixture list.

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

D6's original read-only SQLite preflight was only partly accepted: it still created WAL/SHM sidecars for a WAL-mode newer database. Approved G1 replaces it with a standard-library, read-only 100-byte header read: validate `SQLite format 3\0`, decode the big-endian u32 at offset 60, reject above shared `SUPPORTED_SCHEMA_VERSION` before opening SQLite. WAL-mode and delete-mode temp regressions verify unchanged bytes/journal and no new sidecars. Short, invalid or unreadable headers fall back to existing open/error handling. The post-open version check remains for a newer version only in uncheckpointed WAL; the header-only refusal guarantee does not extend to that normal SQLite-open path. Supported schemas retain WAL/migrations; startup propagates refusal before `record_startup`. [G1 evidence](PHASE_7_5_G1_VERIFICATION.md) supersedes the earlier D6 sidecar-preservation claim. Exact schema-6 SQL, consistent backup implementation and rollback/interruption tests remain separately blocked before B2 migration/class code.

### Plain-HTTP endpoint binding (B5 preview)

Show **"not cryptographically verified"** beside any plain-HTTP identity/session. Bind the interface's stable Windows ID/LUID, local source address/prefix, selected gateway, DHCP-server address, USB-to-NIC correlation, request endpoint, evidence IDs and session generation. The placeholder `00-CC-CC-CC-CC-CC`, any other MAC or matching subnet alone is never identity. The existing discovery model does not yet expose DHCP server identity; missing information stays unknown and may block qualification, never synthesized from DNS or gateway values.

Under the reviewer's D4 policy, each accepted authenticated field retains `transport: plain_http`, its session binding and read-only permitted uses. It may inform recognition, compatibility display and read-only probing. Plain HTTP alone never grants irreversible authority; fresh same-session identity plus physical USB correlation, owner label confirmation, B5 rechecks and separate operation approval would be additional future gates. This policy is recorded here only; B5 binding and all physical write/flash/install paths remain blocked.

Recheck the entire binding and minimal approved identity fields before and after every session/read bundle. USB replug, route/interface/source/DHCP change or conflicting identity cancels and revokes; do not silently follow another gateway. Current same-subnet collision block remains until a source/interface-bound transport is independently qualified. WinHTTP behavior is not presented as strong source binding. Future socket binding needs IPv4/IPv6, route, TLS, cancellation and collision tests before use. Plain HTTP remains susceptible to interception despite these consistency checks; authenticated Digest is not a substitute for a cryptographic host identity.

### Authenticated-read proposal and device-JavaScript citations (B4 preview)

Citations refer to the device's own static `/js/...` files, with one-based line numbers and SHA-256 anchors below. They are reproducible by owner-approved re-download of those exact static files and hash comparison; no temporary directory is an evidence identity. This follow-up reviewed existing file text locally without fetching, executing JavaScript or entering credentials. No vendor scripts are newly committed. The shared helper at `/js/base/ajax_calls.js:349-364` builds `xml_action.cgi?method=get&module=duster&file=` plus the name, uses HTTP GET and `getAuthHeader("GET")`. `/js/base/utils.js:245-246` maps `getData` to that helper.

Primary identity candidate is the dashboard's `sysinfo` node: `/js/panel/dashboard.js:682-684` reads `device_name` and `version_num`; `/js/panel/router/software_upgrade.js:20-21` also reads `sysinfo/version_num`. The response filename is not `sysinfo` by inference. `/js/base/layout_manager.js:55-75` reads menu `xmlName` attributes and later passes them to `setXMLName` (`:90-91`); `:121` loads `xml/ui_` plus `g_platformName` plus `.xml`. The captured root page at line 24 sets `g_platformName = "mifi"`, so the static menu source is `/xml/ui_mifi.xml`. No menu XML body is present in the supplied local capture. The dashboard's literal `file` value therefore remains UNKNOWN. Identifying that value from the exact menu document and matching dashboard implementation is an open B4 task; no CGI filename guessing or request is allowed. Any chosen parser must emit only permitted identity/version fields and discard IMEI, SIM/subscriber and device-unique fields before output, never storing/logging the raw body.

| Exact `file` name | Device's own call site | Evidence / proposed disposition |
| --- | --- | --- |
| Dashboard literal filename UNKNOWN | `/js/panel/dashboard.js:682-684`; `/js/base/layout_manager.js:55-75,90-91,121`; root page `:24` | Primary `sysinfo` candidate. Resolve exact dashboard `xmlName` from `/xml/ui_mifi.xml` before proposing a CGI request. No placeholder filename is registered. |
| `status1` | `/js/panel/internet/internet_connection.js:1853-1854`; helper `/js/base/ajax_calls.js:355-361` | Out of the allowlist unless the primary `sysinfo` source proves insufficient and receives separate review. Carries IMEI; not the default identity target. |
| `wan` | `/js/panel/internet/pin_puk.js:18`, `callProductXML("wan")`; same GET helper | Documented name, deferred from initial identity allowlist: SIM/APN metadata is unnecessary and may be sensitive. |
| `Engineer_parameter` | `/js/panel/dashboard.js:48`, `getData("Engineer_parameter")`; `/js/base/utils.js:245-246`, then same GET helper | Documented name, excluded from initial allowlist pending semantic review of engineering data. No engineering/control request is proposed. |

No other file name is proposed, inferred from the vendor, or borrowed from another device. The software-upgrade page uses a variable `xmlName`; its `version_num` display does not identify a safe literal filename. Do not invent `device_info`/`version` endpoints or use `status1` as fallback without review. Static `/version_num.txt` remains the existing unauthenticated observed-only path and is not a CGI filename.

Capture SHA-256 anchors: `ajax_calls.js` `6432420bc1aefcb7990b7180664bf0be720e434ae952093f1f6a1fde9fd524e1`; `utils.js` `862497a77554c3d67da5fbed611bf13d9df9bc126bf2a7af882c997cb89c6f68`; `internet_connection.js` `84f54df2fd9e91ec34fe9a43c1f7d368d0776367244d39293c9b8df4364e3dee`; `pin_puk.js` `e05c11c3e1520f46c280018a53b9ab4290d2201110bd7893fa78dc45899643ac`; `dashboard.js` `f57e620d871ec877920d1a04cef4a7e41eb2db0af20c77bd457b967c40ae381d`; `layout_manager.js` `9c8d5fd1fc47111180b80ceee1261cc38aa560fc1d53b8e03e286cae6fc3664c`. Captured text is evidence for design, not executable code to vendor into the app. Full root-page and retained-prefix hashes are preserved in the R1-R5 report; line 24 also exists in the committed prefix fixture.

### Full future authentication flow and separate request specs (D2)

All entries in this table are design-only and blocked from implementation/dispatch in this stage. Only `/` and `/version_num.txt` may be registered by the future local B3 skeleton after confirmation. The device helper flow uses wire GETs but establishes and destroys session state; it is not a read-only GET-only operation or a single Digest retry.

| Proposed named spec | Exact operation / closed parameters | Device citation and distinct risk |
| --- | --- | --- |
| `SessionChallengeRead` | GET `/login.cgi`, no query; read bounded `WWW-Authenticate` header | `/js/base/utils.js:27-37`; `/js/base/ajax_calls.js:536-549`. Parse realm, nonce, qop; no arbitrary challenge-selected path/host. |
| `SessionDigestLogin` | GET `/login.cgi?Action=Digest&username=...&realm=...&nonce=...&response=...&qop=...&cnonce=...&temp=asr` | `/js/base/utils.js:68`. Explicit session-state allowlist entry, not an identity read. May displace the owner's browser session; failed/repeated attempts may lock the account. Hashed credentials and username in the query can be logged by the device; treat the entire URL as secret. |
| `SessionIdentityRead` | GET `/xml_action.cgi?method=get&module=duster&file=<reviewed literal>` with Digest Authorization header | `/js/base/ajax_calls.js:349-364`; `/js/base/utils.js:133-162`. No literal allowed until D3 resolves it. Per-read signing uses fixed URI `/cgi/xml_action.cgi` (`utils.js:142,159`), the device's actual coded behavior to confirm during supervised challenge; never try alternative URIs. |
| `SessionLogout` | GET `/xml_action.cgi?Action=logout`, no caller parameters | `/js/base/ajax_calls.js:258-269`. Explicit session-state entry requiring its own approved cleanup scope; may affect shared browser/session state. Logout is not inferred from being absent in the deny list. |

Login's separate digest calculation signs `/cgi/protected.cgi` (`utils.js:56,70`); data-read signing uses `/cgi/xml_action.cgi`. Preserve and confirm these device-defined roles under request-specific review rather than constructing guesses. Credentials and derived digests use only the existing `Secret` path; no logging of URLs/query strings, exceptions with raw request targets, Authorization headers, challenge bodies or raw response bodies. No credential caching/defaults/derivation from IMEI; no copying the device page's browser-cookie/session automation.

The same CGI family includes `login.cgi?Action=Download&file=...` and `xml_action.cgi?Action=Upload...` (`/js/base/utils.js:543-548`). Both are denied, as are all eleven prohibited substrings. Their presence is not export support and never authorizes a firmware or configuration transfer. Login/logout need named request IDs, exact key sets, purpose `session_state`, separate risk-aware user consent, bounded attempt counts and a future approved grant. The generic read registry cannot create either from arbitrary strings or allow them merely because their action is not deny-listed.

Future budget model: one challenge, one explicit login attempt, one reviewed identity read and a separately bounded approved logout; per-stage bounds and overall bound are enforced together. Authentication failure, lockout, 5xx, binding change or cancellation ends the sequence, with no automatic retry/login loop. Cleanup logout is allowed only if its exact operation, current session and unexpired grant were explicitly approved; otherwise discard local Secret/session authority, report cleanup unconfirmed, and send no fallback request. This risk/cleanup policy still requires B4 request-specific approval and owner supervision.

First authenticated read procedure, after design and request-specific review:

1. Resolve the dashboard literal from the exact static menu document, review response fields and the complete challenge/login/read/logout specs. Stop before credentials if any required file name or semantics remain unknown. Current stage sends no request to either CGI.
2. Owner powers/connects target, confirms endpoint/interface, and reviews browser-session displacement, lockout, hashed query-secret logging and plain-HTTP risks. Native approval names each allowed operation, including login and logout; wire GET is not described as absence of session changes. Owner supplies credentials at runtime through `adapters/lifecycle.rs::Secret` only.
3. After separate B4 approval, owner supervises the challenge, one explicit Digest-login attempt and one exact identity read. Confirm fixed device-defined signing URIs against the real challenge/code; do not try alternatives. No guessed credentials, credential derivation, path discovery, automatic relogin or blind retry.
4. Stop on any sign of an unapproved write/state action, any 5xx, lockout/unauthorized text, redirect, unexpected content or changed binding. Erase Secret and revoke authority. Approved login/logout are explicitly session-state actions; no other state action is permitted. Do not silently send logout after grant expiry/cancellation; use only the separately approved cleanup scope while valid.
5. Successful parsing emits only qualified per-field evidence with `transport: plain_http`, permitted read-only uses and session binding. Discard unique/subscriber fields before output; never store/log the raw body. Recheck identity/binding before publication. No extraction/export or internal-storage preservation claim follows. Record any owner observation verbatim with date/source and web-display-only scope.

### Risks and open questions for review

- Which exact dashboard `xmlName` is in `/xml/ui_mifi.xml`? Static source path is identified, but menu contents are not supplied locally. No requests to guess CGI names; `status1` remains excluded unless the primary `sysinfo` source is insufficient and separately reviewed.
- Do `sysinfo/device_name` and `version_num` suffice for the required exact identity? Manufacturer/hardware revision may still be absent. No inference from `REL_MRVL...`, `MF69...` or `LT800`.
- Future owner-supervised authentication must confirm device-defined signing of `/cgi/protected.cgi` for login and `/cgi/xml_action.cgi` for reads. No alternate-URI probing; browser displacement, lockout and device-side query logging require explicit risk acceptance and request-specific review.
- D4 policy is decided: plain-HTTP per-field evidence supports recognition/display/read-only probes, never irreversible authority alone. Stronger B5 checks, fresh same-session identity, USB correlation and label confirmation remain future independent gates.
- How will DHCP-server evidence be exposed and what exact source-binding transport will be qualified? Current missing field/collision guard remains authoritative until B5 review/testing.
- The proposed schema-6 neutral envelope, exact SQL, backup and rollback tests require separate B2 review. G1 raw-header preflight corrects D6's WAL sidecar case; that approval does not authorize schema-6 migration. Do not contaminate firmware history with configuration/report records.
- Registry composition, qualification records and trusted publisher signing need separate reviewed persistence/version rules. No dynamic plugins, official-image URLs or acquisition executor ship from this proposal.

The approved local B1/B3/B6/model-scan items are implemented and recorded in [PHASE_7_5_B1_IMPLEMENTATION.md](PHASE_7_5_B1_IMPLEMENTATION.md). No strategy-probe approval is issued in production; every registered strategy returns `not_implemented`, plans remain nonexecutable, and `canObtain` remains false. Eleven compatibility stages are additive to the existing schema-1 report fields. B2/B4/B5/B7/B8 stay blocked under their separate reviews; all extraction, physical writes, flashing and live USSD stay disabled.

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
