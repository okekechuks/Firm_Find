# Phase 7.5 Stage 2A: authenticated identity-read design

23 October 2026, Africa/Lagos. **Design accepted; only credential-free Challenge software is released. Hardware remains held for separate acceptance.**
Branch `phase-7-5-stage-2a-auth-design` starts at
`5df1aaceb4e4f8e352e5c7280edcae2b42635966`, from the accepted, unmerged Stage 1
branch. That initial delivery changed this document only. The later released
Part 2 software is tracked on `phase-7-5-stage-2a-challenge` in
[its verification report](PHASE_7_5_STAGE_2A_CHALLENGE_VERIFICATION.md).
No credential entry or hardware request is authorized by this document.

Stage 1 is accepted: the supervised static-menu result was `observed`, filename
`status1`, unverified. That observation supplies no request authority. The
reviewer's checks on a clean `5df1aac` copy were ten standard Rust runs (280
passed each plus two doctests), 281 isolated-profile tests, clean Clippy in both
profiles and formatting, 151 frontend tests, TypeScript and eslint. These are
reviewer-reported results, not new executions in this documentation task. The
reviewer did not independently run the builds, GUI or hardware sequence.

## Source register and verification boundary

Device-script names below identify the captured sources, not requests made for
this task. No temporary capture-directory location is a source of authority.
Only static local bytes were read; JavaScript was not executed.

| Source | SHA-256 anchor | Verification in this task |
| --- | --- | --- |
| `/js/base/ajax_calls.js` | `6432420bc1aefcb7990b7180664bf0be720e434ae952093f1f6a1fde9fd524e1` | Local capture hashed and cited lines re-read. |
| `/js/base/utils.js` | `862497a77554c3d67da5fbed611bf13d9df9bc126bf2a7af882c997cb89c6f68` | Local capture hashed and cited lines re-read. |
| `/js/base/layout_manager.js` | `9c8d5fd1fc47111180b80ceee1261cc38aa560fc1d53b8e03e286cae6fc3664c` | Local capture hashed; menu mapping at lines 56–75 re-read. |
| `/js/panel/dashboard.js` | `f57e620d871ec877920d1a04cef4a7e41eb2db0af20c77bd457b967c40ae381d` | Reviewer re-downloaded the static capture and confirmed hash and lines 682–684. Lines 672–678 also read Wi-Fi mode and ssid from the same document. |
| `/js/panel/internet/internet_connection.js` | `84f54df2fd9e91ec34fe9a43c1f7d368d0776367244d39293c9b8df4364e3dee` | Reviewer re-downloaded the static capture and confirmed hash and lines 1853–1854 reading status1 and IMEI. |

The last two capture verifications are attributed to the reviewer, not a new
Codex download. They confirm the identity fields, IMEI risk and Wi-Fi fields.
This task does not download them.
[Stage 1 evidence](PHASE_7_5_B4_HARDWARE_STEP.md) supplies the actual menu result.

The original [B1 design](FIRMWARE_ACQUISITION_ENGINE.md) remains historical where
it calls the dashboard filename unknown or excludes `status1` pending menu
resolution. Stage 1 and this request-specific proposal supersede that uncertainty
only. They do not register or authorize an authenticated request.

## 1. Closed request specifications and validator behavior

These are four **proposed** family entries, never an arbitrary CGI facility.
The current `apps/desktop/src-tauri/src/acquisition/request_registry.rs` still
registers exactly the three accepted static reads. All four entries below are
currently unregistered and must fail. The existing `Login`/`Logout` enum names
are not registered capabilities. No code changes those facts here.

All proposed attempts are one-shot, no redirects, no bodies, no proxy, no
automatic credentials, no cookies, no automatic authentication negotiation,
no browser session reuse and no retries. The broker constructs every path and
query from private typed specs. The caller selects only an already validated
device/interface/normalized numeric endpoint, never a path, query or file name.
Limits below are conservative design choices, not observed device sizes/timing.

| Named spec | Exact wire request | Response cap and parser | Attempts / network budget | Device-script citation |
| --- | --- | --- | --- | --- |
| `Challenge` | `GET /login.cgi`, no query, credentials, Authorization or Cookie | Expected HTTP 401; `WWW-Authenticate` total at most 2 KiB, response headers total 8 KiB. Header-only Digest parser. Do not collect response body; close after headers. | 1 / 5 s, including route preflight, connect and headers | `utils.js:27–37`; `ajax_calls.js:534–550` explicitly GETs the login URL and reads this header. The `devEnv` return at 535–536 is a stub, not a real challenge. |
| `Login` | `GET /login.cgi?Action=Digest&username={u}&realm={r}&nonce={n}&response={h}&qop=auth&cnonce={c}&temp=asr` | HTTP 200 **and** bounded success parser; 4 KiB UTF-8 `text/html` or `text/plain`. Proposed conservative success is exact `200 OK` after surrounding ASCII whitespace, not an arbitrary substring. Actual success envelope remains unqualified. | 1 / 5 s | `utils.js:55–70`, `login_done:12–18`; `ajax_calls.js:516–531` supplies the request helper and an additional Digest header, discussed below. |
| `StatusRead` | `GET /xml_action.cgi?method=get&module=duster&file=status1`, one explicit Digest Authorization header | HTTP 200, `application/xml` or `text/xml`, bounded UTF-8 XML parser from section 5; 32 KiB, headers 8 KiB. No truncation accepted. | 1 / 5 s | `ajax_calls.js:349–366`; `utils.js:133–162`; reviewer-attributed `dashboard.js:682–684` and `internet_connection.js:1853–1854`; Stage 1 selected `status1`. |
| `Logout` | `GET /xml_action.cgi?Action=logout`, no other query parameters, credentials, Cookie or Authorization header | Headers 8 KiB; at most 1 KiB transient `text/html`/`text/plain` acknowledgment if present. Record bounded HTTP outcome, discard content. JavaScript does not establish a success-body contract; HTTP 200 alone is not proof the remote session ended. | At most 1 / separately reserved 3 s | `ajax_calls.js:258–269`; it uses GET with `dataType: html`, has no `beforeSend` Authorization assignment, and clears local browser state in `complete` regardless of server success. |

Parameters mean private backend values, not interpolated caller strings. Exact
key order and spelling are canonical; reject duplicate/extra keys, empty values,
alternate case, fragments, userinfo, alternate hosts/ports/schemes, trailing
segments, traversal, backslashes, encoded separators, double encoding, alternate
file names and any redirect. Existing static validators stay unchanged.

The future auth validator would accept only a typed request ID and typed values,
validate them, and compare against the one regenerated canonical template.
`Action=Digest` is an exception only on `Login`; `Action=logout` only on `Logout`.
`StatusRead` has exactly `method=get`, `module=duster`, `file=status1`; no `Action`.
`Challenge` has no query at all. Thus the two session-state exceptions are
`Login` and `Logout`, not a generic permission for `Action` or `login.cgi`.
Registry spelling or HTTP GET alone never proves absence of session effects.

Keep the complete case-insensitive deny list:
`set`, `upload`, `upgrade`, `reboot`, `restore`, `reset`, `factory`, `download`,
`delete`, `format`, `flash`. `utils.js:541–548` demonstrates dangerous Upload and
Download variants in the same CGI family; none is allowed. Scan the complete
validated decoded path/query, including parameter values. A legitimate-looking
username or realm that contains a denied substring remains conservatively
`REQUEST_DENIED`; do not exempt secret values or echo them to explain rejection.

| Spec | Proposed validation path; no blanket bypass |
| --- | --- |
| `Challenge` | Exact `/login.cgi`, empty query, no denied word, matching `AuthChallenge` proof. Path name does not authorize credential use. |
| `Login` | Exact path plus only the fixed Digest template, bounded typed values and `AuthenticatedIdentityRead` proof. Constants contain no deny word; dynamic values must also pass the deny check. |
| `StatusRead` | Exact fixed get/duster/status1 template; no caller choice. Matching identity-read proof and confirmed same session required. No generic `XmlAction` registration. |
| `Logout` | Exact logout template and separately valid cleanup proof; not exempt from the deny list. No `Action=Download`, Upload or alternate action. |

Proposed initial data alphabets: username and realm `[A-Za-z0-9_.-]{1,64}`;
nonce `[A-Fa-f0-9]{1,128}`; qop exactly `auth`; response exactly 32 lowercase hex;
cnonce exactly 16 lowercase hex generated using system cryptographic randomness.
These are restrictive proposed formats, not facts learned from the stub nonce.
Any real challenge outside them stops for review, rather than widening parsing
or trying another authentication scheme. No password is placed in the query.
An auth-specific secret builder must not pass the assembled secret-bearing URL
through ordinary `url::Url` cloning/Debug/error paths in the metadata validator.
Values are checked while borrowed from guarded buffers; the canonical wire URL
is assembled only into an erasable transport buffer, never exposed as a report.

### Source finding: two signing URIs and an additional login header

The captured code establishes these distinct calculations:

- Query login: HA1 is MD5 of `username:realm:password`; HA2 signs
  `GET:/cgi/protected.cgi`; response uses the challenge nonce, `00000001`, login
  cnonce and qop (`utils.js:55–68`). Neither signing URI is the actual wire path.
- `doLogin` then calls `authentication(url)` (`utils.js:69`). That helper adds
  `Authorization` using `getAuthHeader("GET")` (`ajax_calls.js:516–531`, especially
  525–526), which signs `GET:/cgi/xml_action.cgi` (`utils.js:142,159`). The login
  may therefore carry **both** its protected.cgi-signed query and a separate
  xml_action.cgi-signed Digest header, with a separately generated cnonce.
- `GnCount` starts at 1 (`utils.js:5`); header construction formats it, then
  increments it (`utils.js:152–159`). If that login header is retained, it consumes
  `nc=00000001`, so the first subsequent data header uses `nc=00000002`.
  The query login still uses its own fixed `00000001`.

This is a locally reverified detail beyond the guideline's abbreviated flow.
Do not silently omit the login header, reset the data counter, sign the wire URI,
or probe alternatives. The provisional design follows those captured roles;
the reviewer must explicitly resolve this finding before authenticated code is
released. Part 2 can observe a challenge only; it cannot prove either login
signing convention or nonce-count behavior works on hardware.

## 2. Approval scopes, preview and exact native text

The guideline says “three separate scopes” but names only `AuthChallenge` and
`AuthenticatedIdentityRead`. Proposed resolution for review: these are two
owner-issued scopes; a third, `CleanupLogout`, is a one-use **derived cleanup
proof**, covered explicitly by the final identity-read dialog. It is not a third
independently issuable grant, and grants no login/read authority. If three
independent owner confirmations were intended, that must be specified before
implementation; this proposal does not assume permission to add one.

Each proof is non-Clone, non-serializable, privately minted, single-use and bound
to device ID, discovery revision/generation, selected interface, normalized
numeric endpoint (scheme/host/port), binding snapshot digest, request IDs and
expiry. Claim only after the scoped busy/RAII reservation succeeds. Failed
scope/target/replay claims consume their receipt; a busy rejection consumes none.
No frontend boolean, root/static-version/menu receipt, strategy grant or existing
adapter session can satisfy any of these new scopes, or vice versa.

Proposed preview/approval lifetime is at most five minutes, with credential entry
at most 120 s within that lifetime. Revalidate the whole selection after every
human step. No network deadline runs while the owner reads a dialog or enters
credentials. Actual execution gets fresh deadlines only after credentials are
accepted and the binding is rechecked. This does not change existing 30 s
metadata receipts. Each I/O still checks proof validity before sending.

For a future complete flow, the owner would approve `AuthChallenge`, then the
final identity bundle, then enter credentials through a native prompt. Defer
both dispatches until entry completes: one fresh challenge, one login, one
status read, one cleanup. A standalone Part 2 operation instead dispatches only
its approved challenge and discards nonce immediately. Its receipt/result can
never be upgraded into an identity-read approval or reusable challenge cache.

All substitutions below are backend values, sanitized to single-line, bounded
text with controls/bidi overrides removed; no nonce, credential, hash or raw
response enters confirmation text. Native defaults are No; cancellation grants
nothing. No automated Yes or caller-supplied approval is accepted.

**AuthChallenge title:** `OBTAINER - approve authentication challenge`

```text
Approve one credential-free authentication challenge check?

Device: {device_name}
Interface: {interface_id}
Endpoint: {normalized_endpoint}
Request: GET /login.cgi, one attempt, maximum 5 seconds.

No username, password, Authorization header or cookies will be sent. Only a bounded realm and qop observation may be reported; the nonce and response body will not be kept. Plain HTTP is not cryptographically verified. This does not authorize login, identity reads, logout or firmware access. Continue only if you own or administer this endpoint.
```

**AuthenticatedIdentityRead title:** `OBTAINER - approve authenticated identity read`

```text
Approve one authenticated device-name and software-version read?

Device: {device_name}
Interface: {interface_id}
Endpoint: {normalized_endpoint}
Requests: one separately approved GET /login.cgi challenge; one Digest login at /login.cgi; one GET /xml_action.cgi?method=get&module=duster&file=status1; at most one GET /xml_action.cgi?Action=logout cleanup.

This login may log out your existing browser session on the device. Failed logins may lock the account; the app will not retry. Hashed credential material is sent in a GET query string over plain HTTP and may be recorded by the device. Plain HTTP is not cryptographically verified.

The status response may contain IMEI and subscriber information. Only device_name and version_num will leave the parser; the response and all other fields will be discarded. No firmware transfer, settings write, upgrade, reboot, restore, reset or USSD is authorized.

Cleanup logout is included on success, cancellation, timeout or error if the original endpoint binding remains safe. It has one separate attempt and at most 3 seconds; it may also affect a browser session. Failed or unsafe cleanup will be reported as unconfirmed. Credentials are entered only after this confirmation, used for this operation and cleared afterward.
```

**CleanupLogout:** no independent issuance or hidden consent. Its exact native
consent text is the final paragraph of the identity dialog above, displayed
before credentials. Its proof binds the parent operation/session and exactly
`Logout`, with one reserved attempt and a deadline independent of the cancelled
read. Reusing the parent grant cannot mint a second child. No other request can
claim it. This third-scope interpretation is an explicit review question.

The native credential prompt repeats device/interface/endpoint and operation;
its title is `OBTAINER - credentials for this identity read`. Cancel erases its
buffers and performs no login. There is no save/remember/default option.

## 3. Credentials, digest handling and erasure

The existing path is `apps/desktop/src-tauri/src/adapters/lifecycle.rs::Secret`:
non-serializable, non-Clone, redacted Debug, 1–4096 bytes, volatile clearing plus
a compiler fence on Drop. It is a storage primitive, not an existing safe native
password-entry flow. No new credential UI or IPC is implemented in Part 1.

Proposed entry is a backend-owned native username/password dialog after the
final scope/target display, never React form state, JavaScript strings, ordinary
Tauri invoke arguments, clipboard automation, command-line arguments, environment
variables, files, browser storage, SQLite or a credential cache. Username is
sensitive too. Native UTF-16 edit/result buffers are guarded, converted directly
to owned `Secret` byte buffers, cleared and destroyed. Password bound: 1–1024
UTF-8 bytes, no NUL; no guessed/default value or derivation from IMEI, SIM,
device name, address, USB ID or another field. An empty/cancelled entry ends
before dispatch. Charset compatibility remains a review question.

Password, HA1, intermediate MD5 inputs, digests, nonce, both cnonces, login query,
and Authorization header buffers are secret-bearing. Use `Secret` for bounded
byte values and an equivalently guarded native UTF-16 transport scratch buffer;
avoid ordinary immutable `String`, `format!`, derived Debug, URL cloning and
error formatting that copy the query. Wipe native source buffers as well as final
Secret values; never log a whole request/response/header, username, password,
nonce, digest or URI containing those values. Map transport errors from bounded
numeric/status categories, never a library error that includes a URL.

Erasure runs on success, failed validation, failed login, lockout, cancellation,
timeout, binding conflict, parser failure, panic unwinding and shutdown. A local
scope guard owns each sensitive allocation; a scoped operation guard owns only
its own busy flag. The cleanup request, as captured, has no Authorization header
and needs no retained password; erase authentication material when the active
read ends, then attempt its separately approved logout. Do not copy the device
script's persistent globals, cookie helpers or ten-minute timer into the app.

This promises clearing application-owned buffers, not guaranteed erasure of OS,
WinHTTP, native control or crash-dump copies, nor recovery after forced process
termination. Review the native transport buffer/copy behavior before any release.
The server still sees hashed material in the login GET query and may log it;
client-side redaction cannot remove server logs. No claim of encrypted transport
or secure password storage follows from use of Digest/MD5.

## 4. Risks and consent boundaries

Login/logout change authentication state even though their wire verb is GET.
The owner's existing browser session may be displaced; failed logins may trigger
lockout, and the vendor's limits are unknown. Stop after one failed attempt or
lockout indication. The hashed GET query may remain in device/server logs.
Plain HTTP provides no cryptographic endpoint identity, confidentiality or
response integrity; route checks and Digest do not turn it into TLS.

These risks appear in the final native text, not only documentation or an info
tooltip. Ownership consent and endpoint consistency are necessary for this
bounded read; weak family hints, a successful HTTP response, a menu filename,
user labels or USB IDs alone cannot create authenticated evidence or general
adapter support. No automatic credential prompt follows detection or Stage 1.

Any Set-Cookie indication is a conservative stop for review, retaining at most
a boolean presence flag and never its value. Presence alone does not prove a
cookie dependency. A read failing without cookies likewise does not justify
enabling them, borrowing browser cookies or an experimental second request.

## 5. Strict privacy-preserving status parser

Parse only a complete HTTP 200 XML response, with `application/xml` or `text/xml`
and UTF-8, within 32 KiB and the read deadline. Reject unexpected encoding,
content type, compression/expansion outside the wire budget, Content-Length
mismatch, truncation or oversized content without parsing a prefix as success.
Never follow redirects or fetch external resources.

Reuse the conservative rules of
`apps/desktop/src-tauri/src/acquisition/static_menu.rs`: no DTD, ENTITY
declarations, entity references, CDATA, namespaces/prefixed names/attributes,
processing instructions other than the narrowly accepted initial XML declaration,
invalid/control characters, malformed tags, duplicate attributes or multiple
roots. Scan/reject declaration/entity tokens across chunk boundaries before
expansion could occur; never enable an entity resolver. Validate the full
document, including ignored subtrees, before returning anything.

Use a private bounded erasable response scratch buffer and event parser, not a
general DOM, JSON/XML object tree or persistent response cache. Bound depth to
16, names to 64 bytes, attributes to 16 per element and values to 256 bytes.
No unknown field value is copied into an output object, logs, trace arguments,
metrics, diagnostics, parser exceptions or UI. Erase scratch/event buffers on
every exit; the existing credential `Secret` size limit is not increased to hold
the response. A separate private response-buffer guard would require review.

Require exactly one case-exact `sysinfo` in the whole document and exactly one
direct text-only `device_name` and `version_num` child of it. Duplicate/missing
sysinfo or required fields, mixed content, nested target-field markup or invalid
tokens fail the entire pair with no partial publication. The root wrapper is
not inferred from `/version_num.txt`; its exact status envelope is unobserved.
The proposed unique-sysinfo selector needs review against retained source/fixtures.

Output tokens, preserved exactly without case/date/version normalization:

- `device_name`: 1–64 ASCII bytes; first character alphanumeric, remaining
  characters alphanumeric, ASCII space, `.`, `_`, `(`, `)`, `/`, `+`, `-`;
  reject leading/trailing whitespace. This permits a short display name, not
  proof it is a hardware model identifier.
- `version_num`: 1–128 ASCII bytes from letters, digits, `.`, `_`, `/`, `-`,
  starting alphanumeric. It remains an opaque vendor string; no numeric/date
  ordering or firmware-image inference.

The status1 document contains Wi-Fi configuration as well as IMEI. Wi-Fi `ssid`
and `mode`, IMEI, IMSI, ICCID, SIM state, subscriber IDs, MAC/serial numbers, network/account
fields and **all** other values are discarded inside the parser. Even a nested
unknown field inside sysinfo gets no output slot. Only the two explicitly
allowed strings can leave successful parsing. A malicious server could put
identifier text in an allowed field; the whitelist is not semantic proof that
arbitrary values are non-sensitive or represent a model. Field qualification and
host-trust limitations must remain visible; do not claim a general privacy proof
for a counterfeit endpoint.

Login parser is separate: non-200, 401/403, known lockout indication, malformed,
oversized or unexpected success content never reaches `StatusRead`. Source
`login_done` uses a substring; this design intentionally fails closed until a
reviewed success contract is supplied, rather than accepting “not 200 OK” as
successful authentication. Never retain failure/lockout body text in a report.
For a future HTTP-200 login whose body differs from trimmed `200 OK`, stop as
`AUTHENTICATION_FAILED`. Record only H1-sanitized media type, length bucket
`0`, `1-16`, `17-256`, `257-4096` or `over`, and a boolean for containing `200 OK`.
No body text is retained and the parser is never relaxed automatically.

## 6. Per-field evidence, source separation and permitted uses

A future successful, qualified response would yield only two typed assertions:
field `sysinfo/device_name` and field `sysinfo/version_num`, each with
`source: authenticated_management`, `verification: verified`,
`reliability: strong`, **`transport: plain_http`**, collection/expiry times and
the exact session/device/endpoint/interface/binding references. Verification
means the field was returned through the approved authenticated protocol; it
does not mean cryptographic host verification or independent hardware truth.

Publish neither field until both parse successfully, authentication is accepted,
postflight binding/identity checks pass and no conflict exists. Plain-HTTP policy
D4 allows recognition, display and separately approved read-only probes only.
No write, firmware extraction permission, flash, installation, USSD or irreversible
authority follows. Completed-session evidence may be displayed historically with
its timestamp, but cannot serve as a live session/read authorization after logout.

No unreturned manufacturer, hardware revision, chipset, hardware model, firmware
image identity or feature support is inferred. `device_name` may be a configurable
name; it is not unconditionally mapped to `Attribute::Model` or physical identity.
`version_num` reports that named vendor field; its semantics must be qualified
before mapping it to installed-firmware evidence used by profiles.

Current `profiles/engine.rs::EvidenceRecord` has no transport/session fields;
`profiles/mod.rs::Attribute` has no dedicated device-name attribute. Proposed
integration is an expiring typed per-field envelope beside the operation result,
with mandatory transport and permitted-use tags checked by the central verifier.
Do not strip these qualifiers to fit an old record or abuse a free-text reference
to grant generic trusted support. The exact safe mapping/consumer contract needs
review before authenticated code; no SQLite/schema-6 change or trusted row is
created by this design. Existing stored observations stay untouched.

| Value/source shown separately | State | Interpretation |
| --- | --- | --- |
| Owner's web display: `MF69_AM802L_ZX_DD_N_241106_EU` | User-reported, unverified | Preserve actual source label; never upgrade by textual agreement. |
| `/version_num.txt`: `REL_MRVL_UAPXC_v2.6.0` | Endpoint-observed, unverified | Existing static observation; not established as the same software component. |
| Future `sysinfo/device_name`: no value collected | No authenticated result yet | Never prefill with MG11, LT800, Windows name or user label. |
| Future `sysinfo/version_num`: no value collected | No authenticated result yet | Compare raw strings side by side only after an approved read. |

Different source values are not silently merged, preferred or “corrected”. A
cross-component difference remains an unresolved source discrepancy unless
field semantics are independently established. Conflicting same-field trusted
identity evidence invalidates authority; no guess resolves it. No fabricated
“verified” example value is seeded into a cold-start fixture.

## 7. Session, before/after checks, cancellation and cleanup

Use the existing adapter/session lifecycle with a future source-specific
read-only policy; do not rebuild discovery, profile storage or the session manager.
An initially unverified device requires exact owner approval for bootstrap;
it cannot be required to carry preseeded authenticated evidence. Its bootstrap
grant permits only the fixed handshake/read bundle, never general reads.

Re-read the complete **local binding and current identity evidence** before
each dispatch and after each response and before publication: gateway, resolved
numeric destination, route/interface, source address/prefix, revision, USB
correlation and DHCP server when exposed. Unavailable DHCP-server information
is explicitly unknown, not guessed from DNS/gateway; retain the existing
same-subnet collision/protected-host blocks. This is a proposed consistency
policy, not an implemented or cryptographically strong B5 binding.

On any change, ambiguous/protected target, stale selection or identity conflict,
cancel active I/O, revoke active grants/session, erase candidate output and
prevent stale successful publication. An identical periodic discovery snapshot
alone must not invalidate a session. Source checks must not silently select a
new interface, address, gateway or replacement USB device.

**Remote identity-read count needs reviewer resolution.** The guideline requires
identity re-reads “before and after” but also permits exactly one `StatusRead`.
One authenticated response cannot independently demonstrate two remote identity
snapshots. Local preflight evidence is not mislabelled as a remote authenticated
read. The proposed bundle sends at most one status request and gives no
before/after-remote-identity guarantee. Any operation requiring two fresh remote
snapshots remains blocked until the reviewer explicitly authorizes a second
scoped read and budget, or narrows that requirement for this bootstrap. Future
protected operations still require fresh same-session identity before/after;
this identity-only proposal cannot authorize them.

No automatic reconnect, relogin, nonce refresh, alternate Digest URI, retry after
401/lockout/5xx/redirect/identity mismatch, fallback file, engineering endpoint,
cookie enablement or default credentials. A fresh challenge may follow only a
new owner approval in a separately released operation. Each session is exclusive
to this operation; no browser/cross-device/session token sharing or resumption.

Network execution has at most 5 s challenge + 5 s login + 5 s status + 3 s cleanup,
18 s total after the human steps; each stage also enforces the remaining overall
budget. The identity execution deadline starts freshly after the challenge,
not when an owner first saw a fingerprint. No password prompt occurs inside a
network timeout. No returned device/JavaScript ten-minute timer extends a grant.

Every exit enters cleanup exactly once, including cancel/timeout/error and
validation failure. A reserved independent cleanup cancellation/deadline permits
one best-effort `Logout` even after active-read cancellation or timeout; that
authority was explicitly included in the final native dialog. Never block on
network I/O inside Drop; use explicit bounded cleanup and RAII for local release.

“Always attempt logout” cannot mean sending to a substituted endpoint or issuing
session-state requests before any login attempt. Cleanup dispatch requires a
claimed parent/child, an unexpired cleanup deadline, a login that may have reached
the device and the unchanged original safe binding. Before login, no remote
session was created by this operation and no logout is sent. After login with
lost binding/connectivity or expired cleanup authority, enter cleanup, suppress
unsafe dispatch, erase all local authority and report logout unconfirmed. Do not
claim remote logout succeeded. This constrained interpretation needs review.

Logout receives no retries and never triggers reauthentication, cookies or a
second status request to check whether logout worked. Return primary outcome
plus a bounded `logout_unconfirmed`/cleanup limitation if cleanup fails or cannot
be sent safely. A successful identity read can be reported with that limitation,
but the local session is closed and cannot authorize subsequent work. Busy and
resource guards return only resources owned by this operation on every path.

## 8. Stop conditions and bounded error model

Use the B1 design's proposed codes below, mapped onto existing adapter errors;
the current `strategy_api.rs::StrategyError` has only scope/strategy mismatch and
is not falsely described as implementing this whole model. Reports contain
fixed codes/counts and optional bounded limitations, never raw remote text,
headers, nonce, credentials, hashes, request URLs or discarded identity fields.

| Stop condition | B1 code | Behavior / bounded detail |
| --- | --- | --- |
| Missing consent, No or cancelled credential entry | `APPROVAL_REQUIRED` / `CANCELLED` | No active request; no result fabricated; no receipt promotion. |
| Wrong scope/endpoint/interface/device or receipt reuse | `SCOPE_MISMATCH` / `ALREADY_CLAIMED` | Deny; no transport. |
| Expired preview/receipt | `APPROVAL_EXPIRED` | Erase/release; require fresh explicit preview/consent. |
| Exclusive resource already held | `BUSY` | No claim consumed; do not cancel another operation. |
| Protected, ambiguous or untrusted target | `PROTECTED_TARGET` / `AMBIGUOUS_TARGET` / `ENDPOINT_UNTRUSTED` | No credentials/dispatch. Plain-HTTP scope never upgrades host cryptographic trust. |
| Route/source/gateway/interface/DHCP/USB change | `ENDPOINT_CHANGED` | Revoke; no stale publication or unsafe cleanup destination. |
| Stale menu/family/evidence or generation | `STALE_EVIDENCE` | Re-evaluate; no automatic new request. |
| Conflicting trusted identity | `IDENTITY_CONFLICT` | Invalidate session and candidate output; no retry. |
| Challenge not 401, absent/multiple/malformed Digest headers, unsupported qop/algorithm/charset | `INVALID_RESPONSE` | Bounded detail `challenge_not_usable`; no login. Never use the dev stub nonce. |
| Login HTTP not 200 or unqualified success body; read 401/403 | `AUTHENTICATION_FAILED` | Stop active bundle, zero credentials; only already approved safe cleanup may follow. |
| Lockout indication | `LOCKOUT_DETECTED` | Stop; no retry, defaults, alternate accounts or delay-loop. |
| Any redirect or forbidden path/query/action | `REQUEST_DENIED` | Do not follow or regenerate a different request. |
| Any 5xx | `REMOTE_5XX` | Stop; no retry. |
| Body/header too large, truncation or expansion limit | `RESPONSE_TOO_LARGE` | No prefix parsing/partial evidence. |
| Wrong type, malformed XML, DTD/entities/CDATA/namespaces, duplicate/missing/invalid target fields | `INVALID_RESPONSE` | Fixed parser/type detail only; never raw failure payload. |
| Cookie dependency | `REQUIREMENTS_MISSING` | Detail `cookie_dependency_unreviewed`; leave cookies disabled, no workaround. |
| Cancellation or deadline | `CANCELLED` / `TIMEOUT` | Interrupt active I/O, attempt separately scoped safe cleanup, zero buffers, release owned guard. |
| Logout failed, no trustworthy acknowledgment or unsafe/expired cleanup | Primary outcome retained; cleanup limitation | `logout_unconfirmed`; no remote-success claim or second attempt. |
| Auth code/semantics/binding qualification not released | `NOT_IMPLEMENTED` / `REQUIREMENTS_MISSING` | Neutral gate, no network or firmware incompatibility claim. |

A 401 is expected only for `Challenge`. Treating it as expected there does not
make a later login/read 401 successful. Error presentation never offers an
automatic “retry login”. Old scopes stay distinct from new scopes on failure.

## 9. Future deterministic test and release plan

No tests or simulator code are added in this documentation delivery. The plan
below is required before a later authenticated implementation is released.
Use labelled synthetic fixtures and owned loopback peers; no real credentials
or device-unique values. Use listening barriers/channels and virtual deadlines,
not unbounded waits or fragile sleeps. Existing regressions remain intact.

| Planned tests | Required assertions |
| --- | --- |
| Challenge valid/missing/malformed/multiple header, unexpected status, 401 and wrong qop/algorithm | Exactly one credential-free GET; no Cookie/Authorization/query; nonce never reaches result, logs, diagnostics or disk; no automatic follow-up. |
| Login success, non-200, misleading `200 OK` substring, failure, lockout, redirect and 5xx | Only canonical request, one attempt; no read after failure; no secrets/query in errors; separately approved cleanup policy observed. |
| Digest known-answer fixtures | Distinct query/header cnonces, fixed signing URIs, query nc=1, captured login-header consumption and data counter progression; no URI alternatives or random-counter reuse. This fixture contract needs reviewer resolution first. |
| Status XML valid, missing/duplicate sysinfo or fields, late malformed suffix, wrong type/charset, oversized/truncated body | Only the exact two valid tokens returned; no prefix/partial success or unbounded allocation. |
| DTD, ENTITY, references split across chunks, CDATA, namespace/prefix, PI, deep XML, mixed markup | Fail closed with fixed codes and zero retained body. |
| Extra IMEI/IMSI/ICCID/SIM/subscriber/serial/account fields in multiple subtrees | Sentinels absent from every serialized report, cache, log/trace, diagnostics, metrics and error path; only two allowed fields survive parser output. No DOM/raw response retained. |
| Secret lifecycle, including construction failure and unwind | Instrumented application-owned byte/native buffers zeroed on success, denial, each error, cancel, timeout and close; no Debug/Serialize clone path or UI/IPC/history persistence. Do not read freed memory to test clearing. |
| Cancellation at preview, credential entry, challenge, login, status and cleanup | No request before its approved step; cleanup entered on every exit; one safe logout at most after possible login; independent cleanup timeout; no hang or stale result. |
| Endpoint/source/gateway/interface/DHCP/USB change, identity conflict, session expiry | No credential leak to replacement; invalidate active authority; suppress unsafe logout; unknown DHCP stays unknown; identical scans do not kill valid binding. |
| Idle/busy/concurrent cancellation and competing auth operations | Only own guard released, all completed/rejected operations leave correct resource counts; no cancellation of another operation and no receipt consumption on busy. |
| Replay, cross-scope and cross-session claims, including old metadata/menu/strategy proofs | Bidirectional rejection; no nested grant minting or second cleanup child. Cold start requires no artificially verified evidence. |
| Closed registry matrix | Auth table contains exactly Challenge/Login/StatusRead/Logout, alongside three unchanged static entries. No generic XmlAction, alternate file/action, Download/Upload/set/deny-word path, encoded bypass or arbitrary query; test all eleven deny terms and secret-value false exclusions. |
| Per-field evidence consumers | Missing manufacturer/model/revision remains unknown; labels/USB/static observations never upgraded; plain_http tags cannot be dropped; extraction/write/flash/install/USSD stay disabled. |
| Cookie/session behavior | No cookie jar, Set-Cookie values or ambient browser session; unsupported dependency stops rather than enabling cookies or retrying. |
| Native release, if separately authorized | Owned isolated profile, real manual confirmations, No zero-dispatch evidence, credential privacy checks and before/after normal-data comparison; no automated Yes and no real firmware used as a fixture. |

Later implementation would run both Rust profiles, both Clippy profiles, format,
frontend tests/TypeScript/lint, production and isolated Windows builds and the
reviewer-specified repeat/native checks on its final code. These are planned
gates, not results claimed for this design. No Phase 6 simulator expansion or
physical adapter implementation is included here.

## Reviewer decisions, 23 October

These decisions supersede the original proposals/questions below; they do not
release authentication or a hardware run.

- Q1 accepted: two owner-issued scopes, AuthChallenge and
  AuthenticatedIdentityRead, plus derived CleanupLogout covered by the final
  identity dialog. No third independent confirmation.
- Q2 accepted: exactly one remote StatusRead; before/after means local binding
  and evidence rechecks. No two-remote-snapshot claim. Future protected work
  needs its own fresh same-session read.
- Q3 accepted: enter cleanup exactly once; dispatch only after a login may have
  reached the device, with unchanged safe binding and live cleanup deadline.
  Otherwise report logout_unconfirmed. Never logout before a login attempt.
- Q4 awaits a real credential-free challenge in separately accepted Part 3.
  Part 2 releases software only; synthetic fixtures cannot answer it.
- Q5 accepted: cookies disabled. Only boolean Set-Cookie presence, never its
  value. Presence stops any later authenticated work for review; no workaround.
- Q6 provisionally accepted: query login signs /cgi/protected.cgi with nc=1;
  its extra header signs /cgi/xml_action.cgi with nc=1; data header uses nc=2.
  Hardware proof requires a later supervised authenticated release, one attempt,
  no retry or alternate signing. Challenge observation does not prove it.
- Q7 accepted: HTTP 200 and at most 4 KiB UTF-8 text/html or text/plain body
  exactly `200 OK` after ASCII-whitespace trim. A different body fails with only
  sanitized type, length bucket and contains-200-OK boolean as specified above.
- Q8 accepted with restriction: first username/password release permits printable
  ASCII 0x20–0x7E only. Reject non-ASCII before any request with
  `This release supports ASCII passwords only`. Caps/deadlines otherwise as
  proposed; logout is always unconfirmed absent a later reviewed acknowledgment
  contract. This does not relax the closed login-query validation rules.
- Q9 answered by reviewer capture verification. device_name may be configurable;
  version_num is opaque. No Attribute::Model or firmware-identity mapping. Discard
  Wi-Fi ssid/mode as well as IMEI and every other unapproved field.
- Q10 accepted: typed in-memory envelope containing field, value, plain_http
  transport, scope/session/binding references, collection/expiry times. No profile
  EvidenceRecord mapping or stored row without separate contract review.
- Q11 remains unknown: do not guess DHCP-server evidence. B5 not released.

The released AuthChallenge approval expires 30 seconds after native confirmation,
not the five-minute future-flow proposal in section 2. Its only request is one
query-free, credential-free GET /login.cgi with a five-second network budget.
No Login, StatusRead, Logout, credentials or Secret use is released. Hardware
must wait for Part 2 code acceptance and a separate short guideline.

## 10. Original questions and current Part 2/3 gates

24 October C1 release supersedes the earlier Part 3 hold: after C1's software gate and commit, one owner-supervised credential-free challenge and separate No check are released. Q4 remains unanswered until that real run. Fixed shape facts may now explain unusable syntax without retaining nonce/opaque/domain values. No authenticated request, credential or login release follows from this amendment.

| ID | Question / conservative decision pending review |
| --- | --- |
| Q1 | Are the intended “three scopes” two owner grants plus derived CleanupLogout, or three independent owner confirmations? No third issuance is assumed. |
| Q2 | Does “identity before and after” require two remote status reads? Exactly one is currently described; no second read is authorized and no two-snapshot claim is made. |
| Q3 | Does the reviewer accept always-enter-cleanup with conditional safe logout, rather than sending after binding loss, before login or after cleanup expiry? No unsafe request is permitted to satisfy a literal “always”. |
| Q4 | Real status and challenge shape remain unknown. Released Part 2 software may emit sanitized realm/qop/algorithm, HTTP status and nonce/Set-Cookie/other-scheme presence booleans only; no nonce value. Hardware Part 3 is held until code acceptance and separate release. |
| Q5 | Does the device require a session cookie? JavaScript has cookie helpers/comments, but the cited flow does not prove a dependency. Keep cookies disabled; any dependency stops for a reviewed decision. |
| Q6 | Must login include the additional Digest header from `authentication()`, consuming nc=1, before the status header uses nc=2? Source supports that path; the abbreviated guideline does not mention it. No alternate-wire/signing experiment is authorized. |
| Q7 | What is the actual successful login body and lockout/failure grammar? Source's substring test is insufficient for a strict success contract. Do not guess HTML wrappers, localized lockout text or account lockout thresholds. |
| Q8 | Are the proposed charsets, response caps, username/password encoding and deadlines compatible? A real challenge outside them stops; no silent widening. Is there a trustworthy logout acknowledgment without a second read? |
| Q9 | Supply retained dashboard/internet captures to reverify inherited hashes/lines locally. Is device_name a model or configurable label, and what software component does version_num describe? No field inference or stale-proof authority while unanswered. |
| Q10 | How will transport/session/permitted-use qualifiers be represented and enforced by existing profile consumers without a schema change or generic trusted-model promotion? Proposed envelope needs its own review before code. |
| Q11 | Which DHCP-server/source-binding facts can be observed reliably on Windows? Current unknowns/collision blocks remain; B5 transport-binding implementation is not released here. |

Part 2 software implements the separately released one credential-free
`GET /login.cgi`, new AuthChallenge approval and bounded realm/qop/algorithm,
status and presence-flag output. Nonce and body are not retained; no credentials,
login query/Authorization/Cookie, status read or logout are sent. Its synthetic
and owned-loopback results are linked above. Part 3 hardware remains HELD for
code acceptance and a separate short release. Software fixtures cannot answer
Q4 or the real-device cookie question Q5, and no challenge result authorizes
authenticated code. No request or credential entry occurred in Part 1.

Still blocked: authenticated B4 client/credential entry; B2/schema-6 migration
and artifact classes; B5; B7 official-image registry; B8 analysis worker; Phase 8.
No firmware write, upgrade, reboot, restore, reset, factory reset, set, flashing,
installation, live USSD, credential guessing or unreviewed third-party tool.

## Documentation verification and review handoff

This task re-read and SHA-256-checked the three locally available base captures,
reviewed the current registry, Secret implementation, evidence/session contracts
and accepted Stage 1 evidence, and checked this document's diff/coverage. Full
application suites/builds/GUI/hardware were **not rerun** for this document-only
change; their accepted prior results are attributed above. No app data was
opened, migrated, saved or moved by this task; no network request was made.

Only `docs/PHASE_7_5_STAGE_2_AUTH_DESIGN.md` is changed. Branch
`phase-7-5-stage-2a-auth-design` is based on unmerged Stage 1 at start
`5df1aaceb4e4f8e352e5c7280edcae2b42635966`. End commit and clean-tree result are
provided in delivery (the document cannot embed its own commit ID).
Handoff command: `git diff main..HEAD`; the task-only change is
`git diff 5df1aac..HEAD -- docs/PHASE_7_5_STAGE_2_AUTH_DESIGN.md`.
Stage 1 and main remain untouched; no push, tag, merge or history rewrite.
The preceding handoff describes the original documentation-only delivery.
Part 2 was subsequently released by its own guideline; stop after its software
verification for review. No hardware or authenticated step is released.
