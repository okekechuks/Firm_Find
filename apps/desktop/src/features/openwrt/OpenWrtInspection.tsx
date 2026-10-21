import { useCallback, useEffect, useId, useRef, useState } from "react";
import { X } from "lucide-react";
import { errorMessage } from "../../lib/backend";
import { openWrtApproveReads, openWrtCancel, openWrtConnect, openWrtDisconnect, openWrtInspect, openWrtObserve, openWrtStatus } from "../../lib/openwrt";
import { hostFingerprintSchema, type OpenWrtStatus } from "../../types/openwrt";
import { adapterErrorSchema } from "../../types/adapters";
import type { Device } from "../../types/contracts";
import { ConfigurationInspection } from "./ConfigurationInspection";
import { FirmwareOperations } from "./FirmwareOperations";
import "./openwrt.css";

export function OpenWrtInspection({ devices, onClose }: {devices: Device[]; onClose: () => void}) {
  const [status, setStatus] = useState<OpenWrtStatus | null>(null);
  const [targetId, setTargetId] = useState("");
  const [port, setPort] = useState("22");
  const [ownership, setOwnership] = useState(false);
  const [independent, setIndependent] = useState(false);
  const [fingerprint, setFingerprint] = useState("");
  const [username, setUsername] = useState("root");
  const [readConsent, setReadConsent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(Date.now);
  const mounted = useRef(false);
  const generation = useRef(0);
  const pending = useRef(false);
  const activeSession = useRef(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const titleId = useId();
  const accept = useCallback((next: OpenWrtStatus) => {
    activeSession.current = !["idle", "disconnected", "failed"].includes(next.state);
    setStatus(next);
  }, []);
  useEffect(() => {
    mounted.current = true;
    const element = dialog.current;
    const previous = document.activeElement as HTMLElement | null;
    element?.showModal();
    closeButton.current?.focus();
    const id = ++generation.current;
    void openWrtStatus().then(next => {
      if (mounted.current && id === generation.current) { accept(next); setTargetId(next.targetId ?? ""); }
    }).catch(failure => { if (mounted.current && id === generation.current) setError(errorMessage(failure)); })
      .finally(() => { if (mounted.current && id === generation.current) setLoading(false); });
    return () => {
      mounted.current = false;
      element?.close();
      previous?.focus();
      // Navigation also revokes the in-memory handshake/session. Late responses never update React.
      if (pending.current || activeSession.current) void openWrtCancel().catch(() => {});
    };
  }, [accept]);
  useEffect(() => {
    if (!status?.expiresAt) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [status?.expiresAt]);
  const operation = useCallback(async (work: () => Promise<OpenWrtStatus>, close = false) => {
    const id = ++generation.current;
    pending.current = true;
    setBusy(true); setLoading(false); setError(null);
    try {
      const next = await work();
      if (mounted.current && id === generation.current) { accept(next); if (close) onClose(); }
    } catch (failure) {
      if (mounted.current && id === generation.current) {
        const parsed = adapterErrorSchema.safeParse(failure);
        setError(parsed.success ? `${parsed.data.code}: ${parsed.data.message} ${parsed.data.suggestedAction}` : errorMessage(failure));
        try { const next = await openWrtStatus(); if (mounted.current && id === generation.current) accept(next); }
        catch { /* Retain the operation failure and last acknowledged backend state. */ }
      }
    } finally {
      if (id === generation.current) { pending.current = false; if (mounted.current) setBusy(false); }
    }
  }, [accept, onClose]);
  const working = busy || status?.busy === true;
  const expired = status?.expiresAt !== null && status?.expiresAt !== undefined && now >= status.expiresAt;
  const locked = Boolean(status && !["idle", "disconnected", "failed"].includes(status.state));
  const target = status?.targets.find(item => item.id === (locked ? status.targetId : targetId));
  const currentInterface = devices.filter(device => device.connected).flatMap(device => device.networks)
    .find(network => network.id === target?.interfaceId && network.linkUp && !network.virtualInterface);
  const targetAvailable = Boolean(target && (target.lab || currentInterface));
  const authenticated = Boolean(status?.identity && ["identity_only", "read_authorized", "inspected"].includes(status.state));
  const reportedError = error ?? (status?.error ? `${status.error.code}: ${status.error.message} ${status.error.suggestedAction}` : null);
  const close = () => {
    if (working || locked) void operation(openWrtDisconnect, true);
    else onClose();
  };
  const cancel = () => { setReadConsent(false); setIndependent(false); setFingerprint(""); void operation(openWrtCancel); };
  const changeTarget = (value: string) => { setTargetId(value); setOwnership(false); setIndependent(false); setFingerprint(""); setReadConsent(false); };
  return <dialog ref={dialog} className="openwrt-dialog" aria-labelledby={titleId} onCancel={event => { event.preventDefault(); close(); }}>
    <section className="openwrt-workspace" aria-label="OpenWrt device inspection">
      <div className="section-heading"><div><h2 id={titleId}>OpenWrt device inspection</h2><span className="badge">{target?.lab ? "CONTROLLED OPENWRT VM — NOT PHYSICAL HARDWARE" : "REAL DEVICE — INSPECTION MODE"}</span></div><button ref={closeButton} className="icon-button" aria-label="Close OpenWrt inspection" onClick={close}><X size={18}/></button></div>
      <p>For routers already running OpenWrt with existing SSH key access. A regular proprietary MiFi does not qualify.</p>
      <details><summary>What this connection permits</summary><p>OBTAINER uses reviewed read-only commands. It does not intentionally change persistent configuration. A privileged SSH account can write outside OBTAINER; these application restrictions do not remove the account’s privileges.</p><p>Firmware extraction, configuration changes, flashing and live USSD remain unavailable in this phase.</p></details>
      {reportedError ? <div role="alert" className="inventory-warning">{reportedError}</div> : null}
      {loading ? <p role="status">Loading OpenWrt connection status…</p> : !status ? <button className="secondary-button" onClick={() => void operation(openWrtStatus)}>Retry connection status</button> : !status.enabled ? <p role="status">Real OpenWrt inspection is unavailable in this entry point. The Virtual Router Lab has separate authority.</p> : <>
        <div className="openwrt-target-grid">
          <label>Saved router<select aria-label="OpenWrt target" value={locked ? status.targetId ?? "" : targetId} disabled={working || locked} onChange={event => changeTarget(event.target.value)}><option value="">Select a saved router</option>{status.targets.map(item => <option key={item.id} value={item.id}>{item.label || item.address}{item.lab ? " · controlled VM" : ""}</option>)}</select></label>
          <label>SSH port<input aria-label="OpenWrt SSH port" type="number" min="1" max="65535" value={port} disabled={working || locked} onChange={event => {setPort(event.target.value); setOwnership(false);}}/></label>
        </div>
        {!status.targets.length ? <p>No saved router targets. Close this window and use Add Router, selecting its active network interface.</p> : null}
        {target ? <div className="openwrt-endpoint"><p>Selected address: <strong>{target.address}</strong> · SSH port {locked && status.endpoint ? status.endpoint : port}</p><p>Bound interface: {currentInterface?.displayName ?? target.interfaceId ?? "Not selected"}{status.sourceAddress ? ` · source ${status.sourceAddress}` : ""}</p>{!targetAvailable ? <p role="status">Select a saved router with an active physical network interface. Open Manage saved connections to review the binding.</p> : null}</div> : null}
        {!locked ? <><label className="target-consent"><input type="checkbox" checked={ownership} disabled={working || !targetAvailable} onChange={event => setOwnership(event.target.checked)}/>I own or administer this selected endpoint and approve one bounded SSH host-key observation.</label><button className="primary-button" disabled={working || !targetAvailable || !ownership || !Number.isInteger(Number(port)) || Number(port) < 1 || Number(port) > 65535} onClick={() => {setFingerprint(""); setIndependent(false); void operation(() => openWrtObserve(targetId, Number(port)));}}>Observe SSH host key</button></> : null}
        <p role="status">Connection: {status.state.replaceAll("_", " ")}{expired ? " · approval expired; disconnect and start again" : ""}</p>
        {status.hostFingerprint ? <section className="openwrt-trust" aria-label="SSH host identity"><h3>SSH host identity</h3><code>{status.hostFingerprint}</code><p>Observed fingerprint is untrusted until checked against an independent source, such as the router console or verified administrator records.</p>
          {status.state === "host_observed" ? <><label>Independently verified fingerprint<input aria-label="Independently verified SSH fingerprint" autoComplete="off" spellCheck={false} maxLength={50} disabled={working || expired} value={fingerprint} onChange={event => {setFingerprint(event.target.value); setIndependent(false);}} placeholder="SHA256:…"/></label><label>Existing SSH account<input aria-label="OpenWrt SSH username" autoComplete="off" maxLength={32} disabled={working || expired} value={username} onChange={event => setUsername(event.target.value)} /></label><label className="target-consent"><input type="checkbox" checked={independent} disabled={working || expired} onChange={event => setIndependent(event.target.checked)}/>I independently checked this fingerprint and authorize one identity/authentication handshake with this SSH account.</label><p className="muted small">Next, Windows asks you to select an existing private key. Key content and local paths stay outside the interface. Password login and interactive key prompts are unavailable.</p><button className="primary-button" disabled={working || expired || !independent || !hostFingerprintSchema.safeParse(fingerprint).success || fingerprint !== status.hostFingerprint || !/^[a-z_][a-z0-9_-]{0,31}$/.test(username)} onClick={() => {const verified = fingerprint; setFingerprint(""); setIndependent(false); void operation(() => openWrtConnect(verified, username));}}>Select SSH key and authenticate</button></> : null}
        </section> : null}
        {authenticated && status.identity ? <section className="openwrt-identity" aria-label="Authenticated OpenWrt identity"><h3>Authenticated device identity</h3><dl><div><dt>Model / board</dt><dd>{status.identity.model} / {status.identity.boardName}</dd></div><div><dt>Hardware revision</dt><dd>{status.identity.hardwareRevision ?? "Unknown"}</dd></div><div><dt>Firmware</dt><dd>{status.identity.firmwareFamily} {status.identity.firmwareVersion} · {status.identity.buildRevision}</dd></div><div><dt>Target</dt><dd>{status.identity.target}</dd></div></dl><p className="muted small">Authenticated server-reported identity. Physical hardware qualification remains separate.</p>
          {!status.profileQualified ? <p role="status">Identity inspection only. No exact qualified profile matches this firmware and board; configuration reads are unavailable.</p> : !status.readApproved || status.state === "inspected" ? <><label className="target-consent"><input type="checkbox" checked={readConsent} disabled={working || expired} onChange={event => setReadConsent(event.target.checked)}/>I separately approve the reviewed configuration and firmware-metadata reads for this authenticated target.</label><button className="secondary-button" disabled={working || expired || !readConsent} onClick={() => {setReadConsent(false); void operation(openWrtApproveReads);}}>Authorize read-only inspection</button>{status.state === "inspected" ? <p className="muted small">Each new inspection requires fresh read approval. Results below are the last acknowledged observations.</p> : null}</> : <button className="primary-button" disabled={working || expired} onClick={() => void operation(openWrtInspect)}>Read configuration and firmware feasibility</button>}
        </section> : null}
        <div className="openwrt-actions">{working ? <><span role="status">Running bounded OpenWrt operation…</span><button className="secondary-button" onClick={cancel}>Cancel OpenWrt operation</button></> : null}{locked ? <button className="secondary-button" onClick={() => {setOwnership(false); setReadConsent(false); setIndependent(false); setFingerprint(""); void operation(openWrtDisconnect);}}>Disconnect OpenWrt</button> : null}</div>
        {status.inspection && status.readApproved && !expired ? <><ConfigurationInspection inspection={status.inspection}/><FirmwareOperations report={status.inspection.firmware}/></> : null}
      </>}
    </section>
  </dialog>;
}
