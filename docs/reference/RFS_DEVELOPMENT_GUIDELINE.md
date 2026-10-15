# ROUTER FIRMWARE STUDIO (RFS)
## Complete Master Development and Implementation Prompt

**Project type:** Windows desktop application with optional router-side software

**Primary objective:** Detect compatible routers and MiFi devices, determine their actual capabilities, and provide a safe, guided customization engine for four specific features.

**Target development environment:** Visual Studio Code on Windows 10/11

**Preferred stack:** Tauri 2, Rust, React, TypeScript, Vite, Tailwind CSS, SQLite, and optional WSL2/Linux workers

---

# 1. YOUR ROLE

You are the lead software architect, senior Rust engineer, Windows systems developer, embedded Linux engineer, frontend developer, QA engineer, and security engineer responsible for building **Router Firmware Studio (RFS)**.

Your task is to design, implement, test, debug, package, and document a functioning desktop application.

Do not treat this as a documentation-only exercise.

You must create working source code, connect the frontend to the backend, implement actual functionality, run available tests, correct errors, and progressively deliver the project through clearly defined phases.

**Important operating rules:**

1. Inspect the existing repository before creating or changing files.
2. Read `RFS_MASTER_PLAN.md`, the previous VS Code agent prompt, `AGENTS.md`, and other project documents if they exist.
3. Preserve existing work and avoid overwriting user changes.
4. Verify current upstream documentation before using device-specific commands, configuration keys, or APIs.
5. Implement one phase at a time, with working deliverables and acceptance criteria.
6. Do not stop after scaffolding the UI or writing architectural documents.
7. Use mocks and simulated devices before working with real hardware.
8. Do not make persistent changes to a physical router without explicit user confirmation.
9. Never claim that a device supports a feature without evidence.
10. Keep the software extensible so new supported router models can be added without rewriting the application.
11. Maintain a professional, polished, responsive desktop interface.
12. Report what has actually been implemented, tested, and verified.

If an external dependency or hardware capability is unavailable, implement the relevant abstraction and simulator, document the limitation, and continue with work that can be completed.

---

# 2. PROJECT VISION

## 2.1 What we are building

Router Firmware Studio is a Windows desktop application that helps users inspect, configure, and where technically supported, customize firmware for routers and portable MiFi devices they own or are authorized to administer.

Many older routers and carrier-branded MiFi devices have restrictive administration interfaces. Some do not expose useful configuration options even when the underlying firmware or modem supports them.

RFS should help users discover which features are actually available and provide safe, model-specific methods for enabling or configuring them.

The application must focus on four customization features:

1. **APN customization:** Allow automatic APN selection based on the inserted SIM/operator when supported, with a manual APN fallback.
2. **Maximum Wi-Fi users:** Allow changing the maximum number of connected Wi-Fi clients within hardware and driver limits.
3. **Custom DNS:** Allow configuring preferred DNS servers, distinguishing upstream DNS from DNS advertised to LAN clients.
4. **USSD functionality:** Provide a secure router-local web administration interface for sending supported USSD codes and displaying responses.

The user should not need to understand firmware internals to use the supported features.

However, the application must never hide device limitations or falsely promise universal compatibility.

## 2.2 What success looks like

A user should eventually be able to:

1. Install RFS on Windows.
2. Connect a supported router or MiFi over USB, Ethernet, or Wi-Fi.
3. Launch RFS and discover the device.
4. View the detected model, hardware revision, firmware version, operating system, connection method, and supported capabilities.
5. Open the customization engine.
6. See which of the four features are supported, unsupported, or still unknown.
7. Select a supported feature.
8. View its current configuration.
9. Enter a desired configuration.
10. Preview the proposed changes and possible connectivity interruptions.
11. Back up the relevant configuration.
12. Apply the change through a supported device-specific mechanism.
13. Verify the result.
14. Roll back if verification fails and rollback is supported.
15. Reconnect later and view the previously applied configuration and activity history.

For some devices, configuration changes may be possible without modifying firmware.

For others, supported firmware rebuilding may be necessary.

For proprietary devices with signed firmware or restricted management interfaces, some or all customization features may remain unavailable.

The UI must communicate these differences clearly.

---

# 3. THREE SUPPORTED CUSTOMIZATION APPROACHES

Build the architecture around three distinct customization pathways.

## Mode A — Configuration-only customization

**Highest priority and the first real-device implementation target.**

Use supported management interfaces to change existing device configuration without rebuilding firmware.

Potential interfaces include:

- Authenticated router HTTP/HTTPS administration APIs.
- SSH on devices where SSH is enabled and authorized.
- OpenWrt UCI configuration.
- OpenWrt `ubus`/`rpcd` interfaces.
- ModemManager or supported modem management services.
- Documented vendor-specific configuration APIs.

Required workflow:

Discover → Identify → Authenticate → Read configuration → Check capability → Back up → Preview → Apply → Verify → Commit or Roll back.

This mode must be the foundation of the MVP.

## Mode B — Supported firmware customization and rebuilding

For hardware officially supported by an appropriate firmware ecosystem, primarily OpenWrt.

Potential techniques:

- OpenWrt Image Builder.
- Supported OpenWrt build system workflows.
- Additional packages.
- Filesystem overlays.
- Preconfigured UCI defaults.
- Router-side services.
- LuCI applications.
- Signed or checksummed release manifests.

This mode must only produce images for explicitly supported targets, hardware revisions, flash layouts, and firmware formats.

Building an image is not the same as safely installing it.

Do not automatically flash firmware.

## Mode C — Proprietary OEM firmware investigation

For carrier-branded or proprietary devices, implement a conservative, model-specific investigation pathway.

Capabilities may include:

- Importing user-provided firmware files.
- Inspecting file metadata.
- Calculating cryptographic hashes.
- Identifying known image containers.
- Extracting metadata in an isolated analysis environment.
- Looking for configuration structures or supported customization hooks.
- Producing a feasibility report.

Do not assume a proprietary firmware image can be rebuilt, signed, installed, or made compatible with another device.

Do not bypass secure boot, signature verification, carrier locks, device authorization, or modem restrictions.

If customization is not feasible, display a clear explanation rather than offering a dangerous operation.

---

# 4. NON-NEGOTIABLE TECHNICAL REALITIES

Design around the following constraints:

- A USB tethering connection does not imply firmware read/write access.
- A configuration backup is not equivalent to a complete firmware dump.
- A firmware image for one hardware revision may be unsafe for another.
- Automatic APN selection cannot guarantee that every SIM or mobile network will work.
- APN customization does not remove a SIM lock or network lock.
- A Wi-Fi client limit cannot exceed physical, memory, driver, or firmware constraints.
- Some devices do not expose a configurable Wi-Fi client limit.
- Changing DNS settings does not guarantee that every client or application will use those servers.
- USSD support depends on the modem, firmware, mobile network, modem interface, and available software stack.
- Some modems expose USSD through AT commands; others use different management interfaces or do not support it.
- A Windows desktop application cannot assume it can execute Linux router commands directly.
- The router's administration interface runs on the router, not on the Windows computer.
- Proprietary firmware signatures and secure boot can make modification impossible.

All feature availability must be determined from real evidence, not guessed from the device name.

---

# 5. TECHNOLOGY STACK

## 5.1 Desktop frontend

Use:

- React
- TypeScript
- Vite
- Tailwind CSS
- Lucide icons
- React Router or equivalent routing
- TanStack Query or an equivalent asynchronous state-management approach
- React Hook Form and schema validation where useful

Requirements:

- Strict TypeScript configuration.
- Reusable components.
- Accessible controls.
- Responsive layouts.
- Dark and light themes.
- Clear error states.
- Progress indicators.
- No hardcoded fake device data in production views.

## 5.2 Desktop backend

Use:

- Rust
- Tauri 2
- Tokio for asynchronous tasks where appropriate
- SQLite for persistent application metadata
- SHA-256 for firmware artifact integrity
- Typed Tauri IPC commands and events
- Windows device enumeration APIs
- A carefully isolated network and device communication layer

Use Windows-native functionality where practical.

Do not invoke arbitrary shell commands with unsanitized user input.

## 5.3 Router-side software

For supported OpenWrt devices, investigate and implement a small optional router-side package consisting of:

- A backend service or adapter.
- UCI integration.
- Modem interface integration where available.
- A local administration API.
- An authenticated LuCI integration or equivalent router-local web interface.
- Feature-specific configuration and verification functions.

Use native OpenWrt conventions and current supported APIs.

Avoid inventing configuration options or assuming all OpenWrt versions behave identically.

## 5.4 Optional analysis/build worker

Use WSL2 or another isolated Linux environment when required for:

- OpenWrt image building.
- Firmware image analysis.
- Supported filesystem inspection.
- Integration testing.
- Toolchains unavailable natively on Windows.

Do not require WSL2 merely to launch the desktop application.

---

# 6. HIGH-LEVEL SYSTEM ARCHITECTURE

Implement a modular system with the following layers.

## Layer 1 — Desktop user interface

Responsible for:

- Device discovery screens.
- Device information.
- Compatibility reporting.
- Customization forms.
- Change previews.
- Backup management.
- Firmware library.
- Job progress.
- Logs and activity.
- Settings.

## Layer 2 — Tauri IPC boundary

Responsible for:

- Validated frontend/backend communication.
- Typed request and response objects.
- Long-running job events.
- Error propagation.
- Permission boundaries.

## Layer 3 — Rust application core

Responsible for:

- Device discovery orchestration.
- Device identity resolution.
- Capability evaluation.
- Device profile matching.
- Configuration planning.
- Change transactions.
- Backup and rollback.
- Firmware artifact management.
- Persistent storage.
- Audit logging.

## Layer 4 — Device adapters

Responsible for actual device communication.

Examples:

- Mock router adapter.
- OpenWrt SSH adapter.
- OpenWrt authenticated RPC adapter.
- Supported vendor API adapter.
- ModemManager adapter.
- Device-specific USSD adapter.

Every adapter must declare what it supports.

## Layer 5 — Router-side components

Responsible for device-local functionality when needed.

Examples:

- OpenWrt UCI integration.
- Wi-Fi configuration.
- WAN/APN configuration.
- DNS configuration.
- USSD session handling.
- Local web administration page.

## Layer 6 — Firmware analysis and build workers

Responsible for:

- User-supplied firmware import.
- Hashing and validation.
- Safe inspection.
- Supported OpenWrt image builds.
- Build artifact provenance.

Keep this layer separate from configuration-only customization.

---

# 7. REPOSITORY STRUCTURE

Start with the existing repository if one exists.

Otherwise use a structure similar to:

- `README.md`
- `AGENTS.md`
- `RFS_MASTER_PLAN.md`
- `package.json`
- `apps/desktop/`
- `apps/desktop/src/app/`
- `apps/desktop/src/components/`
- `apps/desktop/src/features/dashboard/`
- `apps/desktop/src/features/devices/`
- `apps/desktop/src/features/customization/`
- `apps/desktop/src/features/customization/apn/`
- `apps/desktop/src/features/customization/wifi-clients/`
- `apps/desktop/src/features/customization/dns/`
- `apps/desktop/src/features/customization/ussd/`
- `apps/desktop/src/features/firmware-library/`
- `apps/desktop/src/features/backups/`
- `apps/desktop/src/features/activity/`
- `apps/desktop/src/features/settings/`
- `apps/desktop/src/services/`
- `apps/desktop/src/types/`
- `apps/desktop/src-tauri/src/commands/`
- `apps/desktop/src-tauri/src/discovery/`
- `apps/desktop/src-tauri/src/identity/`
- `apps/desktop/src-tauri/src/profiles/`
- `apps/desktop/src-tauri/src/capabilities/`
- `apps/desktop/src-tauri/src/adapters/`
- `apps/desktop/src-tauri/src/customization/`
- `apps/desktop/src-tauri/src/customization/apn/`
- `apps/desktop/src-tauri/src/customization/wifi/`
- `apps/desktop/src-tauri/src/customization/dns/`
- `apps/desktop/src-tauri/src/customization/ussd/`
- `apps/desktop/src-tauri/src/transactions/`
- `apps/desktop/src-tauri/src/backups/`
- `apps/desktop/src-tauri/src/firmware/`
- `apps/desktop/src-tauri/src/database/`
- `apps/desktop/src-tauri/src/jobs/`
- `apps/desktop/src-tauri/src/security/`
- `apps/desktop/src-tauri/migrations/`
- `device-profiles/schema/`
- `device-profiles/openwrt/`
- `device-profiles/vendor/`
- `router-packages/openwrt/rfs-agent/`
- `router-packages/openwrt/luci-app-rfs/`
- `workers/firmware-analysis/`
- `workers/openwrt-builder/`
- `tests/fixtures/`
- `tests/simulators/`
- `tests/integration/`
- `docs/architecture/`
- `docs/devices/`
- `docs/security/`
- `docs/testing/`
- `docs/releases/`

Adjust the layout if the existing codebase already uses a coherent architecture.

Avoid unnecessary monorepo complexity.

---

# 8. DEVICE DISCOVERY ENGINE

## 8.1 Windows USB discovery

Implement real Windows device enumeration.

Investigate and use suitable native Windows APIs for:

- Plug-and-Play devices.
- USB device identity.
- USB vendor and product identifiers.
- Serial interfaces.
- Network adapters created by USB-connected routers.
- RNDIS, CDC-NCM, MBIM, and related device classes when exposed by Windows.

Potential Windows API families include SetupAPI, Configuration Manager APIs, and network interface APIs.

PowerShell/CIM may be used as a controlled fallback, but not as the only architecture.

## 8.2 Network discovery

Detect likely directly connected router gateways.

Gather evidence from:

- Active network interfaces.
- Default gateway information.
- Interface configuration.
- Known device addresses.
- User-provided router addresses.
- Explicitly authorized device endpoints.

Do not perform unrestricted network scans.

Do not assume the default gateway is the user's target router.

## 8.3 Device identity

Capture:

- Device identifier.
- Manufacturer.
- Product/model.
- Hardware revision.
- USB VID/PID if applicable.
- Serial number when available and appropriate.
- Network interface.
- Gateway address.
- Router management address.
- Firmware version.
- Operating system.
- Modem model if available.
- Flash/storage information when reliably available.
- Evidence source.
- Confidence level.
- Last detected timestamp.

Distinguish discovered facts from inferred information.

## 8.4 Discovery UI

Provide:

- Rescan button.
- Connection status.
- Device cards.
- USB/network connection labels.
- Unknown-device handling.
- Manual router address entry.
- Troubleshooting messages.
- Permission and authentication prompts when needed.

---

# 9. DEVICE PROFILE AND CAPABILITY ENGINE

Create a versioned, data-driven profile system.

A device profile should contain:

- Profile ID and schema version.
- Manufacturer and model.
- Hardware revisions supported.
- Matching identifiers.
- Firmware family.
- Supported firmware versions.
- Operating system.
- Router management protocols.
- Modem interface types.
- Feature capabilities.
- Supported customization delivery modes.
- Backup support.
- Verification support.
- Rollback support.
- Known limitations.
- Evidence and source references.
- Profile maintainer/version.
- Safety constraints.

Each feature must return a structured decision:

- `supported`
- `unsupported`
- `unknown`
- `requires_authentication`
- `requires_router_package`
- `requires_firmware_build`
- `requires_manual_verification`

Include a human-readable reason.

Examples:

**APN:** Supported through OpenWrt WAN configuration.

**Wi-Fi limit:** Unknown because the driver capability has not been verified.

**DNS:** Supported for upstream DNS but LAN DHCP DNS behavior requires separate configuration.

**USSD:** Unsupported because the detected modem interface does not expose a verified USSD mechanism.

Never infer that all four features work just because the router runs OpenWrt.

---

# 10. THE FOUR CUSTOMIZATION MODULES

## MODULE A — APN CUSTOMIZATION

### Objective

Allow the user to choose between automatic operator-based APN configuration and manual APN configuration.

### Required UI

Create an APN customization screen with:

- Current APN.
- Current connection state.
- Detected operator when available.
- SIM/operator identity evidence.
- Automatic APN toggle.
- Manual APN fields.
- Authentication type.
- Username/password fields only when required.
- PDP/IP family options where supported.
- Save and apply.
- Preview changes.
- Backup and restore.
- Verification results.

### Automatic APN behavior

When supported:

1. Read operator identifiers, such as MCC/MNC, from an authorized modem management interface.
2. Consult an appropriately maintained APN database or supported system-provided APN information.
3. Select a plausible APN profile.
4. Explain the chosen source and confidence.
5. Permit manual override.
6. Preserve the previous working APN.
7. Apply through the device-specific supported interface.
8. Restart or reconnect the cellular data session only if necessary.
9. Verify registration and data connectivity where possible.
10. Offer rollback on failure.

Do not promise that automatic APN configuration makes a router compatible with every SIM.

### Manual APN behavior

Allow users to specify a supported APN configuration.

Validate inputs and redact secrets from logs.

### Important edge cases

Handle:

- SIM not inserted.
- SIM locked by PIN.
- No cellular modem detected.
- Unsupported operator.
- Missing APN database entry.
- Multiple APN profiles.
- Roaming.
- IPv4/IPv6 mismatch.
- Carrier-specific authentication.
- Router connection loss during WAN restart.
- Carrier restrictions unrelated to APN.

**Acceptance criterion:** A supported simulated OpenWrt device can read, preview, apply, verify, and restore APN settings; a real-device implementation is enabled only after hardware-specific validation.

---

## MODULE B — MAXIMUM WI-FI USERS

### Objective

Allow the user to configure a supported maximum number of associated Wi-Fi clients.

### Required UI

Display:

- Wi-Fi radios.
- SSIDs/BSS instances.
- Current connected-client count when available.
- Current configured limit.
- Supported configurable range.
- Hardware/driver limitations.
- New desired limit.
- Potential interruption warning.
- Apply and rollback controls.

### Implementation considerations

Investigate:

- hostapd station limits, including `max_num_sta`.
- OpenWrt wireless configuration and the relevant version-specific options.
- Per-radio versus per-BSS behavior.
- Driver limitations.
- Memory and performance constraints.
- Vendor-specific station-limit mechanisms.

Do not assume a single UCI setting works across all OpenWrt versions.

### Required workflow

1. Identify the active wireless configuration.
2. Determine whether a station limit is configurable.
3. Read the existing effective limit where possible.
4. Determine the valid range from actual capabilities.
5. Preview the exact configuration change.
6. Back up wireless configuration.
7. Apply using a supported mechanism.
8. Reload wireless services safely.
9. Verify the resulting configuration and service state.
10. Restore the previous configuration if verification fails.

Warn users that changing Wi-Fi configuration can disconnect the current management session.

A setting that accepts a number is not sufficient proof that the driver enforces it.

**Acceptance criterion:** The simulator demonstrates the complete workflow, and a supported real device passes a verified configuration and connectivity test.

---

## MODULE C — CUSTOM DNS

### Objective

Allow users to choose DNS servers used by the router and, separately where supported, DNS servers advertised to connected clients.

### Required UI

Provide:

- Current upstream DNS.
- Current DHCP-advertised LAN DNS.
- IPv4 DNS configuration.
- IPv6 DNS configuration where supported.
- Automatic ISP DNS toggle.
- Primary DNS field.
- Secondary DNS field.
- Optional additional DNS servers.
- DNS validation.
- Apply, test, and restore.

### Important distinction

Do not treat these as the same setting:

1. DNS servers used by the router for its own upstream resolution.
2. DNS servers advertised to LAN clients through DHCP or IPv6 mechanisms.
3. DNS forwarding/resolver behavior provided by services such as dnsmasq.
4. DNS settings that may be overridden by client applications or encrypted DNS.

### Implementation considerations

Investigate OpenWrt UCI, netifd, dnsmasq, DHCP, IPv6 RA/DHCPv6, and the actual firmware version.

### Required workflow

1. Read current DNS-related settings.
2. Identify supported upstream and LAN DNS controls.
3. Validate entered IPv4/IPv6 addresses.
4. Preview the configuration diff.
5. Back up the relevant settings.
6. Apply only the requested changes.
7. Reload affected services.
8. Verify configured resolver state.
9. Run safe DNS resolution checks.
10. Roll back when supported.

Do not claim that a successful DNS lookup proves every client uses the chosen DNS.

**Acceptance criterion:** Users can independently configure supported upstream and LAN DNS behavior, with verified persistence and recovery.

---

## MODULE D — USSD FUNCTIONALITY

### Objective

Allow users to run supported USSD codes through a secure administration page hosted on the router.

### Critical architectural requirement

The USSD administration interface must be available through the router's management address, such as a device-specific local IP or hostname.

It must not depend on the Windows desktop application being open after deployment.

### Required functionality

Where supported:

- Enter a USSD code.
- Submit the request.
- View the modem/network response.
- Handle a multi-step USSD session.
- Submit a follow-up response.
- Cancel an active session.
- Display timeouts and modem errors.
- Show whether the feature is supported.

### Modem integration

Research and implement only verified supported mechanisms, potentially including:

- ModemManager USSD interfaces.
- AT `+CUSD` through an authorized modem control interface.
- Supported QMI/MBIM-related interfaces where USSD functionality is actually exposed.
- Documented vendor modem APIs.

Do not assume every QMI or MBIM modem supports USSD.

Do not expose arbitrary AT-command execution through the web UI.

### Security requirements

Implement:

- Router administrator authentication.
- Session authorization.
- CSRF protection.
- Input validation.
- Request size limits.
- Rate limiting.
- Session timeouts.
- Protection against command injection.
- No unauthenticated WAN access.
- No default public exposure.
- Explicit confirmation for operations that may incur carrier charges.
- Minimal retention of sensitive USSD responses.

### USSD interface design

Create a router-local page containing:

- USSD input field.
- Send button.
- Current session status.
- Response display.
- Follow-up input when necessary.
- Cancel button.
- Supported-modem information.
- Clear errors.

The desktop application should also expose a **Manage USSD** action that opens the router-local interface.

**Acceptance criterion:** A simulated modem supports a complete USSD session; a supported real modem passes an authorized end-to-end test with authentication and security checks.

---

# 11. CONFIGURATION TRANSACTION ENGINE

Build one shared transaction engine used by APN, Wi-Fi, and DNS.

Every supported change must follow a controlled process:

1. Read current state.
2. Validate requested values.
3. Confirm device identity and profile.
4. Evaluate capability.
5. Generate a change plan.
6. Display an exact or semantically accurate preview.
7. Identify affected services and connectivity risks.
8. Create a verified backup.
9. Request user confirmation.
10. Apply the change.
11. Verify device reachability.
12. Verify relevant service state.
13. Verify feature-specific configuration.
14. Commit the transaction.
15. Roll back if a supported recovery mechanism is available and verification fails.
16. Record the result in the activity history.

Use transaction states such as:

- Draft.
- Validated.
- Awaiting approval.
- Backing up.
- Applying.
- Verifying.
- Completed.
- Failed.
- Rolling back.
- Rolled back.
- Recovery required.

Some routers may lose management connectivity during an apply operation.

Design reconnect logic and conservative timeouts.

Never claim rollback is guaranteed unless the device supports a reliable recovery mechanism.

---

# 12. BACKUP AND RECOVERY SYSTEM

Support different backup categories:

- Configuration snapshot.
- Relevant feature-specific configuration.
- Device metadata.
- User-imported firmware artifact.
- Supported full firmware backup only when a verified device-specific acquisition mechanism exists.

Do not label a settings backup as a full firmware backup.

Every backup should record:

- Device identity.
- Hardware revision.
- Firmware version.
- Timestamp.
- Backup type.
- Content hash.
- Creation method.
- Source and destination.
- Validation status.
- Restoration limitations.

Backups must be linked to the device and transaction that created them.

Do not restore a backup to a mismatched device without an explicit supported compatibility mechanism.

---

# 13. FIRMWARE LIBRARY AND CUSTOMIZATION BUILD ENGINE

Although the main product focuses on the four features, provide a firmware library and later a supported build pipeline.

## Firmware library

Allow importing user-supplied firmware files.

Implement:

- File selection.
- File size validation.
- Streamed SHA-256 hashing.
- Immutable content-addressed storage.
- Duplicate detection.
- Metadata persistence.
- Search and filtering.
- Integrity re-verification.
- JSON manifest export.
- Clear distinction between imported and device-acquired artifacts.

Treat firmware files as untrusted.

Do not execute firmware contents.

## Firmware analysis

Use isolated tools and environments for supported analysis.

Investigate Binwalk and other maintained firmware-analysis utilities.

Enforce:

- Input size limits.
- Timeouts.
- Resource limits.
- Safe extraction paths.
- No privileged execution of extracted scripts.
- No automatic execution of binaries from firmware images.

## OpenWrt build engine

Implement later, after configuration-only customization is functional.

Requirements:

- Exact device target selection.
- Supported hardware profile verification.
- OpenWrt version selection.
- Official image-building workflow.
- Package selection.
- Router-side RFS package inclusion.
- Configuration overlays.
- Build logs.
- Artifact hashing.
- Build manifests.
- Provenance and reproducibility metadata.
- Clear warnings about installation requirements.

Do not automatically flash generated images.

Do not build an image for an unknown hardware revision.

---

# 14. USER INTERFACE AND EXPERIENCE

Design a professional Windows desktop application.

Use a clean visual style with a sidebar, consistent spacing, clear typography, modern cards, and restrained accent colors.

## Required pages

### Dashboard

Show:

- Connected devices.
- Supported devices.
- Recent activity.
- Firmware library count.
- Running jobs.
- Quick actions.

### Devices

Show:

- Detected routers and MiFi devices.
- USB/network connection details.
- Device status.
- Identity evidence.
- Compatibility.
- Connection troubleshooting.

### Device Details

Show:

- Model and hardware revision.
- Firmware version.
- Connection method.
- Available management interfaces.
- Modem information.
- Four-feature support matrix.
- Backup status.
- Recent configuration changes.

### Customization Studio

Show four feature cards:

- APN Configuration.
- Wi-Fi User Limit.
- Custom DNS.
- USSD.

Each card must display its capability status and explain why it is available or unavailable.

### Configuration Preview

Show:

- Current value.
- Proposed value.
- Affected services.
- Potential connectivity interruption.
- Backup plan.
- Verification plan.
- Rollback availability.

### Firmware Library

Show imported firmware and metadata.

### Backups

Show configuration backups and supported restore actions.

### Activity

Show jobs, changes, warnings, failures, and recovery events.

### Settings

Show:

- Theme.
- Application data location.
- Discovery settings.
- Optional WSL2 integration.
- Logging preferences.
- Security preferences.
- Diagnostic export.

### Compatibility and Diagnostics

Show evidence for device identification and feature support.

Never present an unsupported feature as a working button.

---

# 15. CORE TYPES AND API CONTRACTS

Define shared Rust and TypeScript contracts.

At minimum, design the following entities:

## DeviceIdentity

Fields:

- `id`
- `manufacturer`
- `model`
- `hardwareRevision`
- `firmwareVersion`
- `operatingSystem`
- `connectionMethods`
- `managementEndpoints`
- `modemIdentity`
- `evidence`
- `confidence`
- `lastSeen`

## CapabilityDecision

Fields:

- `feature`
- `status`
- `reason`
- `evidence`
- `supportedDeliveryModes`
- `requiredAuthentication`
- `requiredPackages`
- `constraints`
- `verificationAvailable`
- `rollbackAvailable`

## CustomizationPlan

Fields:

- `planId`
- `deviceId`
- `feature`
- `currentConfiguration`
- `requestedConfiguration`
- `validatedChanges`
- `affectedServices`
- `connectivityRisk`
- `backupRequirements`
- `verificationSteps`
- `rollbackStrategy`

## ApplyTransaction

Fields:

- `transactionId`
- `planId`
- `status`
- `startedAt`
- `completedAt`
- `backupId`
- `progress`
- `verificationResults`
- `errors`
- `recoveryInstructions`

## FirmwareArtifact

Fields:

- `artifactId`
- `originalFilename`
- `sha256`
- `size`
- `source`
- `storagePath`
- `format`
- `importedAt`
- `integrityStatus`

## UssdSession

Fields:

- `sessionId`
- `deviceId`
- `status`
- `startedAt`
- `lastUpdatedAt`
- `response`
- `requiresFollowUp`
- `timeoutAt`

Do not store USSD secrets or responses unnecessarily.

## Device adapter interfaces

Define Rust traits or equivalent interfaces for:

- Device identity.
- Authentication.
- Capability probing.
- Reading configuration.
- Preparing a change.
- Applying a change.
- Verifying a change.
- Backing up configuration.
- Restoring configuration.
- Managing USSD sessions.

Feature adapters must be independently testable.

Do not force every device adapter to implement every feature.

## Suggested Tauri commands

- `discover_devices`
- `get_device_details`
- `connect_device`
- `disconnect_device`
- `get_device_capabilities`
- `get_feature_configuration`
- `validate_customization`
- `create_customization_plan`
- `approve_customization`
- `apply_customization`
- `get_transaction_status`
- `rollback_transaction`
- `list_backups`
- `create_backup`
- `restore_backup`
- `import_firmware`
- `list_firmware`
- `verify_firmware_integrity`
- `get_activity`
- `get_application_settings`

Expose long-running progress through typed events.

Use stable error codes and actionable error messages.

---

# 16. SQLITE DATABASE DESIGN

Create migrations for at least:

- `devices`
- `device_identifiers`
- `device_connections`
- `device_profiles`
- `capability_results`
- `firmware_artifacts`
- `configuration_snapshots`
- `customization_plans`
- `customization_transactions`
- `transaction_steps`
- `jobs`
- `activity_events`
- `application_settings`

Use foreign keys and indexes.

Separate immutable historical records from mutable current state.

Store credentials in an appropriate operating-system-protected credential mechanism, not plaintext SQLite fields.

Use structured logging and redact sensitive values.

---

# 17. SECURITY REQUIREMENTS

Implement security from the beginning.

Threats to consider:

- Malicious firmware files.
- Malicious router responses.
- Compromised or impersonated router endpoints.
- Untrusted USB devices.
- Command injection.
- Path traversal.
- Cross-site scripting.
- CSRF.
- Credential theft.
- Unauthorized router changes.
- Network exposure of the router-side service.
- Accidental connectivity loss.
- Unsafe firmware installation.
- Sensitive data leakage through logs.

Required controls:

- Validate all external input.
- Treat device metadata as untrusted.
- Pin or verify SSH host identity.
- Validate HTTPS certificates; do not silently disable verification.
- Request explicit approval for device-changing actions.
- Use least-privilege access.
- Never store router passwords in plaintext.
- Restrict router-local services to intended management interfaces.
- Protect USSD operations with authentication and CSRF controls.
- Do not expose a generic arbitrary-command terminal as part of the customization engine.
- Do not implement SIM unlocking, IMEI modification, signature bypass, or generic firmware flashing.
- Redact APN credentials, session tokens, router credentials, and sensitive modem data from logs.

---

# 18. IMPLEMENTATION PHASES

Follow these phases in order, unless existing repository work justifies a documented adjustment.

For every phase, provide:

- Goal.
- Files/modules created or modified.
- Implemented behavior.
- Automated tests.
- Build results.
- Manual verification instructions.
- Known limitations.
- Next phase.

Do not declare a phase complete unless its acceptance criteria pass.

## PHASE 0 — Repository and Environment Audit

**Goal:** Understand the current project before modifying it.

Tasks:

- Inspect all existing files.
- Read the master plan and agent instructions.
- Identify implemented and missing functionality.
- Inspect package configuration.
- Check Node.js, npm, Rust, Cargo, MSVC Build Tools, WebView2, and Windows SDK prerequisites.
- Check Git status.
- Determine whether Tauri already runs.
- Produce a gap analysis.
- Create or update `docs/PHASE_STATUS.md`.

**Acceptance criteria:**

- Existing work is preserved.
- Environment limitations are documented.
- A realistic implementation sequence is established.
- The agent can explain how to start the existing project.

## PHASE 1 — Desktop Application Foundation

**Goal:** Produce a functioning Windows desktop shell.

Tasks:

- Set up or repair Tauri 2.
- Configure Rust and TypeScript.
- Add Tailwind CSS.
- Implement routing.
- Add dark/light theme.
- Implement sidebar navigation.
- Create shared UI components.
- Add typed IPC infrastructure.
- Add error boundaries and loading states.
- Add frontend and Rust testing foundations.

**Acceptance criteria:**

- Application launches in development mode.
- Production build succeeds where prerequisites are available.
- Navigation works.
- Theme switching works.
- Frontend can invoke a real Rust command.
- Basic tests pass.

## PHASE 2 — Device Discovery

**Goal:** Detect actual connected device candidates.

Tasks:

- Implement Windows USB enumeration.
- Implement serial interface discovery.
- Implement network adapter and gateway discovery.
- Normalize discovered identifiers.
- Build a stable device list.
- Add bounded rescan behavior.
- Implement manual router address entry.
- Build the Devices page.
- Add mock discovery fixtures.

**Acceptance criteria:**

- Real Windows devices are listed.
- Mock discovery tests pass.
- Unknown devices remain explicitly unknown.
- No unauthorized broad network scanning occurs.

## PHASE 3 — Device Profiles and Capability Detection

**Goal:** Determine what each device can actually support.

Tasks:

- Create versioned profile schema.
- Implement matching logic.
- Implement confidence and evidence reporting.
- Create feature capability evaluator.
- Build support matrix UI.
- Add representative mock profiles.
- Test ambiguous hardware revisions.
- Ensure unknown capabilities never become supported by default.

**Acceptance criteria:**

- Device profile matching is deterministic.
- Feature statuses include reasons.
- Unknown devices cannot access unsupported write actions.
- Unit tests cover conflicting and incomplete evidence.

## PHASE 4 — Persistence, Jobs, and Firmware Library

**Goal:** Provide reliable application storage.

Tasks:

- Set up SQLite migrations.
- Implement device history.
- Implement job tracking.
- Implement structured activity logs.
- Implement firmware import.
- Implement streamed SHA-256.
- Implement immutable artifact storage.
- Implement deduplication.
- Implement integrity verification.
- Build Firmware Library and Activity pages.

**Acceptance criteria:**

- Imported firmware survives application restart.
- Duplicate artifacts are recognized.
- Corrupted artifacts fail integrity verification.
- Long-running jobs report progress.
- Cancellation and errors are handled.

## PHASE 5 — Device Adapter Framework

**Goal:** Separate device-specific behavior from the core application.

Tasks:

- Define adapter traits.
- Define connection and authentication contracts.
- Implement mock adapter.
- Implement adapter registry.
- Implement feature-specific capability probing.
- Add typed errors.
- Add adapter contract tests.

**Acceptance criteria:**

- New device adapters can be added without rewriting the UI.
- Mock adapters provide predictable responses.
- Unsupported operations return explicit errors.

## PHASE 6 — Router Simulator

**Goal:** Enable end-to-end development without risking physical devices.

Tasks:

- Build a simulated OpenWrt-like router.
- Simulate firmware and hardware identity.
- Simulate APN settings.
- Simulate Wi-Fi limits.
- Simulate DNS settings.
- Simulate USSD sessions.
- Simulate authentication.
- Simulate disconnections.
- Simulate failed apply operations.
- Simulate rollback failures and recoveries.

**Acceptance criteria:**

- The desktop application connects to the simulator.
- All four features display simulated current state.
- Failure cases are reproducible.
- Automated tests run without physical hardware.

## PHASE 7 — OpenWrt Read-Only Integration

**Goal:** Connect to a real, explicitly supported OpenWrt device without modifying it.

Tasks:

- Research current OpenWrt interfaces.
- Implement authorized SSH or authenticated RPC connection.
- Verify router identity.
- Read firmware version.
- Read supported UCI state.
- Read wireless configuration.
- Read DNS configuration.
- Read cellular WAN configuration when present.
- Detect modem/USSD support.
- Handle authentication failures.

**Acceptance criteria:**

- A supported OpenWrt device can be inspected.
- No persistent configuration is changed.
- Unsupported features remain disabled.
- Connection and identity verification are tested.

## PHASE 8 — Backup and Transaction Engine

**Goal:** Build safe apply/verify/rollback infrastructure before enabling configuration writes.

Tasks:

- Implement configuration snapshots.
- Implement change plans.
- Implement validation.
- Implement preview diffs.
- Implement approval state.
- Implement transaction execution.
- Implement post-apply verification.
- Implement reconnect logic.
- Implement rollback adapters.
- Implement recovery instructions.

**Acceptance criteria:**

- Simulated changes follow the complete transaction lifecycle.
- A failed apply does not appear successful.
- Supported rollback restores previous state.
- Unsupported rollback is clearly identified.
- No real device write occurs without explicit approval.

## PHASE 9 — DNS Customization MVP

**Goal:** Deliver the first complete customization feature.

Tasks:

- Implement DNS configuration UI.
- Implement upstream/LAN DNS distinction.
- Implement IPv4 and IPv6 validation.
- Implement OpenWrt DNS adapter for verified supported versions.
- Integrate backup and transaction engine.
- Verify resolver configuration.
- Test rollback.
- Add persistence tests.

**Acceptance criteria:**

- Full simulator workflow passes.
- Supported OpenWrt DNS changes can be previewed and verified.
- Affected services and limitations are explained.
- Failed changes produce recovery information.

## PHASE 10 — Wi-Fi Client Limit Customization

**Goal:** Implement verified Wi-Fi station-limit controls.

Tasks:

- Implement radio/SSID selection UI.
- Probe supported limits.
- Investigate hostapd and version-specific OpenWrt mappings.
- Implement supported configuration adapter.
- Handle Wi-Fi reloads.
- Implement connectivity warnings.
- Verify effective configuration.
- Test rollback and reconnect behavior.

**Acceptance criteria:**

- Invalid limits are rejected.
- Unsupported drivers cannot be configured.
- A supported real-device profile can apply and verify a limit.
- Connectivity risks are displayed before approval.

## PHASE 11 — APN Customization

**Goal:** Implement automatic and manual APN workflows.

Tasks:

- Implement APN UI.
- Read current cellular settings.
- Detect operator identifiers when available.
- Integrate a maintained APN data source.
- Implement automatic APN suggestions.
- Implement manual override.
- Implement supported OpenWrt cellular adapter.
- Preserve prior settings.
- Verify cellular connectivity where possible.
- Implement rollback and failure recovery.

**Acceptance criteria:**

- Automatic mode identifies its evidence and limitations.
- Manual mode works on supported devices.
- APN changes do not claim to unlock restricted SIMs.
- Simulated failures and restoration pass tests.

## PHASE 12 — USSD Backend

**Goal:** Implement a safe USSD service abstraction.

Tasks:

- Define USSD session state machine.
- Implement simulated modem.
- Research ModemManager and modem-specific interfaces.
- Implement supported modem adapter.
- Handle responses and interactive sessions.
- Handle cancellation and timeouts.
- Prevent arbitrary AT-command execution.
- Implement rate limits and validation.

**Acceptance criteria:**

- Simulated one-shot and interactive USSD sessions work.
- Unsupported modems are clearly rejected.
- Session errors and timeouts are handled.
- Security tests pass.

## PHASE 13 — Router-Local USSD Administration Page

**Goal:** Make USSD available through the router's own web interface.

Tasks:

- Create the optional OpenWrt RFS package.
- Implement secure router-local API endpoints.
- Implement administrator authentication integration.
- Implement CSRF protection.
- Restrict service exposure.
- Build LuCI integration or a justified equivalent.
- Add USSD form and response UI.
- Add session controls.
- Provide desktop link to the router-local page.

**Acceptance criteria:**

- The USSD page runs on the router.
- It remains functional when the desktop application is closed.
- Unauthorized requests fail.
- The interface is not exposed to the public WAN by default.
- A supported modem can complete an authorized USSD session.

## PHASE 14 — Complete Customization Studio Integration

**Goal:** Unify the four features into one polished experience.

Tasks:

- Implement consistent feature cards.
- Add capability status indicators.
- Add shared configuration forms.
- Add preview and confirmation flows.
- Add transaction progress.
- Add verification summaries.
- Add backup history.
- Add recovery instructions.
- Add contextual help.
- Complete empty, loading, error, and unsupported states.

**Acceptance criteria:**

- All four features use consistent UX patterns.
- Simulator end-to-end flows pass.
- No unsupported feature offers an active apply action.
- User can understand what changed and why.

## PHASE 15 — Supported OpenWrt Firmware Build Engine

**Goal:** Package supported customizations into compatible firmware builds where appropriate.

Tasks:

- Detect supported OpenWrt targets.
- Implement Image Builder integration.
- Validate hardware profile.
- Support optional router-side package inclusion.
- Generate configuration overlays.
- Record build provenance.
- Capture build logs.
- Hash output artifacts.
- Validate generated artifacts.
- Store builds in Firmware Library.

**Acceptance criteria:**

- A supported OpenWrt target produces a build artifact.
- Build logs and manifests are retained.
- Unsupported targets are rejected.
- No automatic flashing is performed.

## PHASE 16 — Proprietary Firmware Feasibility Engine

**Goal:** Determine whether selected proprietary devices have safe, supported customization pathways.

Tasks:

- Add isolated firmware metadata inspection.
- Implement model-specific vendor adapters only when justified.
- Identify known signing and packaging restrictions.
- Produce per-feature feasibility reports.
- Distinguish configuration APIs from firmware modifications.
- Record unsupported reasons.
- Document device-specific evidence.

**Acceptance criteria:**

- Imported proprietary images can be classified conservatively.
- The application does not invent modification capabilities.
- Signed or incompatible firmware is not offered as safely rebuildable.
- Unsupported devices receive actionable explanations.

## PHASE 17 — Hardware Validation

**Goal:** Validate supported functionality on real hardware.

Create a test matrix covering:

- Manufacturer and model.
- Hardware revision.
- Firmware version.
- Connection method.
- Management protocol.
- APN capability.
- Wi-Fi limit capability.
- DNS capability.
- USSD capability.
- Backup method.
- Rollback method.
- Verified test results.

For every supported device profile:

- Confirm identity.
- Confirm read-only access.
- Test each supported feature separately.
- Verify settings persist across relevant service restarts or reboots.
- Test connection interruptions.
- Test recovery.
- Document failures.
- Update profile evidence.

**Acceptance criteria:**

- Every claimed supported capability has reproducible test evidence.
- Unsupported combinations are blocked.
- Hardware-specific limitations are documented.

## PHASE 18 — Security, Reliability, and Performance Hardening

**Goal:** Prepare for production use.

Tasks:

- Threat-model review.
- Dependency audit.
- Input fuzzing where appropriate.
- Firmware parser hardening.
- Credential storage review.
- Router authentication review.
- USSD security review.
- Network timeout testing.
- Database migration testing.
- Backup corruption testing.
- UI accessibility review.
- Resource usage profiling.
- Crash recovery.
- Log redaction verification.

**Acceptance criteria:**

- Critical security findings are resolved.
- No sensitive credentials appear in logs.
- Failed operations remain recoverable where supported.
- Automated test suite passes.

## PHASE 19 — Windows Packaging and Release

**Goal:** Deliver an installable Windows desktop application.

Tasks:

- Configure production Tauri build.
- Add application icons and metadata.
- Produce supported Windows installer artifacts.
- Document prerequisites.
- Test clean installation.
- Test upgrade behavior.
- Test database migration.
- Test uninstall behavior.
- Test application data retention.
- Produce release notes.
- Document supported devices and features.
- Create user documentation.
- Prepare reproducible release builds and signing procedures where available.

**Acceptance criteria:**

- Installer works on supported Windows versions.
- Application launches after installation.
- No development environment is required for ordinary desktop usage.
- Core features work with simulator and verified hardware profiles.
- Supported-device list is accurate.

---

# 19. TESTING STRATEGY

Implement tests continuously rather than postponing them.

## Rust unit tests

Cover:

- Device identity parsing.
- Stable device identifiers.
- Profile matching.
- Capability decisions.
- Configuration validation.
- APN selection logic.
- Wi-Fi limit validation.
- DNS validation.
- USSD session state.
- Transaction transitions.
- Hashing.
- Backup metadata.
- Rollback logic.

## Frontend tests

Cover:

- Device list.
- Capability cards.
- Form validation.
- Unsupported states.
- Loading and error states.
- Confirmation dialogs.
- Transaction progress.
- USSD interaction.
- Accessibility.

## Integration tests

Cover:

- Tauri command contracts.
- SQLite migrations.
- Mock adapter behavior.
- Simulator communication.
- Firmware imports.
- Configuration transactions.
- Backup/restore flows.
- Router-side API behavior.

## End-to-end tests

Cover the full journey:

Discover → Identify → Check support → Authenticate → Read → Configure → Preview → Back up → Approve → Apply → Verify → Finish.

Test failures at every stage.

## Continuous integration

Create CI workflows for:

- TypeScript type checking.
- Frontend linting.
- Frontend tests.
- Rust formatting.
- Rust linting.
- Rust tests.
- Build verification.
- Dependency/security checks where practical.

Do not fabricate successful CI results.

---

# 20. DOCUMENTATION REQUIREMENTS

Maintain:

- `README.md`
- `AGENTS.md`
- `RFS_MASTER_PLAN.md`
- `docs/ARCHITECTURE.md`
- `docs/IMPLEMENTATION_ROADMAP.md`
- `docs/PHASE_STATUS.md`
- `docs/DEVICE_CAPABILITIES.md`
- `docs/DEVICE_PROFILE_SCHEMA.md`
- `docs/SECURITY.md`
- `docs/TESTING.md`
- `docs/OPENWRT_INTEGRATION.md`
- `docs/ROUTER_SIDE_USSD.md`
- `docs/FIRMWARE_BUILDING.md`
- `docs/HARDWARE_TEST_MATRIX.md`
- `docs/USER_GUIDE.md`

Documentation must reflect implemented behavior.

Do not mark proposed features as completed.

---

# 21. IMPLEMENTATION PRIORITIES

Use this priority order:

**Priority 1:** Working Windows desktop foundation.

**Priority 2:** Device discovery and identity.

**Priority 3:** Capability profiles and honest support reporting.

**Priority 4:** Reliable simulator, adapters, storage, and backups.

**Priority 5:** Configuration transaction engine.

**Priority 6:** DNS customization on supported OpenWrt devices.

**Priority 7:** Wi-Fi client-limit customization.

**Priority 8:** APN customization.

**Priority 9:** USSD service and router-local administration page.

**Priority 10:** Full end-to-end desktop integration.

**Priority 11:** Supported OpenWrt firmware building.

**Priority 12:** Proprietary firmware feasibility and model-specific integrations.

**Priority 13:** Hardware testing, security hardening, and release.

Do not let speculative proprietary firmware reverse engineering block a useful configuration-only MVP.

---

# 22. DEVELOPMENT RULES

When implementing each phase:

1. Inspect the relevant existing code.
2. Identify the smallest complete implementation.
3. Write the necessary Rust and TypeScript modules.
4. Connect the UI to actual backend functionality.
5. Add validation.
6. Add error handling.
7. Add tests.
8. Run formatting and linting.
9. Run the test suite.
10. Run the application or build.
11. Fix errors.
12. Update phase status.
13. Explain what works.
14. Continue to the next phase if no approval or hardware blocker exists.

Avoid:

- Fake device detections presented as real.
- Placeholder buttons that appear functional.
- Hardcoded successful responses.
- Arbitrary shell command execution.
- Unverified firmware writes.
- Generic flashing implementations.
- Assumptions that all OpenWrt devices share the same modem or Wi-Fi stack.
- Unsupported claims about proprietary MiFi firmware.
- Excessive dependencies without justification.
- Rewriting the project unnecessarily.

Use clear interfaces, structured errors, and testable modules.

---

# 23. DEFINITION OF DONE

The project is not complete merely because the UI looks finished.

The full project is complete when:

- A Windows installer can be built.
- The application launches reliably.
- Real USB/network device discovery works within documented limitations.
- Device identity and capability evaluation work.
- Unknown devices are handled safely.
- Firmware imports are stored and verified.
- Configuration backups are tracked.
- All four customization modules have functional UI and backend implementations.
- The simulator supports all four workflows.
- At least one supported OpenWrt configuration pathway has passed real hardware testing.
- Each feature advertised as supported has its own verified hardware evidence.
- The router-local USSD interface works on a verified compatible modem/router combination.
- Configuration changes require preview and confirmation.
- Apply/verify/rollback behavior is documented and tested.
- Supported OpenWrt image builds produce traceable artifacts.
- Unsupported proprietary firmware is handled honestly.
- Security and regression tests pass.
- Documentation matches reality.

Do not claim universal router or MiFi support.

---

# 24. REQUIRED AGENT PROGRESS REPORT

At the end of each phase, report using this format:

**Phase:** Number and name

**Status:** Complete / Partial / Blocked

**Implemented:** Actual working functionality

**Files changed:** Important files and directories

**Tests run:** Exact commands and results

**Build status:** Exact command and result

**Manual verification:** How I can test it in VS Code

**Limitations:** Missing hardware, permissions, dependencies, or unsupported capabilities

**Next phase:** What you will implement next

If a command could not run, say so explicitly.

Do not substitute a planned test for an executed test.

---

# 25. YOUR IMMEDIATE INSTRUCTIONS

Begin working on this project now.

First:

1. Inspect the complete current workspace.
2. Read all existing project plans and agent instructions.
3. Determine which phases have already been implemented.
4. Preserve existing files and user modifications.
5. Check the Windows development environment.
6. Create or update the implementation roadmap and phase tracker.
7. Identify the next incomplete phase.
8. Implement that phase with real code.
9. Add tests and run them.
10. Fix errors.
11. Verify that the desktop application builds or explain the exact blocker.
12. Continue through subsequent phases when safe and feasible.

**For the first implementation milestone, prioritize a functioning Tauri desktop application with device discovery, capability reporting, firmware library, and a working simulated customization engine.**

The simulator must eventually support all four features, but begin with a complete vertical slice rather than implementing four disconnected UI mockups.

Build the transaction engine before enabling real device configuration writes.

When reaching physical-router configuration, firmware installation, or other device-changing operations, stop and request explicit authorization for the specific device and action.

Do not stop after generating documentation.

Do not stop after creating UI mockups.

Do not report a feature as implemented unless its frontend, backend, and relevant tests are connected.

**The final goal is a real, installable, secure Windows application that makes supported router customization understandable, verifiable, and reversible wherever the device permits it.**

Start with the repository audit and proceed into implementation.