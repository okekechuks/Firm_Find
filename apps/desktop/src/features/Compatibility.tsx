import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage, getDeviceCompatibility, getProfileRegistry, recordDeviceObservation, validateDeviceProfile } from "../lib/backend";
import type { Device } from "../types/contracts";
import { AdapterStatus } from "./AdapterStatus";
import type { CompatibilityReport, Registry } from "../types/profiles";

const featureNames = {dns: "Custom DNS", wifiUserLimit: "Wi-Fi user limit", apn: "APN", ussd: "USSD"};
const human = (value: string) => value.replaceAll("_", " ").replace(/([a-z])([A-Z])/g, "$1 $2");
export function CompatibilityPanel({deviceId, revision, desktop}: {deviceId: string; revision: number; desktop: boolean}) {
  const [result, setResult] = useState<CompatibilityReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [expired, setExpired] = useState(false);
  const [value, setValue] = useState("");
  const [attribute, setAttribute] = useState("reported_identifier");
  const generation = useRef(0);
  const active = useRef(false);
  const invalidate = useCallback(() => {generation.current++;}, []);
  const refresh = useCallback(async () => {
    const run = ++generation.current;
    setLoading(true); setError(null);
    try { const report = await getDeviceCompatibility(deviceId); if (active.current && run === generation.current) {setResult(report); setExpired(report.expiresAt <= Date.now());} }
    catch (failure) {if (active.current && run === generation.current) setError(errorMessage(failure));}
    finally {if (active.current && run === generation.current) setLoading(false);}
  }, [deviceId]);
  useEffect(() => {
    active.current = true;
    if (desktop) void refresh();
    return () => {active.current = false; invalidate();};
  }, [desktop, deviceId, revision, refresh, invalidate]);
  useEffect(() => {
    if (!result) return;
    const timer = setTimeout(() => setExpired(true), Math.max(0, result.expiresAt - Date.now()));
    return () => clearTimeout(timer);
  }, [result]);
  const visible = result?.deviceId === deviceId && result.discoveryRevision === revision && !expired && !error ? result : null;
  const reportObservation = async (event: React.SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    const run = generation.current;
    try { await recordDeviceObservation(deviceId, attribute, value); if (active.current && run === generation.current) {setValue(""); await refresh();} }
    catch (failure) { if (active.current && run === generation.current) setError(errorMessage(failure)); }
  };
  return <section className="panel compatibility-panel" aria-label="Device compatibility">
    <div className="section-heading"><h2>Feature support</h2><button className="secondary-button" disabled={!desktop || loading} onClick={() => void refresh()}>{loading ? "Evaluating…" : "Re-evaluate support"}</button></div>
    {!desktop ? <p>Open the Windows app to evaluate real device evidence.</p> : null}
    {loading ? <p role="status">Checking identity, profile constraints and feature evidence…</p> : null}
    {error ? <p className="inventory-warning" role="alert">{error}</p> : null}
    {expired ? <p role="status">Evaluation expired. Re-evaluate against current evidence.</p> : null}
    {visible ? <>
      <p><strong>Profile match: {visible.confidence}</strong>{visible.selectedProfile ? ` · ${visible.selectedProfile} @ ${visible.profileVersion}` : " · No authoritative profile selected"}</p>
      <p className="muted small">A profile match does not authorize changes. Reported labels, USB identifiers and HTTP responses do not verify retail model or feature support.</p>
      <AdapterStatus deviceId={deviceId} revision={revision} desktop={desktop}/>
      {visible.warnings.map((warning) => <p role="status" key={warning}>{warning}</p>)}
      {visible.conflicts.length ? <p role="alert">Conflicting evidence: {visible.conflicts.map(human).join(", ")}. Obtain a trusted identity response to resolve it.</p> : null}
      <div className="support-table-wrap"><table className="support-table"><caption>Four-feature support matrix</caption><thead><tr><th>Feature</th><th>Assessment</th><th>Readiness</th><th>Evidence quality</th></tr></thead><tbody>
        {Object.entries(featureNames).map(([key, name]) => {const feature = visible.features[key as keyof typeof featureNames]; return <tr key={key}><th scope="row">{name}</th><td><span className={`assessment ${feature.assessment.toLowerCase()}`}>{feature.assessment}</span></td><td>{feature.readiness.map(human).join(" · ")}</td><td>{feature.quality}</td></tr>;})}
      </tbody></table></div>
      {Object.entries(featureNames).map(([key, name]) => {const feature = visible.features[key as keyof typeof featureNames]; return <details className="feature-explanation" key={key}><summary>{name}: evidence and prerequisites</summary>
        <p>{feature.reason}</p><p className="muted small">{feature.nextAction}</p>
        <dl className="capability-list">{Object.entries(feature.subCapabilities).map(([sub, decision]) => <div key={sub}><dt>{human(sub)}</dt><dd>{decision.assessment} — {decision.reason}</dd></div>)}</dl>
        {key === "wifiUserLimit" ? <p>Verified client-limit range: {feature.verifiedRange ? `${feature.verifiedRange.min}–${feature.verifiedRange.max}` : "Unknown"}</p> : null}
        <p className="muted small">Sources: {feature.evidenceSources.join(", ") || "No applicable verified feature source"}. Configuration in OBTAINER: not yet implemented.</p>
      </details>;})}
      <details className="feature-explanation"><summary>Profile candidates and exclusions</summary>{visible.candidates.map((candidate) => <article key={candidate.profileId}><h3>{candidate.name} <span className="muted small">Profile description · {candidate.confidence}</span></h3>
        <p className="muted small">{candidate.profileId} @ {candidate.profileVersion} · {candidate.origin}</p><p>Matched: {candidate.matchedFields.map(human).join(", ") || "None"}. Missing: {candidate.missingFields.map(human).join(", ") || "None"}. Excluded/conflicting fields: {candidate.conflictingFields.map(human).join(", ") || "None"}.</p>
        <ul>{candidate.reasons.map((reason) => <li key={reason}>{reason}</li>)}</ul></article>)}</details>
      <details className="feature-explanation"><summary>Identity evidence ({visible.evidence.length})</summary>{visible.evidence.map((e) => <article key={e.id}><strong>{human(e.attribute)}: {e.value}</strong><p className="muted small">{human(e.source)} · {human(e.verification)} · {e.reliability} · {new Date(e.collectedAt).toLocaleString()}{e.stale ? " · STALE" : ""}</p>{e.endpoint ? <p className="muted small">Endpoint association: {e.endpoint}</p> : null}{e.conflicts.length ? <p>Contradictory observation retained.</p> : null}</article>)}</details>
      {visible.change ? <details className="feature-explanation"><summary>Evaluation history</summary><p>Previous evaluation: {new Date(visible.change.previousEvaluatedAt).toLocaleString()}</p><ul>{visible.change.reasons.map((reason) => <li key={reason}>{reason}</li>)}</ul></details> : null}
    </> : !loading && desktop && !error ? <p>Select current evidence and re-evaluate. No support decision is active.</p> : null}
    {desktop ? <details className="feature-explanation"><summary>Add user-reported identity evidence</summary><p className="muted small">These observations stay unverified and cannot grant capability support. Add exact text from a label or admin page; do not enter passwords.</p><form className="reported-evidence" onSubmit={(event) => void reportObservation(event)}>
      <label>Reported field<select value={attribute} onChange={(e) => setAttribute(e.target.value)}><option value="reported_identifier">Identifier / unclear label</option><option value="model">Model</option><option value="hardware_revision">Hardware revision</option><option value="firmware_family">Firmware family</option><option value="firmware_version">Software / firmware version</option></select></label>
      <label>Reported value<input value={value} maxLength={256} required onChange={(e) => setValue(e.target.value)}/></label><button className="secondary-button" disabled={loading}>Save reported evidence</button>
    </form></details> : null}
  </section>;
}

export function Compatibility({devices, revision, desktop}: {devices: Device[]; revision: number; desktop: boolean}) {
  const [registry, setRegistry] = useState<Registry | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState("");
  const [document, setDocument] = useState("");
  const [validation, setValidation] = useState<string | null>(null);
  const validationRun = useRef(0);
  const invalidateValidation = useCallback(() => {validationRun.current++;}, []);
  useEffect(() => {
    let active = true;
    if (desktop) void getProfileRegistry().then((r) => {if (active) setRegistry(r);}).catch((e) => {if (active) setError(errorMessage(e));});
    return () => {active = false; invalidateValidation();};
  }, [desktop, invalidateValidation]);
  const validate = async () => {
    const run = ++validationRun.current; setValidation("Validating…");
    try {const p = await validateDeviceProfile(document); if (run === validationRun.current) setValidation(`Valid schema: ${p.profile.profileId} @ ${p.profile.profileVersion}. Local unverified preview only; not installed, trusted or activated.`);}
    catch (failure) {if (run === validationRun.current) setValidation(errorMessage(failure));}
  };
  return <>
    <section className="panel compatibility-panel"><h2>Device compatibility</h2><p>Compare current evidence with exact hardware and firmware profiles. Each feature is assessed independently.</p>
      {!desktop ? <p>Open the Windows app to access profiles and device evidence.</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      <label className="compatibility-selector">Device for compatibility<select value={selected} disabled={!desktop} onChange={(e) => setSelected(e.target.value)}><option value="">Select a Windows device candidate</option>{devices.filter((d) => d.category?.endsWith("candidate") || d.id === selected).map((d) => <option value={d.id} key={d.id}>{d.displayName} · {d.connected ? "present" : "disconnected"}</option>)}</select></label>
    </section>
    {selected ? <CompatibilityPanel key={selected} deviceId={selected} revision={revision} desktop={desktop}/> : null}
    <section className="panel compatibility-panel"><h2>Local profile registry</h2>{!registry && desktop && !error ? <p role="status">Loading validated profiles…</p> : null}{registry?.diagnostics.map((message) => <p role="alert" key={message}>{message}</p>)}
      {registry?.profiles.map(({profile, origin, authoritative}) => <details key={profile.profileId} className="feature-explanation"><summary>{profile.manufacturer} {profile.model} · {profile.profileVersion} · {authoritative ? "Curated documentation" : "Observation only"}</summary>
        <p>{profile.profileId} · Schema {profile.schemaVersion} · {origin}</p><p>Revisions: {profile.hardwareRevisions.join(", ") || "Unknown"}. Firmware: {profile.firmwareFamilies.join(", ") || "Unknown"} {profile.firmwareVersionConstraints.exact.join(", ")}</p><p>{profile.provenance.notes}</p>
        <ul>{profile.knownLimitations.map((limit) => <li key={limit}>{limit}</li>)}</ul>
        {profile.managementInterfaces.map((m, i) => <p key={i}>{m.protocol} · {m.authenticationRequired ? "Authentication required" : "Unauthenticated metadata only"} · {m.security} {m.limitations} Implementation in OBTAINER unavailable.</p>)}
        {profile.evidenceSources.map((source) => <p key={source.id}>{source.id}: {source.reference.startsWith("https://") ? <a href={source.reference} target="_blank" rel="noreferrer">{source.reference}</a> : source.reference} — {source.summary}</p>)}
      </details>)}
      <details className="feature-explanation"><summary>Validate a local profile document</summary><p className="muted small">Review only. Valid JSON does not establish provenance, device compatibility or permission.</p><label>Profile JSON<textarea value={document} maxLength={65536} rows={8} onChange={(e) => {validationRun.current++; setDocument(e.target.value); setValidation(null);}}/></label><button className="secondary-button" disabled={!desktop || !document} onClick={() => void validate()}>Validate profile</button>{validation ? <p role="status">{validation}</p> : null}</details>
    </section>
  </>;
}
