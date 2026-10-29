import { useEffect, useMemo, useRef, useState } from "react";
import { errorMessage, getFirmwareStructure } from "../lib/backend";
import { formatBytes } from "../lib/format";
import type { FirmwareRegion, FirmwareStructureReport } from "../types/firmware-structure";

const readinessLabels = { extracted: "EXTRACTED", modifiable: "MODIFIABLE", repackable: "REPACKABLE", signatureCompatible: "SIGNATURE COMPATIBLE", installable: "INSTALLABLE" };
function RegionNode({ region, index, depth = 0 }: { region: FirmwareRegion; index: Map<string, FirmwareRegion>; depth?: number }) {
  if (depth > 9) return null;
  const absolute = region.sourceAbsoluteOffset;
  return <li role="treeitem" aria-expanded={region.children.length ? true : undefined}>
    <strong>{region.format}</strong> · {region.kind} · {formatBytes(region.length)}
    <p>{absolute !== null ? `Source bytes ${absolute.toLocaleString()}–${(absolute + region.length).toLocaleString()}` : `Relative bytes ${region.relativeOffset.toLocaleString()}–${(region.relativeOffset + region.length).toLocaleString()} (${region.coordinateSpace}); Source offset unavailable (expanded or member ancestry).`}</p>
    <p className="muted small">{region.states.join(" · ")}. {region.sha256 ? `SHA-256: ${region.sha256}` : "Hash unavailable in immutable report."}</p>
    {region.heuristics.map(hint => <p key={hint.kind}>Heuristic {hint.kind.replaceAll("_", " ")}: {hint.reason}</p>)}
    {region.reasons.length ? <ul>{region.reasons.map((reason, ordinal) => <li key={`${ordinal}-${reason}`}>{reason}</li>)}</ul> : null}
    {region.children.length ? <ul role="group">{region.children.map(id => { const child = index.get(id); return child ? <RegionNode key={id} region={child} index={index} depth={depth + 1}/> : null; })}</ul> : null}
  </li>;
}

export function FirmwareStructure({ sha256, revision }: { sha256: string; revision?: string }) {
  const [open, setOpen] = useState(false);
  const [report, setReport] = useState<FirmwareStructureReport | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [refresh, setRefresh] = useState(0);
  const generation = useRef(0);
  useEffect(() => {
    const run = ++generation.current;
    let current = true;
    setReport(null); setError("");
    if (!open) return;
    setLoading(true);
    void getFirmwareStructure(sha256).then(value => { if (current && run === generation.current) setReport(value); }).catch(failure => { if (current && run === generation.current) setError(errorMessage(failure)); }).finally(() => { if (current && run === generation.current) setLoading(false); });
    return () => { current = false; };
  }, [sha256, revision, open, refresh]);
  const index = useMemo(() => new Map(report?.regions.map(region => [region.id, region]) ?? []), [report]);
  return <details onToggle={event => setOpen(event.currentTarget.open)}>
    <summary>Artifact Structure</summary>
    {open ? <section className="firmware-structure" aria-label="Firmware artifact structure" aria-busy={loading}>
      {loading ? <p role="status">Loading immutable structure evidence…</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      {report ? <>
        <p>Source artifact SHA-256: <code>{report.inputSha256}</code>. {report.fileCount} extracted files · {formatBytes(report.totalExtractedBytes)}.</p>
        <p>Offsets refer to their stated coordinate space. Expanded streams and archive members do not establish physical partition offsets.</p>
        <ul role="tree" aria-label="Firmware regions">{report.regions.filter(region => region.parentId === null).map(region => <RegionNode key={region.id} region={region} index={index}/>)}</ul>
        {report.packageMetadata.length ? <details><summary>Package version declarations (unverified)</summary>{report.packageMetadata.map((metadata, ordinal) => <p key={`${metadata.layerId}-${ordinal}`}>{metadata.layerId}: {metadata.versions.join(", ") || "No version declared"}. Package declarations only; signature not verified.</p>)}</details> : null}
        <section aria-label="Rebuild readiness"><h4>Rebuild readiness</h4><dl>{(Object.keys(readinessLabels) as (keyof typeof readinessLabels)[]).map(key => <div key={key}><dt>{readinessLabels[key]}: {report.readiness[key].state}</dt><dd>{report.readiness[key].reason}</dd></div>)}</dl></section>
        <details><summary>Structure provenance and limits</summary><p>{report.provenance.extractor} · Tool SHA-256: <code>{report.provenance.toolSha256}</code></p>{report.provenance.tools.map(tool => <p key={tool.name}>{tool.name}: {tool.version} · <code>{tool.sha256}</code></p>)}{report.limitations.map((limitation, ordinal) => <p key={`${ordinal}-${limitation}`}>{limitation}</p>)}</details>
      </> : !loading && !error ? <p>No completed extraction evidence exists for this artifact. Unknown input remains preserved.</p> : null}
      <button className="secondary-button" disabled={loading} onClick={() => setRefresh(value => value + 1)}>Refresh structure</button>
    </section> : null}
  </details>;
}
