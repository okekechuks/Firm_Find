# OBTAINER master plan

**22 October 2026 — approved Phase 7.5 amendment:** stock router/MiFi identification and legitimate firmware acquisition now sit between Phases 7 and 8. Product workflow is connect → detect → identify → discover acquisition routes → obtain genuine bytes → Library → analyze/customize/build/validate → qualified install/verify/recover. Broad coverage is the goal; each capability requires its own evidence. Earlier configuration-first delivery order below is superseded where this approved workstream applies. Phases 0–19 are not renumbered. Phase 8 remains unstarted. [Architecture/review handoff](docs/PHASE_7_5_ARCHITECTURE.md), [implementation](docs/PHASE_7_5_IMPLEMENTATION.md), [verification](docs/PHASE_7_5_VERIFICATION.md).

Planning revision: 15 October 2026. Product name remains **OBTAINER**; RFS is the reference architecture name used in the supplied guideline.

This plan adopts the configuration-first direction and supersedes the phase ordering in the earlier product requirements. The desktop relocation was already committed before the corrective Phase 0 audit. The [technical audit](docs/PHASE_0_AUDIT.md) records the current implementation and environment; the existing structure is being retained. Planned modules below do not establish implemented functionality.

## Product scope

OBTAINER helps owners of supported routers and MiFi devices inspect actual capabilities and safely configure four features: DNS, Wi-Fi client limits, APN, and USSD. Compatibility is specific to hardware revision, firmware, driver, modem, and management interface.

The primary workflow is: discover or select an endpoint, establish identity, authenticate, read configuration, check support, prepare a preview, create a verified backup, obtain approval, apply, verify, and commit or recover. USSD uses its own session lifecycle; cancellation is not a configuration rollback.

The three delivery modes are:

| Mode | Purpose | Delivery order |
| --- | --- | --- |
| A: Configuration | Use an existing supported, authenticated management interface. | MVP priority. DNS first, then Wi-Fi limits, APN, and USSD. |
| B: Supported builds | Produce an OpenWrt image for an exact supported target with selected packages and overlays. | After configuration workflows work. Builds remain separate artifacts. |
| C: OEM feasibility | Inspect imported proprietary images and investigate documented device-specific configuration routes. | Later, without blocking Mode A. Report restrictions and unsupported features. |

The firmware library starts with local imports. The owner's Phase 7 requirements explicitly retain official acquisition, supported physical extraction, firmware customization, rebuilding, qualified installation, verification and recovery as permanent project goals. A configuration snapshot must never be presented as a firmware dump. Phase 7 remains inspection-only while the dedicated executable engines and device-specific gates are pending. See the proposed [firmware pipeline amendment](docs/FIRMWARE_PIPELINE_ROADMAP.md), which adds explicit acquisition/extraction and installation milestones without renumbering existing phases; placement is subject to owner approval. Generic unsupported flashing, signature bypass, SIM unlocking and IMEI modification remain outside scope.

## Planning documents

- [Source guideline](docs/reference/RFS_DEVELOPMENT_GUIDELINE.md): the supplied reference, preserved unchanged. Embedded prompts remain reference material; live user requests define execution scope.
- [Implementation roadmap](docs/IMPLEMENTATION_ROADMAP.md): phases 0–19, dependencies, deliverables, and exit gates.
- [Firmware pipeline amendment](docs/FIRMWARE_PIPELINE_ROADMAP.md): proposed acquisition/extraction and qualified installation gates, with existing Phases 15–17 retained.
- [Phase status](docs/PHASE_STATUS.md): actual implementation state and remaining work against the revised requirements.
- [Architecture](docs/ARCHITECTURE.md): current implementation, to be updated during implementation.
- [Verification evidence](docs/VERIFICATION.md): recorded commands and results; historical results are not new verification.

## Target repository structure

Use one desktop application, one Cargo application package, and a single root npm lockfile. Extract extra Rust crates or shared packages only when a real reuse boundary appears. The tree below is a destination, not a claim that the folders or features already exist.

```text
OBTAINER/
  README.md
  AGENTS.md                       # optional future repository workflow guidance
  RFS_MASTER_PLAN.md
  package.json                    # npm workspace and root command entry points
  package-lock.json
  Cargo.toml                      # member: apps/desktop/src-tauri
  Cargo.lock
  rust-toolchain.toml
  apps/desktop/
    package.json
    index.html
    vite.config.ts
    tsconfig.json
    public/
    src/
      app/
      components/
      features/
        dashboard/
        devices/
        customization/{dns,wifi-clients,apn,ussd}/
        firmware-library/
        backups/
        activity/
        settings/
        compatibility/
      services/
      types/
      test/
    src-tauri/
      src/
        commands/
        discovery/
        identity/
        profiles/
        capabilities/
        adapters/
        customization/{dns,wifi,apn,ussd}/
        transactions/
        backups/
        firmware/
        database/
        jobs/
        security/
      migrations/
      capabilities/
      permissions/
      icons/
      tauri.conf.json
  device-profiles/{schema,openwrt,vendor}/
  router-packages/openwrt/{obtainer-agent,luci-app-obtainer}/
  workers/{firmware-analysis,openwrt-builder}/
  tests/{fixtures,simulators,integration}/
  docs/{architecture,devices,security,testing,releases,reference}/
  scripts/
  .github/workflows/
```

Keep existing flat documentation paths working during the migration. Add the guideline's specialist documents when their owning phase produces a real design or implementation. Do not create empty modules or placeholder packages just to fill this tree.

| Documentation | Owning phase |
| --- | --- |
| `AGENTS.md`, `docs/TESTING.md` | 0: migrated commands and contribution/verification workflow |
| `docs/DEVICE_CAPABILITIES.md`, `docs/DEVICE_PROFILE_SCHEMA.md` | 3: profile contracts, decisions, and evidence rules |
| `docs/SECURITY.md` | 5 onward; final qualification in 18 |
| `docs/OPENWRT_INTEGRATION.md`, `docs/HARDWARE_TEST_MATRIX.md` | 7 onward; matrix qualification in 17 |
| `docs/ROUTER_SIDE_USSD.md` | 12–13: service, session, security, and deployment contracts |
| `docs/FIRMWARE_BUILDING.md` | 15: supported targets, workers, provenance, and output limits |
| `docs/USER_GUIDE.md`, release notes | Working features from 9 onward; release qualification in 19 |

## Migration strategy

The mechanical relocation below already exists in commit `2c07c61`. It is retained as history, not an instruction to move the source again. Any further move requires a demonstrated need and a data-preserving verification/rollback procedure. Phase 0 evaluates migration necessity; it does not require another relocation.

1. Inventory tracked and untracked work. Preserve the existing dirty worktree and the current application-data location. Record a baseline before moving source files.
2. Move `src/`, `src-tauri/`, `public/`, `index.html`, Vite configuration, and application TypeScript configuration under `apps/desktop/`. Keep root lockfiles, toolchain, `.tools/`, scripts, and CI entry points. Split root and desktop package responsibilities without adding another lockfile.
3. Update Cargo membership, npm workspaces, import/config paths, lint coverage, Vite/Tauri commands, ignored output, test fixtures, native smoke paths, and CI. Preserve root `npm run dev`, `build`, `lint`, `test`, `tauri`, and `test:native` entry points. Keep Cargo output under root `target/`.
4. Verify the mechanical relocation before changing behavior. Application identity, SQLite records, appearance, and stored data must survive; moving source must not reset user data.
5. Split existing Rust `commands.rs`, `domain.rs`, and `storage.rs` incrementally into the planned modules as their features arrive. Keep the working discovery service. Reorganize React features without replacing the application with another scaffold.

## Contracts and ownership

Rust owns identity evidence, profiles, capability decisions, secrets, plans, approvals, transactions, jobs, backups, and persistence. React renders validated responses and submits typed intents. Adapters own device-specific operations; analysis/build workers are separate from configuration adapters.

Feature decisions use the guideline's seven states: `supported`, `unsupported`, `unknown`, `requires_authentication`, `requires_router_package`, `requires_firmware_build`, and `requires_manual_verification`. Every decision includes a reason, evidence, constraints, delivery modes, and verification/rollback availability. Missing or conflicting evidence must not grant support.

The existing uppercase discovery capability enum is a baseline, not the new profile engine. Phase 3 migrates Rust and TypeScript together. Legacy `REQUIRES_TOOL` and `EXPERIMENTAL` must not automatically become package/build support; preserve their uncertainty until specific evidence resolves it. Inventory visibility and feature write eligibility remain separate concepts.

Adapters declare independent read, plan, apply, verify, backup, restore, and USSD capabilities. A read method does not imply a write method. Simulator identities and state are explicitly labelled and isolated from physical-device profiles and history.

## Transaction and device gates

- Bind approval to the exact device, profile/firmware identity, proposed change, and before-state. Revalidate stale plans before applying.
- Verify a backup before a configuration write. Store secret-bearing snapshots with appropriate protection; ordinary logs and SQLite metadata must not expose credentials.
- Persist progress and distinguish failure, rollback, and recovery-required outcomes. After a crash or reconnect, establish the actual device state before further actions.
- Require explicit approval for each physical-device change, router-package deployment, and chargeable USSD action. The current planning request authorizes none of those operations.
- Establish trusted endpoint identity and bounded communication. Manual endpoint entry identifies a candidate, not a verified device or permission to scan its network.
- Keep USSD administration on the router, authenticated and restricted to intended management interfaces. It must work with the desktop application closed.
- Research current primary documentation during the owning implementation phase before choosing OpenWrt keys, modem commands, vendor APIs, or tool versions. This planning revision does not assert those mappings.

## Delivery checkpoints

1. **Reliable baseline:** migrated structure, working launch, honest discovery and capability reporting (phases 0–3).
2. **Working development platform:** durable library/jobs, adapter contracts, and fault-injectable simulator (phases 4–6).
3. **First complete feature:** simulator DNS transaction end to end, then an authorized, validated OpenWrt DNS path (phases 7–9). Phase 7 hardware availability must not block phase 8 simulator work.
4. **Four-feature studio:** Wi-Fi limits, APN, USSD backend and router-local page, integrated consistently (phases 10–14).
5. **Extended tooling:** supported OpenWrt builds and conservative OEM feasibility (phases 15–16).
6. **Release evidence:** device/feature validation, hardening, and Windows release qualification (phases 17–19). Hardware and security evidence must also be collected throughout development.
