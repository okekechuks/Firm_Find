# Phase 7.5 hardware matrix

22 October 2026. Observations and reports are not verified hardware identities. H02 and H03 are independent targets. No reset, flash, privileged maintenance, live USSD or device write is permitted by this phase.

| ID | Owner-reported target | Detection / identity evidence | Management | Acquisition / analysis | Gate |
| --- | --- | --- | --- | --- | --- |
| H01 | comFi MG11 4G MiFi | Native generic USB/RNDIS observation, VID 1286/PID 4E31; reported LT800 and MF69_AM802L_ZX_DD_N_241106_EU remain unverified | Owner-approved USB-only root GET: HTTP 200, candidate family `e_link_oem`; exact identity and authenticated API unknown | Five unknown, nonexecutable acquisition routes; no qualified source/export/read; no firmware artifact | Windows detection, native GUI/IPC and USB root response VERIFIED; exact identity/acquisition pending |
| H02 | Airtel VIDA CPE2000B, older revision | No new observation | Unknown | Unknown | Physical qualification pending |
| H03 | Airtel VIDA CPE2000B, newer revision | No new observation; must not inherit H02 qualification | Unknown | Unknown | Physical qualification pending |
| H04 | Airtel DWR-930-M 4G MiFi | No new observation | Unknown | Unknown | Physical qualification pending |
| H05 | MF968 4G LTE Mobile WiFi | No new observation; OEM not assumed | Unknown | Unknown | Physical qualification pending |

Owner check: connect and power on one target; open Connected Devices, More Info; confirm Windows observations and selected local management address/interface. Optionally approve one read-only root-page check and run Check Compatibility. Export Diagnostics to a new file. Record model/revision/firmware separately as user reports until a qualified identity response is available. A candidate family or HTTP success never proves firmware export. Obtain Firmware must stay disabled without a qualified executable route.

H01 history: first native owner-approved check returned `no_route`, HTTP status absent, because Windows preferred Wi-Fi while both interfaces shared 192.168.100.0/24. No wrong-interface fallback was attempted. After the owner disconnected Wi-Fi and confirmed USB ready, the same native release check passed 21 checkpoints with zero console issues. The bounded root GET returned HTTP 200 / `identity_unknown`, candidate `e_link_oem`, with metadata capped/truncated. Redacted diagnostics and an actual GUI screenshot were captured; normal data remained unchanged. The owner was told Wi-Fi can be reconnected. [Latest report](evidence/phase-7-5-h01-redacted-2026-10-22.json) and [earlier route block](evidence/phase-7-5-h01-routing-blocked-2026-10-22.json) remain separate evidence. No firmware support follows from an HTTP response alone; `canObtain: false` and all four physical features remain UNKNOWN.
