import { useCallback, useEffect, useRef, useState } from "react";
import { analyzeWebUiBundle, assessWebManagementBinding, errorMessage, getWebUiEvidence, importWebUiCapture } from "../lib/backend";
import type { BindingAssessment, WebEndpointEvidence, WebEvidenceReport } from "../types/web-management";

const label = (value: string) => value.replaceAll("_", " ");
const dangerous = new Set(["authentication", "unknown", "config_write", "config_restore", "firmware_upload", "firmware_update_check", "reboot", "factory_reset"]);
const groups: { title: string; includes: (entry: WebEndpointEvidence) => boolean }[] = [
  { title: "Observed management endpoints", includes: entry => entry.evidence !== "static_candidate" },
  { title: "Static route candidates", includes: entry => entry.evidence !== "har_observed" },
  { title: "Firmware-related requests", includes: entry => entry.classification.startsWith("firmware_") },
  { title: "Configuration-related requests", includes: entry => entry.classification.startsWith("config_") },
  { title: "Potentially dangerous requests", includes: entry => dangerous.has(entry.classification) },
  { title: "Unknown requests", includes: entry => entry.classification === "unknown" },
];

function EvidenceGroup({ title, entries }: { title: string; entries: WebEndpointEvidence[] }) {
  return <details><summary>{title} ({entries.length})</summary>{entries.length ? <ul>{entries.map(entry => <li key={entry.id}>
    <code>{entry.method} {entry.path}</code><p className="muted small">Origin: {entry.origin}</p><p>{label(entry.classification)} · {entry.evidence === "both" ? "HAR observed + static candidate" : label(entry.evidence)} · unverified{entry.status ? ` · HTTP ${entry.status}` : ""}</p>
    {entry.queryNames.length ? <p>Query field names: {entry.queryNames.join(", ")}</p> : null}
    {entry.requestBodyNames.length ? <p>Body field names: {entry.requestBodyNames.join(", ")}</p> : null}
  </li>)}</ul> : <p>No evidence collected in this category.</p>}</details>;
}

export function WebManagementWorkflow({ deviceId, revision, connected, managementOrigin, managementInterfaceId="" }: { deviceId: string; revision: number; connected: boolean; managementOrigin: string; managementInterfaceId?:string }) {
  const [open, setOpen] = useState(false);
  const [report, setReport] = useState<WebEvidenceReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [refresh, setRefresh] = useState(0);
  const [binding,setBinding] = useState<BindingAssessment|null>(null);
  const generation = useRef(0);
  const mounted = useRef(false);
  useEffect(() => { mounted.current = true; const requests = generation; return () => { mounted.current = false; requests.current++; }; }, []);
  useEffect(() => {
    const run = ++generation.current;
    setReport(null); setMessage(""); setError(""); setBusy(false); setBinding(null);
    if (!open) return;
    setLoading(true);
    void getWebUiEvidence(deviceId).then(result => {
      if (mounted.current && run === generation.current) setReport(result);
    }).catch(failure => {
      if (mounted.current && run === generation.current) setError(errorMessage(failure));
    }).finally(() => {
      if (mounted.current && run === generation.current) setLoading(false);
    });
  }, [deviceId, revision, connected, managementOrigin, managementInterfaceId, open, refresh]);
  const perform = useCallback(async (operation: () => Promise<WebEvidenceReport | null>, cancelled: string) => {
    const run = ++generation.current;
    setBusy(true); setError(""); setMessage("");
    try {
      const result = await operation();
      if (!mounted.current || run !== generation.current) return;
      if (result) { setReport(result); setMessage("Local evidence updated. No device requests were sent."); }
      else setMessage(cancelled);
    } catch (failure) {
      if (mounted.current && run === generation.current) setError(errorMessage(failure));
    } finally {
      if (mounted.current && run === generation.current) setBusy(false);
    }
  }, []);
  const checkRoute=async()=>{
    const run=++generation.current;setBusy(true);setError("");setBinding(null);
    try {
      const value=await assessWebManagementBinding(deviceId,managementOrigin,managementInterfaceId);
      if(mounted.current&&run===generation.current)setBinding(value);
    } catch(failure) {if(mounted.current&&run===generation.current)setError(errorMessage(failure));}
    finally {if(mounted.current&&run===generation.current)setBusy(false);}
  };
  return <details className="web-management-workflow" onToggle={event => setOpen(event.currentTarget.open)}>
    <summary>Web UI Evidence</summary>
    {open ? <section aria-label="Web management evidence" aria-busy={loading || busy}>
      <p>Import a HAR or selected local management-page files. Analysis never replays captured requests or executes downloaded scripts.</p>
      {loading ? <p role="status">Loading evidence permissions…</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      {message ? <p role="status">{message}</p> : null}
      {report ? <>
        <div className="web-evidence-actions">
          {report.actions.importHar ? <button className="secondary-button" disabled={busy || loading} onClick={() => void perform(() => importWebUiCapture(deviceId), "No capture selected. Evidence unchanged.")}>Import Web UI Capture (HAR)</button> : null}
          {report.actions.analyzeLocalBundle ? <button className="secondary-button" disabled={busy || loading || !managementOrigin} onClick={() => void perform(() => analyzeWebUiBundle(deviceId, managementOrigin), "No management files selected. Evidence unchanged.")}>Analyze Management UI</button> : null}
        </div>
        {!managementOrigin ? <p>Select a management address in Device observations before analyzing local management files.</p> : null}
        <section aria-label="Selected management route">
          <button className="secondary-button" disabled={busy||loading||!connected||!managementOrigin||!managementInterfaceId} onClick={()=>void checkRoute()}>Check selected route</button>
          <p className="muted small">Local Windows adapter and route checks only. No router requests or routing changes.</p>
          {binding?<><p>{binding.reason}</p>{binding.sourceIp?<p>Selected local source: {binding.sourceIp}</p>:null}</>:null}
        </section>
        <p className="muted small">Local evidence is held in memory for this app session. Raw captures, credentials, cookies and response bodies are not retained. HAR absence cannot prove an endpoint does not exist. A GET request can still change a device.</p>
        <h4>Authentication</h4>
        <p>Authentication scheme candidates: {report.authCandidates.length ? report.authCandidates.join(", ") : "Not observed"}.</p>
        <p>{report.actions.reason}</p>
        <p className="muted small">Captured routes cannot authorize authentication or further requests.</p>
        <h4>Firmware Sources</h4>
        <p>Observed downloads and update metadata remain source candidates until exact model/revision, origin, read-only route and authenticity are independently qualified. Configuration exports are not firmware; upload endpoints do not provide installed firmware.</p>
        <section role="region" aria-label="Web-management source assessment">
          {report.strategy.sources.map(source=><div key={source.evidenceId}><p>{label(source.sourceType)}: {label(source.state)}</p><p>{source.reason}</p></div>)}
          <p>Missing requirements:</p><ul>{report.strategy.missingRequirements.map(requirement=><li key={requirement}>{requirement}</li>)}</ul>
        </section>
        <h4>Web-management acquisition: BLOCKED</h4>
        <p>No verified firmware-read/export/FOTA source has been observed. Next: qualify an observed, documented read-only identity route and an exact firmware source.</p>
        {report.warnings.map((warning, index) => <p key={`${index}-${warning}`} role="status">{warning}</p>)}
        {groups.map(group => <EvidenceGroup key={group.title} title={group.title} entries={report.entries.filter(group.includes)}/>)}
        <details><summary>Referenced management resources ({report.resources.length})</summary>{report.resources.length ? <ul>{report.resources.map(resource => <li key={`${resource.origin}${resource.path}`}>{resource.kind}: <code>{resource.path}</code> · reference observed, unverified; not fetched</li>)}</ul> : <p>No referenced resources collected.</p>}</details>
      </> : !loading && !error ? <p>Evidence permissions unavailable. Refresh to retry.</p> : null}
      <button className="secondary-button" disabled={busy || loading} onClick={() => setRefresh(value => value + 1)}>Refresh web evidence</button>
    </section> : null}
  </details>;
}
