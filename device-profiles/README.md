# Device profiles

Profiles describe exact hardware and firmware compatibility. They contain data, evidence references and prerequisites; they cannot execute commands or authorize changes.

| File | Status |
| --- | --- |
| `observed-comfi-mg11.json` | Local observation only. USB VID/PID are candidate hints; retail model, hardware revision and capabilities remain unverified. |
| `tplink-archer-c7-v5-openwrt.json` | Curated documentation reference for Archer C7 **v5**, OpenWrt **23.05.5** only. Historical target, not an installation recommendation or hardware-tested support claim. |
| `schema-v1.json` | Structural JSON schema generated from the frontend contract; the Rust validator additionally checks semantic constraints. |
| `schema-v2.json` | Separate inspection-only OpenWrt OS schema. Requires exact firmware and board/target/build identity; hardware feature assessments remain UNKNOWN. |

The production registry embeds only the two explicitly listed profiles. Schemas, test fixtures and arbitrary files in this folder never become profiles automatically. Schema-1 hardware profiles require exact hardware/firmware evidence. Schema-2 OS-only inspection can leave physical manufacturer/revision unknown but still requires an exact authenticated OS identity; it grants no hardware feature or write support. Passing schema validation does not establish provenance. The qualified QEMU profile exists only in explicit isolated native-test mode.

Schema versions describe the document format. Profile versions describe compatibility content; increment the latter whenever content changes. Startup retains installed versions and quarantines changed content that reuses the same ID/version. Duplicate active IDs quarantine all duplicates.

Generate the schema with `node scripts/generate-profile-schema.mjs`. Tests use synthetic identity responses under the Rust/React test modules; they never turn into live device evidence. See [profile design](../docs/DEVICE_PROFILES.md) and [capability decisions](../docs/CAPABILITY_ENGINE.md).
