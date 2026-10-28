# MG11 vendor firmware requests — UNSENT

Prepared: **October 28, 2026**. These are two separate drafts for owner review. Neither request has been sent, and no vendor response or support commitment exists.

## Recipient and evidence limits

Vitel publishes `support.love@vitelwireless.com` on its [official support page](https://www.vitelwireless.com/vitel-wireless-support), checked October 28, 2026. This is the primary support lead for the Vitel-branded unit. Movingcomm publishes `marketing@movingcomm.com` on its [official ComFi MG11 product page](https://www.movingcomm.com/products/mg11-4g-wifi-6-mifi.html) and [official contact page](https://www.movingcomm.com/contact.html). Movingcomm is an OEM lead; its product page does not establish that it manufactured either particular unit or that its firmware is interchangeable. Ask the recipient to confirm the responsible manufacturer or authorized support channel before supplying a package.

The following values come from owner reports and screenshots. They remain **user-reported and unverified**; they are not authenticated device identity or a compatibility decision.

| Local target | Owner description | Reported hardware | Reported software | Reported software date |
| --- | --- | --- | --- | --- |
| H01-A | Vitel MG11 | `MF69_AM802` | `MF69_AM802_ZX_DD_N_250725_EU` | July 25, 2025 |
| H01-B | Universal MG11 | `MF69` | `MF69_AM802L_ZX_DD_N_241106_EU` | November 7, 2024 |

The `AM802` and `AM802L` software identifiers must be kept distinct. A response for one target does not qualify the other.

## Draft 1 — H01-A Vitel MG11

**To:** `support.love@vitelwireless.com` (Vitel support). If referred to the OEM, send separately to its confirmed support channel; Movingcomm's public marketing contact is a lead, not a confirmed manufacturer for this unit.

**Subject:** Exact original firmware and documented read-only export request — MG11 / MF69_AM802

Hello,

I own a device sold as a Vitel MG11 and am investigating a legitimate, read-only way to preserve its original firmware for local analysis. Its web display reports:

- Hardware: `MF69_AM802`.
- Software: `MF69_AM802_ZX_DD_N_250725_EU`.
- Software date: July 25, 2025.

These are display values, not an independently verified hardware identification. Please first confirm whether this exact variant is your product and identify the responsible manufacturer or authorized support channel if it is not.

For this exact hardware and software combination, please provide or document:

1. The original manufacturer-approved firmware package matching the currently reported release, with its expected byte size, SHA-256, publisher signature, public verification key or certificate, and signature-verification instructions. If no signed package is available, please state that explicitly and provide the strongest official authenticity evidence available.
2. The exact supported board/hardware revision, product identifier, region, carrier branding and any compatibility restrictions. Please confirm whether the `EU` suffix specifies a region and whether the Vitel-branded unit requires a separate package. Do not substitute an `AM802L` package or a package for another hardware revision.
3. The package format and whether it is a full image, recovery/service package or differential update. For a delta, specify the exact required base release, partition sizes and base hashes, and whether a complete matching original image is available separately. Please describe the included and excluded partitions.
4. Any documented, owner-authorized **read-only export of installed firmware** or service backup. Please explain whether it returns actual installed firmware bytes, a vendor image, settings only, calibration/NV data or another artifact; identify its partition coverage and omissions. Provide the official procedure and interface documentation rather than undocumented maintenance commands.
5. Official recovery documentation for this exact revision, including prerequisites, supported recovery interface, backup requirements and any regions or device-specific data that must be preserved. This request is for documentation only; no recovery or installation operation is being requested now.

The owner’s currently observed configuration page exposes restore/update controls and IMEI/TTL controls, but no export control has been observed there. We have not used those controls. Please identify any legitimate export feature for this exact release without requiring a reset, firmware update, identity modification or credential bypass.

If an export requires authentication, please describe the supported login/session behavior and whether it affects an existing browser session. We will not send credentials by email. If the interface uses plain HTTP, please state that; an HTTP response or download alone will not be treated as cryptographically verified identity or permission to write the device.

Please do not send credentials or device-specific identifiers. We are not requesting carrier unlocking, cross-flashing or a modified image, and we do not intend to install any supplied package during this investigation.

Thank you.

## Draft 2 — H01-B universal MG11

**To:** `marketing@movingcomm.com`

**Subject:** Exact original firmware and documented read-only export request — MG11 / MF69 / AM802L release

Hello,

I own a device sold as a universal MG11 and am investigating a legitimate, read-only way to preserve its original firmware for local analysis. Its web display reports:

- Hardware: `MF69`.
- Software: `MF69_AM802L_ZX_DD_N_241106_EU`.
- Software date: November 7, 2024.

These are display values, not an independently verified hardware identification. Please first confirm whether this exact variant is your product and identify the responsible manufacturer or authorized support channel if it is not.

For this exact hardware and software combination, please provide or document:

1. The original manufacturer-approved firmware package matching the currently reported release, with its expected byte size, SHA-256, publisher signature, public verification key or certificate, and signature-verification instructions. If no signed package is available, please state that explicitly and provide the strongest official authenticity evidence available.
2. The exact supported board/hardware revision, product identifier, region, carrier branding and any compatibility restrictions. Please clarify the relationship between the displayed hardware value `MF69` and the `AM802L` software family, and whether `EU` imposes regional restrictions. Do not substitute the other unit’s `MF69_AM802_ZX_DD_N_250725_EU` package.
3. The package format and whether it is a full image, recovery/service package or differential update. For a delta, specify the exact required base release, partition sizes and base hashes, and whether a complete matching original image is available separately. Please describe the included and excluded partitions.
4. Any documented, owner-authorized **read-only export of installed firmware** or service backup. Please explain whether it returns actual installed firmware bytes, a vendor image, settings only, calibration/NV data or another artifact; identify its partition coverage and omissions. Provide the official procedure and interface documentation rather than undocumented maintenance commands.
5. Official recovery documentation for this exact revision, including prerequisites, supported recovery interface, backup requirements and any regions or device-specific data that must be preserved. This request is for documentation only; no recovery or installation operation is being requested now.

We have seen a configuration-export example on page 18 of the [manual for the **different ComFi IX240 product**](https://27648563.s21i.faiusr.com/61/ABUIABA9GAAghMyCxAYoyPCHqwE.pdf). That example does not establish MG11 support. Please confirm whether this particular MG11 release has its own documented export facility and distinguish a configuration backup from an installed firmware image. No configuration-export support is assumed for this unit.

If an export requires authentication, please describe the supported login/session behavior and whether it affects an existing browser session. We will not send credentials by email. If the interface uses plain HTTP, please state that; an HTTP response or download alone will not be treated as cryptographically verified identity or permission to write the device.

Please do not send credentials or device-specific identifiers. We are not requesting carrier unlocking, cross-flashing or a modified image, and we do not intend to install any supplied package during this investigation.

Thank you.

## Handling any future response

Keep both requests and responses separate. A vendor package is a manufacturer-supplied artifact, not proof that it was dumped from either connected device. Preserve the original download, compute its size and hash, verify any publisher signature independently, and review the exact hardware/region/base-release restrictions before associating it with a target. A settings backup must remain classified as a configuration artifact. No response by itself authorizes authentication, physical storage access, configuration writes, installation or recovery.

No IMEI, serial number, SSID, credentials or other device-specific secrets are included in these drafts. Any future disclosure or operation needs its own owner decision.
