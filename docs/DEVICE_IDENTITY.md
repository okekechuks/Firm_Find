# Device identity

`apps/desktop/src-tauri/src/discovery/identity.rs` centralizes candidate classification and identity evidence. It does not provide device compatibility profiles or authentication adapters.

| Category | Evidence needed | Meaning |
|---|---|---|
| `router_mifi_candidate` | USB physical relationship and a non-software network adapter, without an explicit Android-compatible hint | Could also be another tethering device; dedicated router status is unverified |
| `usb_modem_candidate` | USB relationship with modem/COM function | Modem/control interface candidate; no command is sent |
| `network_gateway_candidate` | Active physical interface with a reported gateway | Gateway belongs to that interface; model/ownership unknown |
| `generic_network_interface` | Remaining network interfaces, including virtual/VPN records | Windows network hardware/software, not a confirmed router |
| `unrelated_device` | Other included USB/serial records | Advanced diagnostics only |
| `unknown` | No current classification; for example an old persisted summary | Refresh required for current evidence |

## Confidence and observations

Confidence is categorical: `verified`, `probable`, `candidate`, `unknown`, `conflicting`. Current production observations support only `candidate` or `unknown`; the model does not manufacture a verified identity. `probable` and `verified` require stronger future protocol evidence. The old numeric `confidence` wire field remains at zero for compatibility and is not displayed as a score.

Each observation carries kind, value, source, reliability and collection timestamp. Windows manufacturer properties describe the Windows device/driver record. Server headers are untrusted endpoint observations, retained with the saved endpoint; they are not parsed into a manufacturer. User labels are never fed into identity resolution. Verified manufacturer/model/firmware remain null when no supported protocol response exists.

`conflicting_claims` detects contradictory non-hint observations of the same identity claim and preserves both. Different component/driver manufacturer strings are retained as Windows properties rather than silently consolidated into a device manufacturer. No authenticated/vendor identity provider exists in Phase 2, so a real Windows-versus-management manufacturer conflict cannot yet be qualified on hardware. Fixture tests exercise contradictory structured claims. Future providers must use this resolver and mark conflicts before selecting any profile.

## Correlation and stable identifiers

`usb_root` follows at most 32 parents with cycle detection and stops at the first non-composite USB device rather than an upstream hub. `physical_key` prefers its nonzero USB container ID, then the Windows instance anchor. Shared non-USB PC container IDs are not used to merge internal devices. Adapter association uses the driver's `NetCfgInstanceId`; display names, manufacturer, VID alone, IPs and gateway addresses are never join keys. Canonicalized keys are SHA-256 hashed.

Manual endpoint IDs hash normalized URL plus selected interface. Different interface scopes remain separate even when they use the same IP. A manual entry's optional device association refers to the chosen Windows interface; it does not prove that the endpoint is the attached hardware's management service. Saved addresses can be removed and added again if their endpoint changes.

## Management identity boundary

An HTTP success proves response reachability. A redirect is recorded and not followed. A 401 or 403 establishes access restrictions, not the credential scheme or a successful login. A 404/405 can indicate a management UI that does not support HEAD; OBTAINER reports unsupported detection rather than inferring that no router exists. No response here grants firmware/configuration capabilities.

The user-reported **comFi MG11 Portable 4G Wi-Fi 6 MIFI**, USB-connected at `192.168.100.1`, is a physical test target, not a built-in vendor profile. Reported model, Windows USB/interface observations and endpoint results must remain distinct in the [verification evidence](PHASE_2_VERIFICATION.md).
