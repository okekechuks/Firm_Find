import { useEffect, useId, useRef, useState } from "react";
import { Search, Usb, RefreshCw, X, LockKeyhole } from "lucide-react";
import { deviceStatus, extractionEligibility } from "../lib/device-status";
import { errorMessage, getDeviceAdapterMatches, getDeviceCompatibility } from "../lib/backend";
import type { Device, DiscoverySnapshot } from "../types/contracts";

const labels = {usb: "USB", serial: "Serial", network: "Network"};
export function Devices({ inventory, desktop, error, refreshing, onRefresh, onCancel }: {
  inventory: DiscoverySnapshot | null; desktop: boolean; error: string | null; refreshing: boolean;
  onRefresh: () => Promise<void>; onCancel?: () => Promise<void>;
}) {
  const [advanced, setAdvanced] = useState(false);
  const [query, setQuery] = useState("");
  const [transport, setTransport] = useState("all");
  const [showDisconnected, setShowDisconnected] = useState(false);
  const [infoId, setInfoId] = useState<string | null>(null);
  const [checks, setChecks] = useState<Record<string, number>>({});
  const devices = inventory?.devices ?? [];
  const revision = inventory?.revision ?? 0;
  const matches = devices.filter(device =>
    (showDisconnected || device.connected) &&
    (transport === "all" || device.transports.includes(transport as "usb" | "serial" | "network")) &&
    [device.displayName, device.vendorId, device.productId, ...device.interfaces.map(item => item.portName ?? "")]
      .join(" ").toLowerCase().includes(query.toLowerCase().trim()));
  const candidates = matches.filter(device => ["router_mifi_candidate", "usb_modem_candidate", "network_gateway_candidate"].includes(device.category ?? ""));
  const diagnostics = matches.filter(device => !candidates.includes(device));
  const infoDevice = devices.find(device => device.id === infoId);
  const waiting = desktop && !error && (!inventory || inventory.status === "starting");
  const card = (device: Device) => {
    const tone = checks[device.id] === revision || deviceStatus(device).tone === "error" ? "error" : "warning";
    return <article key={device.id} className={`panel device-card device-${tone}${device.connected ? "" : " disconnected"}`} aria-label={`${device.displayName}: ${tone === "error" ? "firmware extraction unavailable" : "compatibility not verified"}`}>
      <h3>{device.displayName || "Unknown device"}</h3>
      <p className="device-connection">{device.transports.map(type => labels[type]).join(" / ") || "Unknown connection"}{!device.connected ? " · Disconnected" : ""}</p>
      <button className="secondary-button" onClick={() => setInfoId(device.id)} aria-label={`More info for ${device.displayName || "unknown device"}`}>More info</button>
    </article>;
  };
  return <>
    {!desktop ? <section className="panel device-empty"><Usb size={32}/><h2>Open the Windows app to discover devices</h2><p>Browser preview has no hardware inventory.</p></section> : <>
      {(error || inventory?.warnings.length) ? <div className="inventory-warning" role="alert">{error ? <p>{error}</p> : null}{inventory?.warnings.map(warning => <p key={warning}>{warning}</p>)}</div> : null}
      <div className="device-toolbar">
        <label className="device-search"><Search size={16}/><input aria-label="Search devices" value={query} onChange={event => setQuery(event.target.value)} placeholder="Search name, USB ID, or COM port"/></label>
        <select aria-label="Filter by connection" value={transport} onChange={event => setTransport(event.target.value)}><option value="all">All connections</option><option value="usb">USB</option><option value="serial">Serial</option><option value="network">Network</option></select>
        <label className="disconnected-toggle"><input type="checkbox" checked={showDisconnected} onChange={event => setShowDisconnected(event.target.checked)}/>Show disconnected</label>
        <button className="secondary-button" disabled={refreshing || waiting} onClick={() => void onRefresh()}><RefreshCw size={15} className={refreshing ? "spin" : undefined}/>{refreshing ? "Refreshing…" : "Refresh devices"}</button>
        {refreshing && onCancel ? <button className="secondary-button" onClick={() => void onCancel()}>Cancel refresh</button> : null}
      </div>
      {inventory?.cancelled ? <p role="status">Discovery cancelled. Refresh to try again.</p> : null}
      {waiting ? <div className="panel device-empty" role="status">Reading Windows device inventory…</div> : candidates.length ? <div className="device-grid">{candidates.map(card)}</div> :
        <section className="panel device-empty"><Usb size={32}/><h2>{error || inventory?.status === "failed" ? "Inventory unavailable" : devices.length === 0 ? "No devices to show" : "No connected router or MiFi found"}</h2><p>Connect and turn on your device. Windows device changes update this list automatically when automatic device refresh is enabled.</p></section>}
      <details className="other-interfaces"><summary>Windows diagnostics</summary><label className="disconnected-toggle"><input type="checkbox" checked={advanced} onChange={event => setAdvanced(event.target.checked)}/>Show advanced Windows diagnostics and unrelated devices</label>{advanced ? <div className="device-grid">{diagnostics.map(card)}</div> : null}</details>
      {infoDevice ? <DeviceDetails key={infoDevice.id} device={infoDevice} revision={revision} onClose={() => setInfoId(null)} onChecked={() => setChecks(current => ({...current, [infoDevice.id]: revision}))}/> : null}
    </>}
  </>;
}

function DeviceDetails({device, revision, onClose, onChecked}: {device: Device; revision: number; onClose: () => void; onChecked: () => void}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const generation = useRef(0);
  const [state, setState] = useState<"idle" | "checking" | "unavailable" | "error">("idle");
  const [message, setMessage] = useState("");
  useEffect(() => {
    const element = dialog.current;
    const previous = document.activeElement as HTMLElement | null;
    element?.showModal();
    return () => {element?.close(); previous?.focus();};
  }, []);
  useEffect(() => {const requests=generation; requests.current++; setState("idle"); setMessage(""); return () => {requests.current++;};}, [revision, device.connected]);
  const verify = async () => {
    const run = ++generation.current;
    setState("checking"); setMessage("");
    let timeout: ReturnType<typeof setTimeout> | undefined;
    try {
      const [profile, adapters] = await Promise.race([
        Promise.all([getDeviceCompatibility(device.id), getDeviceAdapterMatches(device.id)]),
        new Promise<never>((_, reject) => {timeout = setTimeout(() => reject(new Error("Compatibility check timed out. Try again.")), 15000);}),
      ]);
      if (run !== generation.current) return;
      if (profile.deviceId !== device.id || adapters.deviceId !== device.id || profile.discoveryRevision !== revision || adapters.discoveryRevision !== revision || profile.expiresAt <= Date.now() || adapters.expiresAt <= Date.now()) throw new Error("Device information changed. Refresh devices and try again.");
      // No acquisition implementation or extraction IPC exists in production yet.
      // Identity/configuration support cannot grant firmware acquisition.
      setMessage(extractionEligibility(device).reason);
      setState("unavailable"); onChecked();
    } catch (failure) {
      if (run === generation.current) {setState("error"); setMessage(errorMessage(failure));}
    } finally {if (timeout) clearTimeout(timeout);}
  };
  const addresses = [...new Set(device.networks.flatMap(network => network.addresses))];
  const dns = [...new Set(device.networks.flatMap(network => network.dnsServers ?? []))];
  const routes = [...new Set(device.networks.flatMap(network => network.defaultRoutes?.map(route => route.gateway) ?? network.gateways))];
  return <dialog ref={dialog} className="device-info-dialog device-summary-dialog" aria-labelledby={titleId} onCancel={event => {event.preventDefault(); onClose();}}>
    <div className="section-heading"><h2 id={titleId}>{device.displayName}</h2><button className="icon-button" aria-label="Close device information" onClick={onClose}><X size={18}/></button></div>
    <section className="device-network-summary" aria-label="Network info"><h3>Network info</h3><dl><div><dt>Local addresses</dt><dd>{addresses.join(", ") || "Not reported"}</dd></div><div><dt>DNS</dt><dd>{dns.join(", ") || "Not reported"}</dd></div><div><dt>Default routes</dt><dd>{routes.join(", ") || "Not reported"}</dd></div></dl></section>
    {state === "checking" ? <progress className="compatibility-progress" aria-label="Verifying firmware extraction compatibility"/> : null}
    {state === "unavailable" ? <div className="inventory-warning" role="alert"><strong>Cannot locate firmware</strong><p>{message}</p></div> : null}
    {state === "error" ? <p role="alert">{message}</p> : null}
    {!device.connected ? <p role="status">Device disconnected. Reconnect it to verify compatibility.</p> : null}
    <div className="dialog-actions"><button className="secondary-button" disabled={!device.connected || state === "checking"} onClick={() => void verify()}>Verify Compatibility</button><button className="primary-button" disabled><LockKeyhole size={15}/>Extract Firmware</button></div>
  </dialog>;
}
