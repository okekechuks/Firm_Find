//! Metadata assessments are never firmware artifacts, backups or permission to write.
use super::query::{Board, Partition};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    RequiresVerification,
    RequiresTool,
    Unsupported,
    Unknown,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Assessment {
    pub status: Status,
    pub reason: String,
    pub evidence: Vec<String>,
    pub missing_requirements: Vec<String>,
    pub executable: bool,
}
impl Assessment {
    fn unknown(reason: &str, missing: &[&str]) -> Self {
        Self {
            status: Status::Unknown,
            reason: reason.into(),
            evidence: vec![],
            missing_requirements: missing.iter().map(|s| (*s).into()).collect(),
            executable: false,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomizationPath {
    pub direct_configuration: Assessment,
    pub firmware_rebuild: Assessment,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareFeasibilityReport {
    pub device_identity: Board,
    pub acquisition: Assessment,
    pub extraction: Assessment,
    pub partition_layout: Assessment,
    pub partitions: Vec<Partition>,
    pub image_format: Assessment,
    pub customization: BTreeMap<String, CustomizationPath>,
    pub build: Assessment,
    pub installation: Assessment,
    pub recovery: Assessment,
    pub restrictions: Vec<String>,
    pub last_observed: u64,
    pub confidence: String,
}

/// `board` must come from the authenticated query. The report itself grants no authority.
pub fn evaluate(
    board: Board,
    partitions: Option<Vec<Partition>>,
    now: u64,
) -> FirmwareFeasibilityReport {
    let mut acquisition = Assessment::unknown(
        "No exact compatible official artifact has been verified.",
        &[
            "Exact board and hardware revision",
            "Official image manifest and integrity/authenticity verification",
        ],
    );
    let mut build = Assessment::unknown(
        "No qualified build target is established for this device.",
        &[
            "Exact build profile",
            "Package and overlay requirements",
            "Output validation",
        ],
    );
    let mut image_format = Assessment::unknown(
        "An installed firmware version does not identify an installable image format.",
        &[
            "Exact image type and board mapping",
            "Signature and encryption requirements",
            "Size constraints",
        ],
    );
    // Only the researched release/target yields this candidate source. Server data is never a URL.
    if board.release.distribution == "OpenWrt"
        && board.release.version == "25.12.6"
        && board.release.target == "x86/64"
    {
        let source = "https://downloads.openwrt.org/releases/25.12.6/targets/x86/64/".to_string();
        acquisition.status = Status::RequiresVerification;
        acquisition.reason = "Official x86/64 release images exist. Exact board, BIOS/EFI and layout compatibility remain unqualified; no firmware was acquired by this report.".into();
        acquisition.evidence.push(source.clone());
        build.status = Status::RequiresTool;
        build.reason = "Official matching ImageBuilder exists; the OBTAINER build engine is not implemented and no customized image has been built.".into();
        build.evidence.push(source.clone());
        image_format.status = Status::RequiresVerification;
        image_format.reason = "Official x86 images distinguish ext4/squashfs and BIOS/EFI. The running root filesystem alone does not choose a compatible installation image.".into();
        image_format.evidence.push(source);
    }
    let partition_layout = match &partitions {
        Some(rows) => Assessment {
            status: Status::RequiresVerification,
            reason: format!("{} MTD metadata entries observed. Completeness, calibration secrets, dual-bank semantics and installability are unverified.", rows.len()),
            evidence: vec!["Authenticated /proc/mtd metadata query".into()],
            missing_requirements: vec!["Device-specific partition map".into(), "Qualified recovery procedure".into()],
            executable: false,
        },
        None => Assessment::unknown(
            "MTD metadata unavailable; this does not establish missing storage or extraction support.",
            &["Reviewed platform-specific storage metadata source"],
        ),
    };
    let customization = ["dns", "wifiUserLimit", "apn", "ussd"].into_iter().map(|feature| (
        feature.into(),
        CustomizationPath {
            direct_configuration: Assessment::unknown(
                "Readability does not establish a qualified configuration transaction.",
                &["Feature-specific verified device support", "Backup, preview, apply, verification and restore implementation"],
            ),
            firmware_rebuild: Assessment::unknown(
                "No feature-specific rebuilt image or package integration has been qualified.",
                &["Compatible builder", "Required hardware/driver/modem evidence", "Qualified installation and recovery"],
            ),
        }
    )).collect();
    FirmwareFeasibilityReport {
        device_identity: board,
        acquisition,
        extraction: Assessment::unknown(
            "No qualified physical extraction method. Partition metadata is not firmware bytes or an installable backup.",
            &["Device-specific authorized read method", "Coverage and integrity validation", "Secret-bearing partition policy"],
        ),
        partition_layout,
        partitions: partitions.unwrap_or_default(),
        image_format,
        customization,
        build,
        installation: Assessment::unknown(
            "No qualified installation route; the physical installation engine is not implemented.",
            &["Exact board/image compatibility", "Protected backups", "Power and connection-loss handling", "Explicit approval", "Recovery qualification"],
        ),
        recovery: Assessment::unknown(
            "Recovery and rollback have not been tested on this device.",
            &["Device-specific documented procedure", "Required physical access/tools", "Hardware recovery evidence"],
        ),
        restrictions: vec![
            "Inspection only; no configuration changes, raw partition reads, flashing or live USSD.".into(),
            "A privileged SSH account may be capable of writing; OBTAINER restricts its command dispatch.".into(),
            "Authenticated server reports are not independent physical-hardware certification.".into(),
            "This metadata report is not a firmware image or backup.".into(),
        ],
        last_observed: now,
        confidence: "Authenticated server metadata; device-specific feasibility requires independent qualification".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn board() -> Board {
        Board::parse(br#"{"model":"QEMU","board_name":"qemu","release":{"distribution":"OpenWrt","version":"25.12.6","revision":"r1","target":"x86/64"}}"#).unwrap()
    }
    #[test]
    fn exact_release_target_has_candidate_route_but_never_executable_support() {
        let report = evaluate(board(), None, 42);
        assert_eq!(report.acquisition.status, Status::RequiresVerification);
        assert_eq!(report.build.status, Status::RequiresTool);
        assert!(!report.acquisition.executable);
        assert_eq!(report.recovery.status, Status::Unknown);
        assert_eq!(report.installation.status, Status::Unknown);
        assert!(report
            .customization
            .values()
            .all(|p| p.direct_configuration.status == Status::Unknown
                && p.firmware_rebuild.status == Status::Unknown));
    }
    #[test]
    fn wrong_release_target_or_claim_never_generates_official_source() {
        for (version, target, family) in [
            ("25.12.5", "x86/64", "OpenWrt"),
            ("25.12.6", "ath79/generic", "OpenWrt"),
            ("25.12.6", "x86/64", "Vendor"),
        ] {
            let mut value = board();
            value.release.version = version.into();
            value.release.target = target.into();
            value.release.distribution = family.into();
            let report = evaluate(value, None, 1);
            assert!(report.acquisition.evidence.is_empty());
            assert_eq!(report.build.status, Status::Unknown);
        }
    }
    #[test]
    fn partition_metadata_does_not_grant_extraction_or_infer_signature_encryption() {
        let entries = vec![Partition {
            name: "firmware".into(),
            identifier: "mtd0".into(),
            bytes: 16_777_216,
            erase_bytes: 65_536,
        }];
        let report = evaluate(board(), Some(entries), 1);
        assert_eq!(report.partition_layout.status, Status::RequiresVerification);
        assert_eq!(report.extraction.status, Status::Unknown);
        assert!(!report.extraction.executable);
        assert!(report
            .image_format
            .missing_requirements
            .iter()
            .any(|v| v.contains("Signature")));
    }
}
