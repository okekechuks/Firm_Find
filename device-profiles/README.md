# Device profiles

Profiles describe exact hardware and firmware compatibility. They contain data, evidence references and prerequisites; they cannot execute commands or authorize changes.

| File | Status |
| --- | --- |
| `observed-comfi-mg11.json` | Local observation only. USB VID/PID are candidate hints; retail model, hardware revision and capabilities remain unverified. |
| `tplink-archer-c7-v5-openwrt.json` | Curated documentation reference for Archer C7 **v5**, OpenWrt **23.05.5** only. Historical target, not an installation recommendation or hardware-tested support claim. |
| `schema-v1.json` | Structural JSON schema generated from the frontend contract; the Rust validator additionally checks semantic constraints. |

The registry embeds only the two explicitly listed profiles. `schema-v1.json`, test fixtures and arbitrary files in this folder never become profiles automatically. A profile with unknown revision or firmware cannot become an exact match. Passing schema validation does not establish provenance.

Schema versions describe the document format. Profile versions describe compatibility content; increment the latter whenever content changes. Startup retains installed versions and quarantines changed content that reuses the same ID/version. Duplicate active IDs quarantine all duplicates.

Generate the schema with `node scripts/generate-profile-schema.mjs`. Tests use synthetic identity responses under the Rust/React test modules; they never turn into live device evidence. See [profile design](../docs/DEVICE_PROFILES.md) and [capability decisions](../docs/CAPABILITY_ENGINE.md).
