# Device discovery

OBTAINER inventories Windows devices without opening modem/control ports or sending management requests. Devices is the existing desktop navigation destination; browser preview cannot enumerate hardware.

## Native implementation

- `apps/desktop/src-tauri/src/discovery/windows.rs`: SetupAPI enumerates present nodes, USB/network/serial device interfaces, friendly names, manufacturer properties, hardware/compatible IDs and device paths. Configuration Manager supplies parents and USB container IDs. Registry reads are restricted to COM names and adapter GUID associations.
- The same provider extends `GetAdaptersAddresses` with IPv4/IPv6 addresses, on-link prefix lengths, DNS, interface indexes, IPv6 scope indexes and LUIDs. `GetIfEntry2` reports hardware/software interface status. `GetIpForwardTable2` associates every observed default route with its LUID; displayed route metrics are offsets, not a combined preference score.
- `discovery/mod.rs::assemble` retains the existing USB ancestry/container correlation and adapter-GUID join. GUID/index is not inferred from a matching display name. SHA-256 IDs use the physical anchor; two devices with the same VID/PID or private gateway are not merged. Without a hardware serial, moving a device between USB ports can change Windows identity.
- `discovery/service.rs` runs enumeration on its existing worker. Configuration Manager and IP Helper notifications are debounced (200 ms quiet interval, one-second cap); a 30-second reconciliation remains. Generations cancel queued scans and reject superseded results. Windows API calls already in progress finish before cancellation is applied; no thread is forcibly terminated.
- Failed enumeration retains the last successful inventory and exposes a warning. Refresh and retry remain available. A cancelled scan does not replace the last inventory. Snapshot revisions order frontend updates; enumeration duration is measured.

## What the UI means

USB/network relationships produce **candidates**, not confirmed routers. Active physical gateway interfaces become network gateway candidates. Serial/modem functions may produce USB modem candidates. Software interfaces remain generic. Unrelated USB hardware is hidden until advanced diagnostics is enabled. Android-compatible interface evidence prevents a dedicated MiFi classification; unidentified tethering devices remain uncertain regardless of names.

Windows `connected` means the inventory record is present. The separate lifecycle describes discovery/management state. An active adapter does not establish management connectivity. `reachable` means an HTTP response was observed. `authentication_required` describes denied/required access; `connected` is reserved for a future established management session and is never emitted by Phase 2. Missing devices become disconnected; the same physical anchor reconnects without a second logical device.

## Manual targets and endpoint checks

`connection.rs`, `connection/windows.rs`, `commands.rs`, `ManualRouters.tsx` and validated frontend bindings provide the complete saved-target workflow. Add Router accepts a hostname, IPv4 or IPv6 address and HTTP/HTTPS port. Gateway buttons prefill the address and its Windows interface. Names are explicitly user labels. Saving performs no network request.

Each check requires explicit ownership/administration confirmation. The backend independently requires that confirmation and revalidates the saved target. It resolves at most one chosen endpoint after bounded Windows DNS resolution, rejects non-local answers, and requires an active on-link physical interface or its associated gateway. Ambiguous interfaces require a selection. `GetBestRoute2` checks that Windows would use that interface, including in the presence of a VPN. The request pins the validated numeric resolution while retaining the original Host/TLS name; redirect destinations are never requested. Network changes during a check invalidate publication.

Only one TCP connection preflight and one unauthenticated `HEAD /` are attempted. No body, login page, passwords, scripts, arbitrary paths, SSH commands or vendor API actions are fetched. There are at most two in-flight checks, a bounded DNS timeout, 800 ms TCP preflight and three-second asynchronous HTTP deadline. Cancellation is observed every 50 ms during HTTP waiting; DNS/preflight complete within their own bounds. Errors distinguish validation/scope, routing, refusal, DNS, timeout, TLS, authentication, redirect, unknown identity and unsupported management.

WinHTTP uses no proxy, cookies, default credentials or authentication retry. Header size is bounded to 16 KiB and the retained server header to 255 UTF-16 units. HTTPS uses Windows certificate validation. On trust failure the UI explains deliberate owner/fingerprint/hostname verification and Windows trust-store steps; OBTAINER never imports a certificate or bypasses validation. Hostname pinning requires Windows 10 21H1 or newer; unsupported Windows versions fail closed.

## Persistence and privacy

Schema 2 adds `manual_routers` and `discovery_metadata` to the existing SQLite database at `%LOCALAPPDATA%\OBTAINER\obtainer.db`. Settings, firmware, jobs, audit and existing inventory rows are preserved. Stored discovery metadata consists of hashed association IDs, category, confidence and last-seen time; no full hardware dump or credentials are saved. Up to 100 manual entries retain the latest bounded endpoint observation. There is no automatic probe at startup. Historical responses remain labelled historical until checked again; disconnected summary records survive restart.

The UI hides potential serial portions of instance/interface paths. Logs do not contain raw inventory or passwords. Developer fixtures and local HTTP test servers are separate from production discovery and cannot be selected through IPC.

## Verification

Run the existing check commands from [Phase 2 verification](PHASE_2_VERIFICATION.md). Native regression coverage extends `scripts/verify-foundation.mjs` with `--phase2`; `scripts/verify-discovery.mjs` uses the real WebView2, IPC, SQLite and WinHTTP against explicitly labelled local fixtures. A physical target is only checked when supplied using `--router <local address>`.

API references: [GetAdaptersAddresses](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses), [route rows](https://learn.microsoft.com/en-us/windows-hardware/drivers/network/mib-ipforward-row2), [WinHTTP options](https://learn.microsoft.com/en-us/windows/win32/winhttp/option-flags), [asynchronous cancellation and callback lifetime](https://learn.microsoft.com/en-us/windows/win32/api/winhttp/nf-winhttp-winhttpclosehandle), [bounded DNS resolution](https://learn.microsoft.com/en-us/windows/win32/api/ws2tcpip/nf-ws2tcpip-getaddrinfoexw).
