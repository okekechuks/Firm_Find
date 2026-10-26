//! Proposed manifest-2 wire contract only. No export or producer consumer is
//! wired before Q5 review; manifest-1 and its existing export remain unchanged.
#![allow(dead_code)]
use super::types::{valid_hash, ArtifactClass};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Provenance {
    UserImport,
    OfficialVendorDownload,
    AuthorizedDeviceAcquisition,
    LegacyUnclassified,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Coverage {
    total_bytes: u64,
    observed_bytes: u64,
    complete: bool,
    manifest_sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Occurrence {
    artifact_class: ArtifactClass,
    provenance: Provenance,
    collected_at: u64,
    coverage: Option<Coverage>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestV2 {
    manifest_schema_version: u8,
    sha256: String,
    size_bytes: u64,
    artifact_class: ArtifactClass,
    occurrences: Vec<Occurrence>,
    omitted_occurrence_count: u64,
    installability: Installability,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Installability {
    Unknown,
}
impl ManifestV2 {
    fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 64 * 1024 {
            return Err("manifest exceeds limit");
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| "invalid manifest contract")?;
        if value.manifest_schema_version != 2
            || !valid_hash(&value.sha256)
            || value.occurrences.len() > 50
        {
            return Err("invalid manifest contract");
        }
        for item in &value.occurrences {
            if item.artifact_class == ArtifactClass::UserImportedUnverified
                && !matches!(item.provenance, Provenance::UserImport)
            {
                return Err("provenance conflict");
            }
            if let Some(c) = &item.coverage {
                if item.artifact_class != ArtifactClass::PhysicalFlashDump
                    || c.observed_bytes > c.total_bytes
                    || c.manifest_sha256.as_deref().is_some_and(|s| !valid_hash(s))
                    || (c.complete
                        && (c.total_bytes == 0
                            || c.observed_bytes != c.total_bytes
                            || c.manifest_sha256.is_none()))
                {
                    return Err("invalid dump coverage");
                }
            }
        }
        Ok(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> serde_json::Value {
        serde_json::json!({"manifestSchemaVersion":2,"sha256":"a".repeat(64),"sizeBytes":4,"artifactClass":"user_imported_unverified","occurrences":[{"artifactClass":"user_imported_unverified","provenance":"user_import","collectedAt":1,"coverage":null}],"omittedOccurrenceCount":0,"installability":"unknown"})
    }
    #[test]
    fn stage3_manifest_two_contract_rejects_identity_paths_authority_and_bad_coverage() {
        let original = fixture();
        let parsed = ManifestV2::parse(&serde_json::to_vec(&original).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), original);
        for key in [
            "localPath",
            "deviceId",
            "description",
            "credentials",
            "imei",
            "sourceReference",
        ] {
            let mut bad = fixture();
            bad[key] = "sentinel-private-value".into();
            assert!(ManifestV2::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
            let mut bad = fixture();
            bad["occurrences"][0][key] = "sentinel-private-value".into();
            assert!(ManifestV2::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        for (key, v) in [
            ("manifestSchemaVersion", serde_json::json!(1)),
            ("installability", serde_json::json!("verified")),
            ("sha256", serde_json::json!("A".repeat(64))),
        ] {
            let mut bad = fixture();
            bad[key] = v;
            assert!(ManifestV2::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        let mut bad = fixture();
        bad["occurrences"][0]["coverage"] = serde_json::json!({"totalBytes":4,"observedBytes":4,"complete":true,"manifestSha256":"a".repeat(64)});
        assert!(ManifestV2::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        bad["occurrences"][0]["artifactClass"] = "physical_flash_dump".into();
        assert!(ManifestV2::parse(&serde_json::to_vec(&bad).unwrap()).is_ok());
        bad["occurrences"][0]["coverage"]["observedBytes"] = serde_json::json!(3);
        assert!(ManifestV2::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
}
