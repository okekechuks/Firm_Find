# Web-management authentication and Windows source binding

Audit, 29 October 2026: the accepted production challenge collector only sends the separately approved credential-free `GET /login.cgi`. It deliberately discards nonce values. `endpoint_binding::Registry` already requires a live material stamp, successful fresh inventory for new approval, and current route/interface facts. Its transport uses WinHTTP after a global `GetBestRoute2` check; it does not bind the socket's local IPv4 address. Its conservative overlapping-subnet rejection remains unchanged.

The separate unmerged Stage 2 worktree contains a handwritten MD5-only response function. This milestone does not copy it. No physical credential submission, login, authenticated status read or firmware acquisition is qualified by that code.

## Dependency decision

Evaluate `digest_auth` 0.3.1 against RFC 7616. Its public challenge/context/response API supports advertised algorithms, qop, opaque, cnonce and nonce count without owning transport or retry policy. `diqwest` 3.2.0 instead wraps request dispatch and automatically repeats/caches authentication; that would hide the per-request approval and binding checkpoints. Use the smaller `digest_auth` primitive with an application-owned, bounded state machine.

Pin `digest_auth = 0.3.1`, `zeroize = 1.9.1` and the already resolved `reqwest = 0.13.5` in Cargo.lock. Enable reqwest blocking plus native TLS only, with defaults disabled; Windows uses the OS certificate trust store. `zeroize` 1.9.1 includes the upstream 6 October fixes and opaque Debug behavior. Credentials still deliberately implement neither Debug nor Serialize. Digest authentication does not provide encryption or prove hardware identity on plain HTTP.

Primary references: [digest_auth API](https://docs.rs/digest_auth/0.3.1/digest_auth/), [diqwest dispatch model](https://docs.rs/diqwest/3.2.0/diqwest/), [RFC 7616](https://www.rfc-editor.org/rfc/rfc7616.html), [reqwest source binding](https://docs.rs/reqwest/0.13.5/reqwest/blocking/struct.ClientBuilder.html#method.local_address), [GetBestRoute2](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-getbestroute2), [GetAdaptersAddresses](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses), [zeroize changelog](https://github.com/RustCrypto/utils/blob/master/zeroize/CHANGELOG.md).

## Authority and scope

HAR and static bundle routes remain evidence, never request authority. The production qualified identity/status route registry is empty. A public factory cannot create a verified route from a URL or a caller-provided `verified` boolean. Therefore this milestone's authenticator is reusable and exercised through deterministic test-only connectors, while the physical Authenticate action remains blocked until a device-family route receives independent qualification.

NONE, BASIC, DIGEST, COOKIE_SESSION, FORM_SESSION and UNKNOWN are distinct scheme identifiers. Only the reusable Digest calculation and controlled identity-read driver are implemented here; Basic and cookie/form session clients remain unsupported. Basic would require transport confidentiality and explicit scope before implementation. Existing credential-free metadata reads remain separate. No automatic downgrade, arbitrary URL replay, credential guessing, login CGI special case or sensitive request is permitted.

The Digest wrapper supports MD5, SHA-256 and SHA-512/256, their session forms with qop, auth and empty-GET auth-int, opaque, actual path/query, monotonic nonce count and random client nonces. A fresh challenge is required for each operation; one fresh stale-nonce retry is allowed without changing realm or algorithm. Legacy no-qop MD5 is supported. Session algorithms without qop are rejected because digest_auth 0.3.1 omits their required cnonce in serialization. Unknown algorithms, duplicate parameters, malformed challenges and unsupported encodings fail closed. The strict pre-parser currently rejects quoted-pair escapes rather than guessing their meaning.

Credentials and authorization material have no Serialize or Debug implementation, remain operation-local, and are erased on drop. Dependency-internal temporary allocations and OS network buffers cannot be claimed to be fully erased. No credential or Authorization value enters application logs, reports, SQLite or diagnostics.

## Selected-adapter transport

The new transport retains actual adapter GUID, LUID, index, local IPv4, prefix and management destination. It checks current `GetAdaptersAddresses` records and asks Windows `GetBestRoute2` for the selected interface and explicit source. It can construct a reqwest client with `local_address`, no proxy, no redirects, no cookie store, a two-second connect timeout and a five-second request timeout. TLS uses Windows platform validation. The private client exposes only construction of a qualified identity GET, and has no dispatch method. No production connector is registered. A future reviewed connector must enforce bounded response bodies and invoke source/binding checks immediately before dispatch and before publication; controlled tests already enforce those checkpoints at the driver boundary. It neither changes routing nor installs persistent routes. Overlapping subnets remain blocked until actual Windows qualification proves source binding is safe for that case; a mock route result cannot qualify physical hardware.

`management_binding_ipc::assess_web_management_binding` now exposes local adapter
and route observations in the selected-device workflow. It performs no HTTP,
rechecks the live inventory before returning, hides source details after failure
and always returns `canAuthenticate: false` and `cryptographicallyVerified: false`.
HAR evidence and a successful local route observation cannot qualify one another.
A separate controlled Windows loopback socket regression tests reqwest source
binding; its synthetic route permit is compiled only for tests and does not
qualify a physical adapter or change production target restrictions.

## Verification

Tests use synthetic challenges and controlled connectors only. Initial combined behavioral red: 79 passed / 22 failed, including four authentication/binding stubs. The first implemented run exposed two incorrect expected RFC-vector strings and one sandbox directory-lease denial. RFC 2617 uses `Circle Of Life`, while RFC 7616's SHA-256 example uses `Circle of Life`; those passwords are distinct. The printed RFC 7616 SHA-512/256 response has [reported erratum 4897](https://www.rfc-editor.org/errata_search.php?rec_status=0&rfc=7616); the regression uses an independently computed Python hashlib FIPS SHA-512/256 result, not the disputed printed value. After corrections, the acquisition suite passed 113 tests / 0 failed under the native directory-lease permissions. A separate failing-first dependency-edge test demonstrated the missing-cnonce problem for no-qop session algorithms before fail-closed rejection was added.

The graph projection suite passed five tests / zero failed. Final suite totals and exact log locations are recorded in the milestone verification report. Cancellation, timeout, failed authentication, fresh/stale challenge handling, selected identity mismatch, endpoint/source substitution, single-use authorization, unqualified routes and overlapping subnet refusal are covered locally. A real Windows loopback socket verifies selected-source behavior; physical adapter binding, authentication and acquisition remain unqualified. No live router request is authorized by this document.
