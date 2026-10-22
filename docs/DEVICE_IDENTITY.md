# Device identity

`apps/desktop/src-tauri/src/discovery/identity.rs` centralizes candidate classification and identity evidence. It does not provide device compatibility profiles or authentication adapters.

| Category | Evidence needed | Meaning |
|---|---|---|
| `router_mifi_candidate` | USB physical relationship and a non-software network adapter, without an explicit Android-compatible hint | Could also be another tethering device; dedicated router status is unverified |
| `usb_modem_candidate` | USB relationship with modem/COM function | Modem/control interface candidate; no command is sent |
| `network_gateway_candidate` | Backend-created remote target from an explicitly selected saved local endpoint and active physical route | Separate from the protected PC interface; endpoint identity/ownership and support require their own evidence and approval |
| `generic_network_interface` | Remaining network interfaces, including virtual/VPN records | Windows network hardware/software, not a confirmed router |
| `unrelated_device` | Other included USB/serial records | Advanced diagnostics only |
| `unknown` | No current classification; for example an old persisted summary | Refresh required for current evidence |

## Confidence and observations

Confidence is categorical: `verified`, `probable`, `candidate`, `unknown`, `conflicting`. Passive Windows observations support only `candidate` or `unknown`. Authenticated pinned management evidence is evaluated separately by bootstrap/profile matching and may establish a verified identity without granting unqualified feature support. The old numeric `confidence` wire field remains at zero for compatibility and is not displayed as a score.

Each observation carries kind, value, source, reliability and collection timestamp. Windows manufacturer properties describe the Windows device/driver record. Server headers are untrusted endpoint observations, retained with the saved endpoint; they are not parsed into a manufacturer. User labels are never fed into identity resolution. Verified manufacturer/model/firmware remain null when no supported protocol response exists.

Different Windows component/driver manufacturer strings are retained as `windows_manufacturer` observations, not contradictory authenticated router claims. The former unreachable `conflicting_claims` branch was removed. Phase 5 bootstrap/profile matching rejects conflicting authenticated identity and ambiguous authoritative profiles. Phase 7 pinned OpenWrt board responses can supply authenticated management evidence; OS-only identity never invents manufacturer or hardware revision. Physical qualification remains separate from the controlled OS fixture.

## Correlation and stable identifiers

`usb_root` follows at most 32 parents with cycle detection and stops at the first non-composite USB device rather than an upstream hub. `physical_key` prefers its nonzero USB container ID, then the Windows instance anchor. Shared non-USB PC container IDs are not used to merge internal devices. Adapter association uses the driver's `NetCfgInstanceId`; display names, manufacturer, VID alone, IPs and gateway addresses are never join keys. Canonicalized keys are SHA-256 hashed.

Manual endpoint IDs hash normalized URL plus selected interface. Different interface scopes remain separate even when they use the same IP. A manual entry's optional device association refers to the chosen Windows interface; it does not prove that the endpoint is the attached hardware's management service. Saved addresses can be removed and added again if their endpoint changes.

## Management identity boundary

An HTTP success proves response reachability. A redirect is recorded and not followed. A 401 or 403 establishes access restrictions, not the credential scheme or a successful login. A 404/405 can indicate a management UI that does not support HEAD; OBTAINER reports unsupported detection rather than inferring that no router exists. No response here grants firmware/configuration capabilities.

The user-reported **comFi MG11 Portable 4G Wi-Fi 6 MIFI**, USB-connected at `192.168.100.1`, is a physical test target, not a built-in vendor profile. Reported model, Windows USB/interface observations and endpoint results must remain distinct in the [verification evidence](PHASE_2_VERIFICATION.md).
