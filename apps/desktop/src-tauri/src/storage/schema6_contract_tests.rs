//! Exercise the actual migration asset in memory, including SQLite conflict rules.
use super::*;
use rusqlite::types::Value;

fn contract() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    for sql in [
        include_str!("../../migrations/001_foundation.sql"),
        include_str!("../../migrations/002_discovery.sql"),
        include_str!("../../migrations/003_profiles.sql"),
        include_str!("../../migrations/004_firmware_library.sql"),
        include_str!("../../migrations/005_desktop_workspace.sql"),
        include_str!("../../migrations/006_artifact_classes.sql"),
    ] {
        connection.execute_batch(sql).unwrap();
    }
    connection
}

#[test]
fn stage3_sql_contract_rejects_null_identity_and_noncanonical_hash_storage() {
    let connection = contract();
    let hash = "a".repeat(64);
    connection
        .execute("INSERT INTO artifact_blobs VALUES(?1,4)", [&hash])
        .unwrap();
    let mut accepted = vec![];
    for (name, value) in [
        ("null_blob", Value::Null),
        ("binary_hash", Value::Blob(vec![0; 64])),
        (
            "nul_suffix_hash",
            Value::Text(format!("{}\0suffix", "b".repeat(64))),
        ),
    ] {
        if connection
            .execute("INSERT INTO artifact_blobs VALUES(?1,4)", [value])
            .is_ok()
        {
            accepted.push(name);
        }
    }
    if connection
        .execute(
            "INSERT INTO artifact_blob_state VALUES(NULL,'synthetic','unknown','not_checked',NULL)",
            [],
        )
        .is_ok()
    {
        accepted.push("null_state");
    }
    if connection.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,acquisition_occurrence_id,collected_at) VALUES(NULL,?1,'metadata_report','synthetic','null-id',1)", [&hash]).is_ok() { accepted.push("null_occurrence"); }
    for (name, value) in [
        ("binary_manifest", Value::Blob(vec![0; 64])),
        (
            "nul_manifest",
            Value::Text(format!("{}\0suffix", "c".repeat(64))),
        ),
    ] {
        if connection.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,acquisition_occurrence_id,collected_at,coverage_total_bytes,coverage_observed_bytes,coverage_complete,coverage_manifest_sha256) VALUES(?1,?2,'physical_flash_dump','synthetic',?1,1,4,4,1,?3)", params![name,hash,value]).is_ok() { accepted.push(name); }
    }
    for (name, value) in [
        ("binary_backup_hash", Value::Blob(vec![0; 64])),
        (
            "nul_backup_hash",
            Value::Text(format!("{}\0suffix", "d".repeat(64))),
        ),
    ] {
        let backup_contract = contract();
        if backup_contract.execute("INSERT INTO artifact_schema_receipt VALUES(6,5,'schema-before-6-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.sqlite',?1,1,1)", [value]).is_ok() { accepted.push(name); }
    }
    assert!(
        accepted.is_empty(),
        "invalid SQL identities/hashes accepted: {accepted:?}"
    );
}

#[test]
fn stage3_sql_contract_replace_cannot_reclassify_or_rewrite_immutable_history() {
    let mut accepted = vec![];
    for recursive in [false, true] {
        for case in [
            "blob",
            "occurrence",
            "acquisition_id",
            "admission",
            "firmware",
            "receipt",
        ] {
            let connection = contract();
            connection
                .pragma_update(None, "recursive_triggers", recursive)
                .unwrap();
            let hash = "a".repeat(64);
            connection
                .execute("INSERT INTO artifact_blobs VALUES(?1,4)", [&hash])
                .unwrap();
            for (id, class, provenance) in [
                ("config", "configuration_backup", "synthetic"),
                ("user", "user_imported_unverified", "user_import"),
                ("official", "official_image_download", "synthetic"),
            ] {
                connection.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,acquisition_occurrence_id,collected_at) VALUES(?1,?2,?3,?4,?1,1)",params![id,hash,class,provenance]).unwrap();
            }
            connection
                .execute(
                    "INSERT INTO firmware_admissions VALUES(?1,'user','user_imported_unverified')",
                    [&hash],
                )
                .unwrap();
            connection
                .execute("INSERT INTO firmware_artifacts VALUES(?1,4)", [&hash])
                .unwrap();
            connection
                .execute(
                    "INSERT INTO artifact_schema_receipt VALUES(6,NULL,NULL,NULL,NULL,1)",
                    [],
                )
                .unwrap();
            let result = match case {
                "blob" => connection.execute("INSERT OR REPLACE INTO artifact_blobs VALUES(?1,5)", [&hash]),
                "occurrence" => connection.execute("INSERT OR REPLACE INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,acquisition_occurrence_id,collected_at) VALUES('config',?1,'official_image_download','synthetic','config',2)", [&hash]),
                "acquisition_id" => connection.execute("INSERT OR REPLACE INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,acquisition_occurrence_id,collected_at) VALUES('replacement',?1,'official_image_download','synthetic','config',2)", [&hash]),
                "admission" => connection.execute("INSERT OR REPLACE INTO firmware_admissions VALUES(?1,'official','official_image_download')", [&hash]),
                "firmware" => connection.execute("INSERT OR REPLACE INTO firmware_artifacts VALUES(?1,4)", [&hash]),
                _ => connection.execute("INSERT OR REPLACE INTO artifact_schema_receipt VALUES(6,NULL,NULL,NULL,NULL,2)", []),
            };
            if result.is_ok() {
                accepted.push(format!("{case}/recursive={recursive}"));
            }
        }
    }
    assert!(
        accepted.is_empty(),
        "immutable SQL identities replaced: {accepted:?}"
    );
}
