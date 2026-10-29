import { useEffect, useMemo, useRef, useState } from "react";
import { cancelLibraryJob, errorMessage, exportExtractedFiles, getFirmwareExtraction, readExtractedText, startFirmwareExtraction } from "../lib/backend";
import { formatBytes } from "../lib/format";
import type { ExtractionReport } from "../types/extraction";
import type { JobRecord } from "../types/library";
import { FirmwareStructure } from "./FirmwareStructure";

type LayeredReport = Extract<ExtractionReport, {version:2}>;
function LayerTree({layers,parent=null,depth=0}:{layers:LayeredReport["layers"];parent?:string|null;depth?:number}) {
  if(depth>7)return null;
  return <ul>{layers.filter(layer=>layer.parentId===parent).map(layer=><li key={layer.id}>
    <strong>{layer.format}</strong> · {layer.completeness} · {formatBytes(layer.length)}
    <p className="muted small">{layer.sourcePath} · {layer.space} bytes {layer.offset.toLocaleString()}–{(layer.offset+layer.length).toLocaleString()} · {layer.handler} v{layer.handlerVersion}</p>
    {layer.reasons.length?<ul>{layer.reasons.map((reason,index)=><li key={index}>{reason}</li>)}</ul>:null}
    <LayerTree layers={layers} parent={layer.id} depth={depth+1}/>
  </li>)}</ul>;
}

export function FirmwareExtraction({ sha256, jobs }: { sha256: string; jobs: JobRecord[] }) {
  const [report, setReport] = useState<ExtractionReport | null>(null);
  const [submitted, setSubmitted] = useState<JobRecord | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [folder, setFolder] = useState("");
  const [selection, setSelection] = useState<Set<string>>(() => new Set());
  const [preview, setPreview] = useState<{ path: string; text: string } | null>(null);
  const [exportApproved, setExportApproved] = useState(false);
  const [notice, setNotice] = useState("");
  const alive = useRef(false);
  const request = useRef(0);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const job = jobs.find(j => j.kind === "firmware_extraction" && j.relatedArtifact === sha256 && (submitted ? j.id === submitted.id : true)) ?? submitted;
  const active = job?.status === "pending" || job?.status === "running";
  useEffect(() => {
    let current = true;
    void getFirmwareExtraction(sha256).then(value => { if (current) {setReport(value);setSelection(new Set());setExportApproved(false);setFolder("");setPreview(null);request.current++;} }).catch(e => { if (current) setError(errorMessage(e)); });
    return () => { current = false; };
  }, [sha256, job?.status]);
  const visible = useMemo(() => report?.entries.filter(e => (e.path.includes("/") ? e.path.slice(0, e.path.lastIndexOf("/")) : "") === folder) ?? [], [report, folder]);
  const operate = async (action: () => Promise<void>) => {
    setBusy(true); setError(null); setNotice("");
    try { await action(); } catch (e) { if (alive.current) setError(errorMessage(e)); }
    finally { if (alive.current) setBusy(false); }
  };
  const inspect = (path: string) => {
    const id = ++request.current; setPreview(null);
    void operate(async () => { const text = await readExtractedText(sha256, path); if (alive.current && id === request.current) setPreview({ path, text }); });
  };
  const files = report?.entries.filter(entry=>entry.kind==="file").length??0;
  const verdict = report?.version===2 ? ({complete:"Complete container extraction",partial:files?"Partial extraction":"Partial inspection; no ordinary files extracted",unsupported:"Unsupported format","encrypted-or-unknown":"Encrypted or unknown; no decoded filesystem"}[report.completeness]) : report?.status==="partially_extracted"?"Filesystem extracted; some image contents omitted":"Filesystem extracted and verified";
  return <section aria-label="Extract firmware contents" className="extraction-section">
    <h3>Extract Contents</h3>
    <p>Unpack supported filesystems for inspection. Original firmware stays preserved.</p>
    {error ? <p role="alert">{error}</p> : null}
    {job?.status === "failed" || job?.status === "cancelled" ? <p role={job.status === "failed" ? "alert" : "status"}>{job.message}</p> : null}
    {active ? <div role="status"><p>{job.stage.replaceAll("_", " ")}</p><progress aria-label="Extraction progress"/><button disabled={busy} className="secondary-button" onClick={() => void operate(async () => { const next = await cancelLibraryJob(job.id); if (alive.current) setSubmitted(next); })}>Cancel extraction</button></div>
      : <button className="primary-button" disabled={busy} onClick={() => void operate(async () => { const next = await startFirmwareExtraction(sha256); if (alive.current) setSubmitted(next); })}>{report ? "Extract again" : "Extract Contents"}</button>}
    {report ? <>
      <p>{files} files · {formatBytes(report.totalBytes)} · {verdict}</p>
      <FirmwareStructure sha256={sha256} revision={report.bundleSha256}/>
      {report.version===2?<>
        <details><summary>Layer tree</summary><LayerTree layers={report.layers}/></details>
        <details><summary>Unexplained regions ({report.unexplainedRegions.length})</summary>
          {report.unexplainedRegions.length?<ul>{report.unexplainedRegions.map((region,index)=><li key={index}>{region.layerId}: bytes {region.offset.toLocaleString()}–{(region.offset+region.length).toLocaleString()} · {region.reason}</li>)}</ul>:<p>No unexplained byte ranges reported. See layer limitations.</p>}
          {report.omittedMembers.length?<><p>Omitted members</p><ul>{report.omittedMembers.map((member,index)=><li key={index}>{member.path} · {formatBytes(member.declaredBytes)} · {member.reason}</li>)}</ul></>:null}
        </details>
        <details><summary>Entropy map</summary><p>Sampled 64 KiB windows. High entropy can mean compression or encryption; it does not establish encryption.</p>
          <div className="extraction-files"><table><thead><tr><th>Source bytes</th><th>Bits per byte</th><th>Observation</th></tr></thead><tbody>{report.entropyMap.map(window=><tr key={window.offset}><td>{window.offset.toLocaleString()}–{(window.offset+window.length).toLocaleString()}</td><td>{window.bitsPerByte.toFixed(3)}</td><td>{window.classification==="high_entropy_compressed_or_encrypted"?"High entropy":"Mixed or low entropy"}</td></tr>)}</tbody></table></div>
        </details>
      </>:null}
      <details><summary>Extraction details and customization eligibility</summary><p>{report.extractor} · {report.region.format??"SquashFS"} at {report.region.offset.toLocaleString()}</p>{report.containerInfo ? <p>TRX v1 container checked by OpenWrt otrx. See the layer tree for each partition's result.</p> : null}
        {report.version===2?<p>Engine {report.provenance.engineVersion} · {report.provenance.tools.map(tool=>tool.version).join(" · ")||"Built-in bounded readers"}</p>:null}
        <ul>{report.limitations.map(line => <li key={line}>{line}</li>)}</ul><p>Rebuilding requires a separately qualified device and build adapter.</p></details>
      {report.entries.length?<details open><summary>Browse Extracted Files</summary>
        <p>Folder: /{folder}</p>
        {folder ? <button className="secondary-button" onClick={() => { setFolder(folder.includes("/") ? folder.slice(0, folder.lastIndexOf("/")) : ""); setPreview(null); request.current++; }}>Up one folder</button> : null}
        <div className="extraction-files"><table><thead><tr><th>Export</th><th>Name</th><th>Type</th><th>Size</th></tr></thead><tbody>
          {visible.map(entry => <tr key={entry.path}><td>{entry.kind === "file" ? <input type="checkbox" disabled={Boolean(entry.exportError)} aria-label={`Select ${entry.path} for export`} checked={selection.has(entry.path)} onChange={e => { const checked = e.target.checked; setSelection(previous => { const next = new Set(previous); if (checked) next.add(entry.path); else next.delete(entry.path); return next; }); setExportApproved(false); }}/> : null}</td><td>
            {entry.kind === "directory" ? <button className="text-button" onClick={() => { setFolder(entry.path); setPreview(null); request.current++; }}>{entry.path.split("/").at(-1)}/</button> : entry.kind === "file" ? <button className="text-button" disabled={busy || entry.size > 65536} onClick={() => inspect(entry.path)}>{entry.path.split("/").at(-1)}</button> : entry.path.split("/").at(-1)}
            {entry.exportError ? <p className="muted small">{entry.exportError}</p> : null}
          </td><td>{entry.kind.replaceAll("_", " ")}</td><td>{entry.kind === "file" ? formatBytes(entry.size) : "—"}</td></tr>)}
        </tbody></table></div>
      </details>:null}
      {preview ? <section aria-label="Extracted text preview"><h4>{preview.path}</h4><pre className="extraction-preview">{preview.text}</pre></section> : null}
      {files?<><p>{selection.size} files selected. Export includes only your selected ordinary files, with no executable permissions or links.</p>
      <label><input type="checkbox" checked={exportApproved} onChange={e => setExportApproved(e.target.checked)}/>I reviewed selected files for passwords, keys and personal information.</label>
      <div><button className="secondary-button" disabled={busy || active || !selection.size || !exportApproved} onClick={() => void operate(async () => { const saved = await exportExtractedFiles(sha256, [...selection]); if (alive.current) setNotice(saved ? "Selected files exported as a TAR archive." : "Export cancelled."); })}>Export Extracted Files</button></div>
      {notice ? <p role="status">{notice}</p> : null}
      </>:null}
    </> : null}
  </section>;
}
