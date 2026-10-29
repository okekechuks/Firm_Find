//! Read-only structural projection of existing immutable extraction evidence.
use super::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareStructure {
    pub version: u32,
    pub input_sha256: String,
    pub input_bytes: u64,
    pub bundle_sha256: String,
    pub source_report_version: u32,
    pub source_status: String,
    pub file_count: u64,
    pub total_extracted_bytes: u64,
    pub provenance: StructureProvenance,
    pub regions: Vec<StructureRegion>,
    pub entropy_observations: Vec<Entropy>,
    pub package_metadata: Vec<PackageMetadata>,
    pub readiness: RebuildReadiness,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureProvenance {
    pub kind: &'static str,
    pub extractor: String,
    pub tool_sha256: String,
    pub engine_version: Option<String>,
    pub engine_sha256: Option<String>,
    pub tools: Vec<Tool>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageMetadata {
    pub layer_id: String,
    pub versions: Vec<String>,
    pub authority: &'static str,
    pub signature_verified: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureRegion {
    pub id: String,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub format: String,
    pub kind: &'static str,
    pub coordinate_space: String,
    pub relative_offset: u64,
    pub length: u64,
    pub source_absolute_offset: Option<u64>,
    pub sha256: Option<String>,
    pub confidence: String,
    pub states: Vec<&'static str>,
    pub extractor: Option<String>,
    pub extractor_version: Option<u32>,
    pub reasons: Vec<String>,
    pub heuristics: Vec<Heuristic>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Heuristic {
    pub kind: &'static str,
    pub reason: &'static str,
    pub verified: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Readiness {
    pub state: &'static str,
    pub executable: bool,
    pub reason: &'static str,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuildReadiness {
    pub extracted: Readiness,
    pub modifiable: Readiness,
    pub repackable: Readiness,
    pub signature_compatible: Readiness,
    pub installable: Readiness,
}
fn in_range(offset: u64, length: u64, limit: u64) -> bool {
    length > 0 && offset.checked_add(length).is_some_and(|end| end <= limit)
}
fn bounded(value: &str, maximum: usize) -> bool {
    value.len() <= maximum && !value.chars().any(|character| character == '\0')
}
fn kind(format: &str) -> &'static str {
    match format {
        "squashfs" | "ubifs" | "jffs2" | "cramfs" | "ext" | "ext2" | "ext3" | "ext4" | "yaffs2" => {
            "filesystem"
        }
        "gzip" | "xz" | "lzma" | "bzip2" | "lz4" | "zstd" => "compression",
        "delta" | "bsdiff" | "bsdiff40" => "delta",
        "trx" | "cpio" | "cpio_newc" | "cpio_crc" | "tar" | "zip" | "rar5" | "uimage" | "fit"
        | "ubi" | "android_boot" | "android_sparse" => "container",
        "raw" => "artifact",
        _ => "unknown",
    }
}
fn heuristics(format: &str) -> Vec<Heuristic> {
    if kind(format) == "filesystem" {
        vec![Heuristic { kind: "rootfs_candidate", reason: "Filesystem signature is only a rootfs candidate. Boot role, partition identity and system completeness remain unverified; no first-filesystem preference is applied.", verified: false }]
    } else if matches!(format, "uimage" | "fit" | "android_boot") {
        vec![Heuristic { kind: "kernel_candidate", reason: "Boot-image container is a kernel candidate only. The report does not establish that every payload is a kernel or define its physical partition.", verified: false }]
    } else {
        Vec::new()
    }
}
fn add_region(regions: &mut Vec<StructureRegion>, region: StructureRegion) -> Result<()> {
    if let Some(parent) = region.parent_id.as_deref() {
        let parent = regions
            .iter_mut()
            .find(|item| item.id == parent)
            .ok_or_else(invalid)?;
        parent.children.push(region.id.clone());
    }
    regions.push(region);
    Ok(())
}
fn gap_region(
    parent: &StructureRegion,
    id: String,
    offset: u64,
    length: u64,
    reason: String,
) -> Result<StructureRegion> {
    if !in_range(offset, length, parent.length) || !bounded(&reason, 2048) {
        return Err(invalid());
    }
    let absolute = parent
        .source_absolute_offset
        .map(|base| base.checked_add(offset).ok_or_else(invalid))
        .transpose()?;
    Ok(StructureRegion {
        id,
        parent_id: Some(parent.id.clone()),
        children: Vec::new(),
        format: "unknown".into(),
        kind: "unknown",
        coordinate_space: parent.coordinate_space.clone(),
        relative_offset: offset,
        length,
        source_absolute_offset: absolute,
        sha256: None,
        confidence: "unknown".into(),
        states: vec!["bounded", "unknown"],
        extractor: None,
        extractor_version: None,
        reasons: vec![reason],
        heuristics: Vec::new(),
    })
}
fn layered(report: &ExtractionReport) -> Result<Vec<StructureRegion>> {
    let layers = report.layers.as_deref().ok_or_else(invalid)?;
    let gaps = report.unexplained_regions.as_deref().ok_or_else(invalid)?;
    if layers.is_empty() || layers.len() > 256 || gaps.len() > 4096 {
        return Err(invalid());
    }
    let mut regions: Vec<StructureRegion> = Vec::new();
    let mut positions: HashMap<&str, (usize, usize)> = HashMap::new();
    for (ordinal, layer) in layers.iter().enumerate() {
        if positions.contains_key(layer.id.as_str())
            || layer.id != format!("layer-{ordinal:04}")
            || !valid_hash(&layer.input_sha256)
            || layer.length == 0
            || layer.length > MAX_INPUT
            || !matches!(layer.space.as_str(), "source" | "expanded" | "member")
            || !matches!(layer.confidence.as_str(), "signature" | "validated")
            || !matches!(
                layer.completeness.as_str(),
                "complete" | "partial" | "unsupported" | "encrypted-or-unknown"
            )
            || !bounded(&layer.format, 40)
            || !bounded(&layer.handler, 40)
            || layer.reasons.len() > 4096
            || layer.reasons.iter().any(|reason| !bounded(reason, 2048))
        {
            return Err(invalid());
        }
        let (absolute, depth) = if let Some(parent) = layer.parent_id.as_deref() {
            let (index, depth) = positions.get(parent).copied().ok_or_else(invalid)?;
            let parent = &regions[index];
            if layer.space == "source" && !in_range(layer.offset, layer.length, parent.length)
                || layer.space != "source" && (layer.offset != 0 || layer.length > MAX_OUTPUT)
            {
                return Err(invalid());
            }
            let absolute = if layer.space == "source" {
                parent
                    .source_absolute_offset
                    .map(|base| base.checked_add(layer.offset).ok_or_else(invalid))
                    .transpose()?
            } else {
                None
            };
            (absolute, depth + 1)
        } else {
            if ordinal != 0
                || layer.space != "source"
                || layer.offset != 0
                || layer.length != report.input_bytes
                || layer.input_sha256 != report.input_sha256
            {
                return Err(invalid());
            }
            (Some(0), 0)
        };
        if depth > 7
            || absolute.is_some_and(|offset| !in_range(offset, layer.length, report.input_bytes))
        {
            return Err(invalid());
        }
        let mut states = vec!["bounded"];
        if layer.format == "unknown" {
            states.push("unknown");
        } else {
            states.insert(0, "detected");
        }
        if layer.confidence == "validated" && layer.format != "unknown" {
            states.push("validated");
        }
        if layer.completeness == "unsupported" {
            states.push("unsupported");
            if layer
                .reasons
                .iter()
                .any(|reason| reason.starts_with("Nested layer rejected: corrupt_image;"))
            {
                states.push("corrupt");
            }
        } else if layer.decoded_bytes > 0 && layer.confidence == "validated" {
            states.extend(["extractable", "extracted"]);
        }
        if layer.completeness == "encrypted-or-unknown" && !states.contains(&"unknown") {
            states.push("unknown");
        }
        if absolute.is_some_and(|offset| {
            report
                .entropy_map
                .as_deref()
                .unwrap_or_default()
                .iter()
                .any(|window| {
                    window.bits_per_byte >= 7.6
                        && window.offset < offset.saturating_add(layer.length)
                        && offset < window.offset.saturating_add(window.length)
                })
        }) {
            states.push("encrypted_or_high_entropy");
        }
        let region = StructureRegion {
            id: layer.id.clone(),
            parent_id: layer.parent_id.clone(),
            children: Vec::new(),
            format: layer.format.clone(),
            kind: kind(&layer.format),
            coordinate_space: layer.space.clone(),
            relative_offset: layer.offset,
            length: layer.length,
            source_absolute_offset: absolute,
            sha256: Some(layer.input_sha256.clone()),
            confidence: layer.confidence.clone(),
            states,
            extractor: Some(layer.handler.clone()),
            extractor_version: Some(layer.handler_version),
            reasons: layer.reasons.clone(),
            heuristics: heuristics(&layer.format),
        };
        add_region(&mut regions, region)?;
        positions.insert(layer.id.as_str(), (ordinal, depth));
    }
    for (ordinal, gap) in gaps.iter().enumerate() {
        let (index, _) = positions
            .get(gap.layer_id.as_str())
            .copied()
            .ok_or_else(invalid)?;
        let region = gap_region(
            &regions[index],
            format!("unknown-{ordinal:04}"),
            gap.offset,
            gap.length,
            gap.reason.clone(),
        )?;
        add_region(&mut regions, region)?;
    }
    Ok(regions)
}
fn legacy(report: &ExtractionReport) -> Result<Vec<StructureRegion>> {
    if !matches!(report.container.as_str(), "raw" | "gzip" | "trx")
        || report.region.format != "squashfs"
        || report.unextracted_regions.len() > 4096
    {
        return Err(invalid());
    }
    let mut regions = vec![StructureRegion {
        id: "artifact".into(),
        parent_id: None,
        children: Vec::new(),
        format: report.container.clone(),
        kind: kind(&report.container),
        coordinate_space: "source".into(),
        relative_offset: 0,
        length: report.input_bytes,
        source_absolute_offset: Some(0),
        sha256: Some(report.input_sha256.clone()),
        confidence: "validated".into(),
        states: vec!["detected", "bounded", "validated"],
        extractor: Some(report.extractor.clone()),
        extractor_version: None,
        reasons: Vec::new(),
        heuristics: Vec::new(),
    }];
    let expanded = report.container == "gzip";
    let limit = if expanded {
        report.expanded_bytes
    } else {
        report.input_bytes
    };
    if !in_range(report.region.offset, report.region.length, limit) {
        return Err(invalid());
    }
    let mut states = vec!["detected", "bounded", "validated"];
    if report.file_count > 0 {
        states.extend(["extractable", "extracted"]);
    }
    add_region(
        &mut regions,
        StructureRegion {
            id: "legacy-filesystem".into(),
            parent_id: Some("artifact".into()),
            children: Vec::new(),
            format: report.region.format.clone(),
            kind: "filesystem",
            coordinate_space: if expanded { "expanded" } else { "source" }.into(),
            relative_offset: report.region.offset,
            length: report.region.length,
            source_absolute_offset: (!expanded).then_some(report.region.offset),
            sha256: if !expanded
                && report.region.offset == 0
                && report.region.length == report.input_bytes
            {
                Some(report.input_sha256.clone())
            } else {
                None
            },
            confidence: "validated".into(),
            states,
            extractor: Some(report.extractor.clone()),
            extractor_version: None,
            reasons: vec![
                "Legacy report does not retain subregion byte hashes or the complete layer graph."
                    .into(),
            ],
            heuristics: heuristics(&report.region.format),
        },
    )?;
    for (ordinal, gap) in report.unextracted_regions.iter().enumerate() {
        if !in_range(gap.offset, gap.length, limit) {
            return Err(invalid());
        }
        let mut parent = regions[0].clone();
        if expanded {
            parent.coordinate_space = "expanded".into();
            parent.source_absolute_offset = None;
            parent.length = report.expanded_bytes;
        }
        let region = gap_region(
            &parent,
            format!("unknown-{ordinal:04}"),
            gap.offset,
            gap.length,
            "Unextracted range from legacy immutable report.".into(),
        )?;
        add_region(&mut regions, region)?;
    }
    Ok(regions)
}
pub fn derive(report: &ExtractionReport) -> Result<FirmwareStructure> {
    if report.worker_exit != 0
        || !matches!(report.version, 1 | 2)
        || !valid_hash(&report.input_sha256)
        || !valid_hash(&report.bundle_sha256)
        || !valid_hash(&report.tool_sha256)
        || report.input_bytes == 0
        || report.input_bytes > MAX_INPUT
        || report.total_bytes > MAX_OUTPUT
        || report.entries.len() > 20000
        || report.file_count
            != report
                .entries
                .iter()
                .filter(|entry| entry.kind == "file")
                .count() as u64
        || report.limitations.len() > 4088
        || report
            .limitations
            .iter()
            .any(|reason| !bounded(reason, 2048))
    {
        return Err(invalid());
    }
    let regions = if report.version == 2 {
        layered(report)?
    } else {
        legacy(report)?
    };
    let entropy = report.entropy_map.clone().unwrap_or_default();
    if entropy.len() > 512
        || entropy.iter().any(|window| {
            !in_range(window.offset, window.length, report.input_bytes)
                || !window.bits_per_byte.is_finite()
                || !(0.0..=8.0).contains(&window.bits_per_byte)
        })
    {
        return Err(invalid());
    }
    let tools = report
        .provenance
        .as_ref()
        .map(|provenance| provenance.tools.clone())
        .unwrap_or_default();
    if tools.len() > 16
        || tools.iter().any(|tool| {
            !valid_hash(&tool.sha256) || !bounded(&tool.name, 240) || !bounded(&tool.version, 128)
        })
    {
        return Err(invalid());
    }
    let declared = report.swupdate.as_deref().unwrap_or_default();
    if declared.len() > 256
        || declared.iter().any(|summary| {
            summary.signature_verified
                || summary.authority != "package_declarations_unverified"
                || summary.versions.len() > 32
                || summary
                    .versions
                    .iter()
                    .any(|version| !bounded(version, 128))
                || !regions.iter().any(|region| region.id == summary.layer_id)
        })
    {
        return Err(invalid());
    }
    let package_metadata = declared
        .iter()
        .map(|summary| PackageMetadata {
            layer_id: summary.layer_id.clone(),
            versions: summary.versions.clone(),
            authority: "package_declarations_unverified",
            signature_verified: false,
        })
        .collect();
    let unknown = |reason| Readiness {
        state: "unknown",
        executable: false,
        reason,
    };
    let readiness=RebuildReadiness{extracted:Readiness{state:if report.file_count>0{"observed"}else{"unknown"},executable:false,reason:if report.file_count>0{"Ordinary files were retained and hashed by the extraction pipeline; this is not complete physical firmware acquisition."}else{"No ordinary files were retained; detected or inspected containers are not an extracted filesystem."}},modifiable:unknown("No qualified hardware feature and image-modification adapter is established by extraction."),repackable:unknown("Container layout, alignment, checksums, partition-size limits and a structurally qualified packer are required separately."),signature_compatible:unknown("Required signature rules and legitimate signing compatibility are unqualified."),installable:unknown("Exact device binding, signatures, installation verification and recovery are unqualified; extraction never grants installation authority.")};
    let mut limitations = report.limitations.clone();
    limitations.extend(["Derived from immutable Library extraction evidence; no physical acquisition claim.","Filesystem/rootfs and boot-image/kernel labels are heuristics; no boot mapping or physical partition identity is established.","Layer bounds describe exact input views, which may include unexplained trailer bytes; they do not automatically describe partition or filesystem declared sizes.","Expanded/member coordinates cannot establish physical source offsets without a separately recorded reversible mapping.","Unknown-range hashes and legacy subregion hashes are unavailable in existing immutable reports; original bytes remain preserved.","Per-layer extracted state requires recorded decoded bytes; overall extracted readiness refers only to retained ordinary files.","High entropy is sampled and may mean compression or encryption; it does not prove encryption.","No signature, packer, partition-size or install/recovery qualification follows from extraction."].into_iter().map(String::from));
    Ok(FirmwareStructure {
        version: 1,
        input_sha256: report.input_sha256.clone(),
        input_bytes: report.input_bytes,
        bundle_sha256: report.bundle_sha256.clone(),
        source_report_version: report.version,
        source_status: report.status.clone(),
        file_count: report.file_count,
        total_extracted_bytes: report.total_bytes,
        provenance: StructureProvenance {
            kind: "library_image_analysis_not_physical_acquisition",
            extractor: report.extractor.clone(),
            tool_sha256: report.tool_sha256.clone(),
            engine_version: report
                .provenance
                .as_ref()
                .map(|provenance| provenance.engine_version.clone()),
            engine_sha256: report
                .provenance
                .as_ref()
                .map(|provenance| provenance.engine_sha256.clone()),
            tools,
        },
        regions,
        entropy_observations: entropy,
        package_metadata,
        readiness,
        limitations,
    })
}
impl LibraryService {
    pub fn firmware_structure(&self, hash: &str) -> Result<Option<FirmwareStructure>> {
        self.extraction_report(hash)?
            .as_ref()
            .map(derive)
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn layer(
        id: &str,
        parent: Option<&str>,
        space: &str,
        offset: u64,
        length: u64,
        format: &str,
    ) -> Value {
        json!({"id":id,"parentId":parent,"space":space,"offset":offset,"length":length,"sourcePath":"synthetic","inputSha256":if parent.is_none(){"a".repeat(64)}else{"b".repeat(64)},"handler":format,"handlerVersion":1,"format":format,"confidence":"validated","decodedBytes":0,"completeness":"partial","reasons":[]})
    }
    fn report(layers: Vec<Value>) -> ExtractionReport {
        serde_json::from_value(json!({"version":2,"inputSha256":"a".repeat(64),"inputBytes":2048,"bundleSha256":"c".repeat(64),"region":{"format":"unknown","offset":0,"length":2048,"compression":0},"container":"unknown","expandedBytes":5120,"extractor":"Synthetic bounded engine","toolSha256":"d".repeat(64),"entries":[],"totalBytes":0,"fileCount":0,"directoryCount":0,"filesystemRoots":["/"],"workerExit":0,"status":"inspection_only","limitations":[],"isolation":{},"createdAt":0,"completeness":"partial","layers":layers,"unextractedRegions":[],"unexplainedRegions":[],"entropyMap":[],"omittedMembers":[],"swupdate":[]})).unwrap()
    }
    fn graph(report: &ExtractionReport) -> Value {
        serde_json::to_value(derive(report).unwrap()).unwrap()
    }
    fn find<'a>(graph: &'a Value, id: &str) -> &'a Value {
        graph["regions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|region| region["id"] == id)
            .unwrap()
    }

    #[test]
    fn firmware_structure_offsets_do_not_invent_physical_positions_after_decode_or_members() {
        let mut report = report(vec![
            layer("layer-0000", None, "source", 0, 2048, "unknown"),
            layer(
                "layer-0001",
                Some("layer-0000"),
                "source",
                128,
                1024,
                "gzip",
            ),
            layer("layer-0002", Some("layer-0001"), "expanded", 0, 5120, "tar"),
            layer(
                "layer-0003",
                Some("layer-0002"),
                "source",
                64,
                512,
                "squashfs",
            ),
            layer(
                "layer-0004",
                Some("layer-0000"),
                "source",
                1536,
                256,
                "squashfs",
            ),
            layer("layer-0005", Some("layer-0004"), "member", 0, 256, "cpio"),
        ]);
        report.unexplained_regions = Some(vec![Unexplained {
            layer_id: "layer-0003".into(),
            offset: 16,
            length: 32,
            reason: "synthetic unknown bytes".into(),
        }]);
        let graph = graph(&report);
        assert_eq!(find(&graph, "layer-0001")["sourceAbsoluteOffset"], 128);
        assert!(find(&graph, "layer-0002")["sourceAbsoluteOffset"].is_null());
        assert!(find(&graph, "layer-0003")["sourceAbsoluteOffset"].is_null());
        assert_eq!(find(&graph, "layer-0004")["sourceAbsoluteOffset"], 1536);
        assert!(find(&graph, "layer-0005")["sourceAbsoluteOffset"].is_null());
        let gap = find(&graph, "unknown-0000");
        assert_eq!(gap["relativeOffset"], 16);
        assert_eq!(gap["length"], 32);
        assert_eq!(gap["parentId"], "layer-0003");
        assert!(gap["sourceAbsoluteOffset"].is_null());
        assert!(gap["sha256"].is_null());
    }
    #[test]
    fn firmware_structure_keeps_multiple_rootfs_candidates_and_rebuild_states_independent() {
        let mut report = report(vec![
            layer("layer-0000", None, "source", 0, 2048, "trx"),
            layer(
                "layer-0001",
                Some("layer-0000"),
                "source",
                64,
                512,
                "squashfs",
            ),
            layer(
                "layer-0002",
                Some("layer-0000"),
                "source",
                1024,
                512,
                "squashfs",
            ),
        ]);
        report.entries = vec![Entry {
            path: "etc/synthetic".into(),
            kind: "file".into(),
            size: 1,
            sha256: Some("e".repeat(64)),
            link_target: None,
            offset: 0,
            export_error: None,
            mode: Some(0o600),
        }];
        report.file_count = 1;
        report.total_bytes = 1;
        let graph = graph(&report);
        for id in ["layer-0001", "layer-0002"] {
            let node = find(&graph, id);
            assert!(node["heuristics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|hint| hint["kind"] == "rootfs_candidate" && hint["verified"] == false));
        }
        assert_eq!(graph["readiness"]["extracted"]["state"], "observed");
        for state in [
            "modifiable",
            "repackable",
            "signatureCompatible",
            "installable",
        ] {
            assert_eq!(graph["readiness"][state]["state"], "unknown");
            assert_eq!(graph["readiness"][state]["executable"], false);
        }
    }
    #[test]
    fn firmware_structure_unknown_entropy_and_unsupported_never_become_extracted() {
        let mut report = report(vec![
            layer("layer-0000", None, "source", 0, 2048, "unknown"),
            layer(
                "layer-0001",
                Some("layer-0000"),
                "source",
                128,
                512,
                "ubifs",
            ),
        ]);
        let first = &mut report.layers.as_mut().unwrap()[0];
        first.confidence = "signature".into();
        first.completeness = "encrypted-or-unknown".into();
        let second = &mut report.layers.as_mut().unwrap()[1];
        second.confidence = "signature".into();
        second.completeness = "unsupported".into();
        report.unexplained_regions = Some(vec![Unexplained {
            layer_id: "layer-0000".into(),
            offset: 0,
            length: 128,
            reason: "unknown wrapper".into(),
        }]);
        report.entropy_map = Some(vec![Entropy {
            offset: 0,
            length: 2048,
            sample_bytes: 64,
            bits_per_byte: 7.9,
            classification: "high_entropy_compressed_or_encrypted".into(),
        }]);
        let graph = graph(&report);
        assert_eq!(graph["readiness"]["extracted"]["state"], "unknown");
        assert!(find(&graph, "layer-0001")["states"]
            .as_array()
            .unwrap()
            .contains(&json!("unsupported")));
        assert!(!find(&graph, "layer-0001")["states"]
            .as_array()
            .unwrap()
            .contains(&json!("extracted")));
        assert_eq!(
            graph["entropyObservations"][0]["classification"],
            "high_entropy_compressed_or_encrypted"
        );
        assert_eq!(find(&graph, "unknown-0000")["sourceAbsoluteOffset"], 0);
    }
    #[test]
    fn firmware_structure_rejects_duplicate_cyclic_overflow_and_out_of_bounds_topology() {
        for mode in 0..4 {
            let mut report = report(vec![
                layer("layer-0000", None, "source", 0, 2048, "unknown"),
                layer("layer-0001", Some("layer-0000"), "source", 128, 512, "gzip"),
            ]);
            let node = &mut report.layers.as_mut().unwrap()[1];
            match mode {
                0 => node.id = "layer-0000".into(),
                1 => node.parent_id = Some("layer-0001".into()),
                2 => node.offset = u64::MAX,
                _ => node.length = 2048,
            };
            assert!(derive(&report).is_err(), "mutation {mode}");
        }
    }
    #[test]
    fn firmware_structure_legacy_projection_is_deterministic_and_never_rewrites_report() {
        let mut report = report(vec![]);
        report.version = 1;
        report.layers = None;
        report.container = "gzip".into();
        report.expanded_bytes = 4096;
        report.region = Region {
            format: "squashfs".into(),
            offset: 64,
            length: 1024,
            compression: 4,
        };
        report.unextracted_regions = vec![ByteRegion {
            offset: 0,
            length: 64,
        }];
        let before = serde_json::to_vec(&report).unwrap();
        let first = graph(&report);
        let second = graph(&report);
        assert_eq!(first, second);
        assert_eq!(before, serde_json::to_vec(&report).unwrap());
        let node = find(&first, "legacy-filesystem");
        assert_eq!(node["coordinateSpace"], "expanded");
        assert!(node["sourceAbsoluteOffset"].is_null());
        assert!(node["sha256"].is_null());
        assert_eq!(
            first["provenance"]["kind"],
            "library_image_analysis_not_physical_acquisition"
        );
    }
}
