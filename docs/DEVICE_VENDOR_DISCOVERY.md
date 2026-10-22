# Stock-device family recognition

Sprint A collects only owner-approved root-page metadata, capped at 16 KiB. Only allowlisted title/server signals leave the parser; scripts are not evaluated and returned page text is not logged or persisted. Signals may suggest E-LINK/OEM, OpenWrt, Huawei, ZTE, D-Link or VIDA. These are candidate labels, not verified protocols or OEM attribution. No Goform, HiLink, VIDA or firmware-export API is called or claimed implemented.

USB VID/PID and Windows names remain observations. User labels/software remain user reports. Exact identity requires fresh, consistent, trusted evidence tied to one endpoint/interface; conflicts, expiration and substitution remove identity authority. Host NIC transport is kept separate from its remote router identity. No IMEI/SIM identifiers or credentials are requested.

Root transport reuses existing private/on-link interface selection, actual Windows route validation and WinHTTP pinning/TLS verification. No proxy, authentication, cookies, redirects or TLS bypass. Header bound: 16 KiB. Body bound: 16 KiB. HTTP deadline: three seconds after request setup, plus existing bounded DNS/TCP preflight. Cancellation closes request handles; callback-owned read buffers survive pending I/O until final handle closure.

Primary implementation references: [Microsoft WinHttpReadData](https://learn.microsoft.com/en-us/windows/win32/api/winhttp/nf-winhttp-winhttpreaddata) documents async read-buffer lifetime and null byte-count output; [WinHTTP option flags](https://learn.microsoft.com/en-us/windows/win32/winhttp/option-flags) documents transport policy. These support transport behavior, not vendor extraction capability. Candidate rules are explicitly runtime heuristics requiring later documented, exact-hardware qualification.
