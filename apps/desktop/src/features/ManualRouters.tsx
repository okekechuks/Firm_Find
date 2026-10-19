import { useEffect, useId, useRef, useState } from "react";
import { addManualRouter, cancelEndpointProbe, errorMessage, getManualRouters, probeManagementEndpoint, removeManualRouter } from "../lib/backend";
import type { Device, EndpointResult, ManualRouterInput, SavedRouter } from "../types/contracts";
import { AdapterStatus } from "./AdapterStatus";

export type RouterDraft = {address: string; interfaceId: string | null};
export function ManualRouters({devices, draft, open = true, onClose, onDraftClose = onClose}: {devices: Device[]; draft: RouterDraft | null; open?: boolean; onClose: () => void; onDraftClose?: () => void}) {
  const [routers, setRouters] = useState<SavedRouter[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<Record<string, boolean>>({});
  const [consent, setConsent] = useState<Record<string, boolean>>({});
  const [observations, setObservations] = useState<Record<string, EndpointResult>>({});
  const [notices, setNotices] = useState<Record<string, string>>({});
  const [adapterId, setAdapterId] = useState<string | null>(null);
  const mounted = useRef(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const titleId = useId();
  const generations = useRef(new Map<string, number>());
  const listGeneration = useRef(0);
  const mutations = useRef<Promise<void>>(Promise.resolve());
  useEffect(() => {
    mounted.current = true;
    let active = true;
    const generation = listGeneration.current;
    void getManualRouters().then((saved) => { if (active && generation === listGeneration.current) setRouters(saved); })
      .catch((failure) => { if (active && generation === listGeneration.current) setError(errorMessage(failure)); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; mounted.current = false; };
  }, []);
  useEffect(() => {
    const element = dialog.current;
    if (!open || !element) return;
    const previous = document.activeElement as HTMLElement | null;
    element.showModal();
    closeButton.current?.focus();
    return () => { element.close(); previous?.focus(); };
  }, [open]);
  const acceptList = (saved: SavedRouter[]) => { listGeneration.current++; setRouters(saved); setError(null); setLoading(false); };
  // Serialize UI mutations; response order must agree with SQLite commit order.
  const mutate = (action: () => Promise<SavedRouter[]>) => {
    const next = mutations.current.catch(() => {}).then(async () => {
      const saved = await action();
      if (mounted.current) acceptList(saved);
    });
    mutations.current = next;
    return next;
  };
  const check = async (router: SavedRouter) => {
    const generation = (generations.current.get(router.id) ?? 0) + 1;
    generations.current.set(router.id, generation);
    setBusy((old) => ({...old, [router.id]: true}));
    setNotices((old) => ({...old, [router.id]: "Checking one management endpoint…"}));
    try {
      const result = await probeManagementEndpoint(router.id, consent[router.id] === true);
      if (mounted.current && generations.current.get(router.id) === generation) {
        setObservations((old) => ({...old, [router.id]: result}));
        setNotices((old) => ({...old, [router.id]: result.message}));
      }
    } catch (failure) {
      if (mounted.current && generations.current.get(router.id) === generation) setNotices((old) => ({...old, [router.id]: errorMessage(failure)}));
    } finally {
      if (mounted.current && generations.current.get(router.id) === generation) setBusy((old) => ({...old, [router.id]: false}));
    }
  };
  const cancel = async (id: string) => {
    generations.current.set(id, (generations.current.get(id) ?? 0) + 1);
    try {
      await cancelEndpointProbe(id);
      if (mounted.current) { setBusy((old) => ({...old, [id]: false})); setNotices((old) => ({...old, [id]: "Check cancelled. Previous observation retained."})); }
    } catch (failure) { if (mounted.current) setError(errorMessage(failure)); }
  };
  const remove = async (id: string) => {
    generations.current.set(id, (generations.current.get(id) ?? 0) + 1);
    try { await mutate(() => removeManualRouter(id)); }
    catch (failure) { if (mounted.current) setError(errorMessage(failure)); }
  };
  const networks = devices.filter((d) => d.connected).flatMap((d) => d.networks);
  return <dialog ref={dialog} className="saved-connections-dialog" aria-labelledby={titleId} onCancel={(event) => {event.preventDefault(); onClose();}} onClick={(event) => {if (event.target === event.currentTarget) onClose();}}>
    <section className="panel manual-routers" aria-label="Saved manual connections">
    <div className="section-heading"><div><h2 id={titleId}>Saved manual connections</h2><span className="muted small">Names supplied by you · identity unverified</span></div><button ref={closeButton} className="icon-button" aria-label="Close saved connections" onClick={onClose}><span aria-hidden="true">×</span></button></div>
    <p className="muted small">Saving sends no network request. Checks run only after you confirm ownership. Responses are observations, not verified device identities.</p>
    {draft ? <RouterForm key={`${draft.address}|${draft.interfaceId}`} draft={draft} networks={networks} onClose={onDraftClose} onSave={(input) => mutate(() => addManualRouter(input))}/> : null}
    {loading ? <p role="status">Reading saved connections…</p> : null}
    {error ? <p className="inventory-warning" role="status">{error}</p> : null}
    {!loading && !error && routers.length === 0 ? <p className="muted">No saved routers. Use Add Router or select a candidate gateway.</p> : null}
    {routers.map((router) => {
      const observation = observations[router.id] ?? router.lastResult;
      const unavailable = router.interfaceId !== null && !devices.some((d) => d.connected && d.networks.some((n) => n.id === router.interfaceId && n.linkUp));
      const state = busy[router.id] ? "identifying" : unavailable ? "disconnected" : observations[router.id]?.state ?? router.state;
      return <article className="saved-router" key={router.id} aria-label={router.label || router.address}>
        <h3>{router.label || "Unnamed router candidate"} <span className="muted small">User label</span></h3>
        <p>{router.address}</p>
        <p className="muted small">{state} · Identity unknown{router.deviceId ? " · Associated with selected Windows interface" : " · Interface chosen during check"}</p>
        {observation ? <div className="endpoint-observation"><p><strong>Last check: {observation.code.replaceAll("_", " ")}</strong>{observation.status ? ` · HTTP ${observation.status}` : ""} · {observation.elapsedMs} ms</p>
          <p className="muted small">{new Date(observation.checkedAt).toLocaleString()} · {observation.resolvedAddress ?? "No resolved endpoint recorded"} · TLS: {observation.tls.replaceAll("_", " ")}</p>
          {observation.server ? <p className="muted small">Server header (unverified): {observation.server}</p> : null}
          {!observations[router.id] ? <p className="muted small">Historical observation; check again to confirm current reachability.</p> : null}
        </div> : null}
        {observation?.tls === "untrusted" ? <details className="inventory-warning"><summary>Certificate warning and deliberate trust steps</summary>
          <p>OBTAINER rejected this certificate. Check the router address, clock and certificate name first.</p>
          <ol><li>Obtain the certificate or issuing CA from the device owner through a trusted channel.</li><li>Verify its fingerprint and hostname with the owner. Do not trust a certificate based only on this response.</li><li>If you choose to trust it, use Windows certificate management for your current user to import the verified issuing certificate into the appropriate trust store. This affects other Windows clients too.</li><li>Return here, confirm ownership and retry. OBTAINER always validates certificates and never installs one automatically.</li></ol>
        </details> : null}
        {notices[router.id] ? <p role="status">{notices[router.id]}</p> : observation ? <p className="muted small">{observation.message}</p> : null}
        <label className="target-consent"><input type="checkbox" checked={consent[router.id] ?? false} disabled={busy[router.id]} onChange={(e) => setConsent((old) => ({...old, [router.id]: e.target.checked}))}/>I own or administer this target and allow one read-only check.</label>
        <div className="router-actions"><button className="secondary-button" disabled={!consent[router.id] || busy[router.id]} onClick={() => void check(router)}>Check / Retry</button>
          {busy[router.id] ? <button className="secondary-button" onClick={() => void cancel(router.id)}>Cancel check</button> : null}
          <button className="secondary-button" onClick={() => setAdapterId(old=>old===router.id?null:router.id)} aria-expanded={adapterId===router.id}>Adapter status</button>
          <button className="secondary-button" onClick={() => void remove(router.id)}>Remove saved entry</button></div>
        {adapterId===router.id?<AdapterStatus deviceId={router.id} revision={null} desktop targetKind="saved_endpoint"/>:null}
      </article>;
    })}
    </section>
  </dialog>;
}

function RouterForm({draft, networks, onClose, onSave}: {draft: RouterDraft; networks: Device["networks"]; onClose: () => void; onSave: (input: ManualRouterInput) => Promise<void>}) {
  const [address, setAddress] = useState(draft.address);
  const [label, setLabel] = useState("");
  const [scope, setScope] = useState(draft.interfaceId ?? "");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const active = useRef(false);
  useEffect(() => { active.current = true; input.current?.focus(); return () => { active.current = false; }; }, []);
  const submit = async (event: React.SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    if (!address.trim() || address.length > 2048 || /[\s@?#\\]/.test(address.trim())) { setError("Enter a router address without credentials, paths, queries or fragments."); return; }
    setSaving(true);
    try { await onSave({address: address.trim(), label, interfaceId: scope || null}); if (active.current) onClose(); }
    catch (failure) { if (active.current) setError(errorMessage(failure)); }
    finally { if (active.current) setSaving(false); }
  };
  return <div className="router-form" aria-label="Add router form" onKeyDown={(e) => { if (e.key === "Escape" && !saving) onClose(); }}>
    <form onSubmit={(event) => void submit(event)}>
      <h3>Add Router</h3>
      <label>Management address<input ref={input} required maxLength={2048} value={address} disabled={saving} onChange={(e) => setAddress(e.target.value)} placeholder="192.168.100.1 or https://router.local"/></label>
      <label>Descriptive name (optional)<input maxLength={120} value={label} disabled={saving} onChange={(e) => setLabel(e.target.value)}/></label>
      <label>Router interface<select value={scope} disabled={saving} onChange={(e) => setScope(e.target.value)}><option value="">Choose automatically if unambiguous</option>{networks.map((n) => <option key={n.id} value={n.id}>{n.displayName} · {n.addresses.join(", ")}{n.virtualInterface ? " (virtual; checks unavailable)" : ""}</option>)}</select></label>
      <p className="muted small">HTTP/HTTPS only. Private or link-local targets must match an active physical interface. For IPv6 link-local addresses, select the interface. No login or configuration changes.</p>
      {error ? <p role="alert">{error}</p> : null}
      <div className="router-actions"><button className="primary-button" disabled={saving} type="submit">{saving ? "Saving…" : "Save router"}</button><button className="secondary-button" disabled={saving} type="button" onClick={onClose}>Cancel</button></div>
    </form>
  </div>;
}
