# Phase 7.5 hardware matrix

22 October 2026. Observations and reports are not verified hardware identities. H02 and H03 are independent targets. No reset, flash, privileged maintenance, live USSD or device write is permitted by this phase.

| ID | Owner-reported target | Detection / identity evidence | Management | Acquisition / analysis | Gate |
| --- | --- | --- | --- | --- | --- |
| H01 | comFi MG11 4G MiFi | Historical generic USB/RNDIS observation, VID 1286/PID 4E31; reported LT800 and MF69_AM802L_ZX_DD_N_241106_EU remain unverified | Historical unauthenticated root GET: HTTP 200, E-LINK Wireless Router; candidate family, not confirmed OEM/API | No qualified source/export/read; no firmware artifact | Windows detection and native GUI/IPC verified; selected-USB HTTP BLOCKED by competing Wi-Fi route; exact identity/acquisition pending |
| H02 | Airtel VIDA CPE2000B, older revision | No new observation | Unknown | Unknown | Physical qualification pending |
| H03 | Airtel VIDA CPE2000B, newer revision | No new observation; must not inherit H02 qualification | Unknown | Unknown | Physical qualification pending |
| H04 | Airtel DWR-930-M 4G MiFi | No new observation | Unknown | Unknown | Physical qualification pending |
| H05 | MF968 4G LTE Mobile WiFi | No new observation; OEM not assumed | Unknown | Unknown | Physical qualification pending |

Owner check: connect and power on one target; open Connected Devices, More Info; confirm Windows observations and selected local management address/interface. Optionally approve one read-only root-page check and run Check Compatibility. Export Diagnostics to a new file. Record model/revision/firmware separately as user reports until a qualified identity response is available. A candidate family or HTTP success never proves firmware export. Obtain Firmware must stay disabled without a qualified executable route.

Current H01 qualification: native owner-approved check returned `no_route`, HTTP status absent. Windows preferred Wi-Fi while both interfaces shared 192.168.100.0/24. No wrong-interface fallback was attempted. For USB-only verification, owner can temporarily disconnect PC Wi-Fi, keep USB active, rerun approved Check Compatibility, save redacted diagnostics, then reconnect Wi-Fi. No firmware support follows from an HTTP response alone.
