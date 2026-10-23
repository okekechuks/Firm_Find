# Phase 7.5 B4 pre-step plan — approved as plan; implementation not authorized

23 October 2026. Design only: no request spec, approval scope, parser, transport or device request is implemented by this plan. Authenticated CGI work remains blocked.

## Question and static evidence

Resolve the literal dashboard response `file` name without guessing a CGI endpoint. Captured root line 24 sets `g_platformName = "mifi"`; `layout_manager.js:121` builds `xml/ui_` + platform + `.xml`. Lines 53–75 and 90–91 map menu `xmlName` to the selected panel. `dashboard.js:7` defines the plugin `$.fn.objdashboard`; lines 682–684 read `sysinfo/device_name` and `version_num`. The candidate discriminator is therefore the case-exact `implFunction="objdashboard"`, derived from static code and unconfirmed against a real menu body. Literal filename remains unknown; no menu capture is supplied.

Both captures and cited lines were re-read for this review. SHA-256 matches the anchors in [the accepted design](FIRMWARE_ACQUISITION_ENGINE.md): `layout_manager.js` = `9c8d5fd1fc47111180b80ceee1261cc38aa560fc1d53b8e03e286cae6fc3664c`; `dashboard.js` = `f57e620d871ec877920d1a04cef4a7e41eb2db0af20c77bd457b967c40ae381d`. These are static-code citations, not hardware observations.

## Proposed single static read

After separate approval, add exactly one closed `RequestSpec`: `GET /xml/ui_mifi.xml`, no query, no body, maximum 16 KiB, one attempt, five-second total network deadline, XML response only, no redirect following. It shares the existing native confirmation mechanism but requires a new scope named `StaticMenuMetadata`, bound to the selected device, discovery revision, physical interface, normalized endpoint, exact request ID and expiry. Existing `EndpointMetadata` and strategy-probe receipts cannot authorize it. Scope is consumed once; no scope is issued by current code. It grants neither authentication nor subsequent CGI access.

Proposed confirmation text (values use existing single-line, 80-character sanitization):

> Approve one read-only static menu check?
>
> Device: {device}
>
> Interface: {interface}
>
> Endpoint: {normalized endpoint}
>
> Request: GET /xml/ui_mifi.xml (maximum 16 KiB, one attempt)
>
> This reads the dashboard menu mapping only. No login, settings change, firmware transfer or other request is authorized. Plain HTTP is not cryptographically verified. Proceed only if you own or administer this device.

Buttons: Yes / No. Receipt expires 30 seconds after Yes. Network request must finish within its own bounded deadline and remaining receipt lifetime. Cancellation, stale discovery or interface/endpoint change invalidates the result.

## Authentication expectation and stop rules

`callXML` at `js/base/layout_manager.js:37–47` is a plain jQuery request with `type: "GET"`, the supplied `url`, `dataType: "xml"`, `async: false` and a success callback. It has no `Authorization` header or Digest helper. Combined with the static menu construction at line 121, this supports an unauthenticated static GET expectation, not authenticated CGI or hardware evidence. This citation does not prescribe blocking I/O in OBTAINER. A 401 means this read is protected or access is otherwise refused; stop and record that limitation. Do not supply credentials or infer the authentication method. On any redirect, non-success status, unexpected content type, oversized/truncated body, malformed XML, missing/duplicate dashboard entry, ambiguous mapping, timeout or cancellation: stop, record a bounded limitation, do not retry or request another path. A relative redirect is also refused.

## Proposed parser and tests

Parse bounded XML with DTDs, external entities and entity expansion disabled. Follow the exact shapes in `parseXml`, `layout_manager.js:53–75`: `Tab` elements have `Name` and `type`; tabs whose case-exact `type` is `submenuabsent` carry `implFunction` and `xmlName` directly. Other tabs contain nested `Menu` elements with `id`, `implFunction` and `xmlName`. Select only eligible entries whose case-exact `implFunction` equals `objdashboard`. Different case or spelling is no match. This rule is derived from static code and remains unconfirmed against a real menu body.

Require exactly one matching entry across both shapes and extract only its `xmlName`. Zero matches, multiple matches, missing/empty `xmlName`, or a value other than one bounded literal token returns `no unique dashboard entry` and extracts nothing. Reject whitespace, separators, query, fragment, traversal, percent encoding and action syntax. Discard all other attributes, nodes and fields. Never store or log raw body, credentials, subscriber identifiers or menu contents. A filename observation grants no request authority; any future CGI spec needs its own source citation and review.

Use synthetic menu fixtures until the owner supplies a genuine menu capture. Do not label synthetic data as captured. Review a supplied capture for device-specific data before trimming; retain no unique identifiers. Proposed tests cover both Tab/Menu shapes, case-exact `objdashboard`, different case/spelling, unrelated panels, duplicate matches across shapes, zero matches, missing/empty/invalid `xmlName`, namespaces, malformed/oversized/truncated XML, DTD/entity payloads, path/query injection, cancellation and stale approval. Do not infer a literal filename or menu position from this static discriminator. These tests and parser are not implemented; separate authorization remains required.

Closed-table tests must prove only this additional literal path is added after approval. Caller-supplied paths, encoded variants, other hosts, redirects, login/logout and `xml_action.cgi` remain rejected. Test separate scope, replay, expiry, changed endpoint/interface/revision and owner No without traffic. No wildcard path or dynamically assembled vendor-JavaScript URL becomes a spec.

## Owner-supervised hardware step (future approval required)

Owner confirms powered USB device and selected management endpoint, records current web-display settings/version, then approves the exact native dialog. Run one static GET on a non-conflicting route. Stop under every rule above or if owner declines. Do not change Windows routes, log in, send credentials or follow links. Show only extracted filename observation or limitation. Owner checks the same web display afterward; quote dated owner words with that limited scope. A successful menu read is not authenticated identity or firmware acquisition. Any authentication or further request requires another reviewed guideline.
