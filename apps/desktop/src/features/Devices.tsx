import { useEffect, useId, useRef, useState } from "react";
import { Cable, Network, Search, ShieldCheck, Usb, RefreshCw, X } from "lucide-react";
import { ManualRouters, type RouterDraft } from "./ManualRouters";
import { CompatibilityPanel } from "./Compatibility";
import { deviceStatus, extractionEligibility } from "../lib/device-status";
import { LockKeyhole } from "lucide-react";
import type { Device, DiscoverySnapshot } from "../types/contracts";

const labels = {usb: "USB", serial: "Serial", network: "Network"};
export function Devices({ inventory, desktop, error, refreshing, onRefresh, onCancel }: {
  inventory: DiscoverySnapshot | null;
  desktop: boolean;
  error: string | null;
  refreshing: boolean;
  onRefresh: () => Promise<void>;
  onCancel?: () => Promise<void>;
}) {
  const [draft, setDraft] = useState<RouterDraft | null>(null);
  const [manualOpen, setManualOpen] = useState(false);
  const [advanced, setAdvanced] = useState(false);
  const [query, setQuery] = useState("");
  const [transport, setTransport] = useState("all");
  const [showDisconnected, setShowDisconnected] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [infoId, setInfoId] = useState<string | null>(null);
  const devices = inventory?.devices ?? [];
  const matches = devices.filter((device) =>
    (showDisconnected || device.connected) &&
    (transport === "all" || device.transports.includes(transport as "usb" | "serial" | "network")) &&
    [device.displayName, device.vendorId, device.productId, ...device.interfaces.map((item) => item.portName ?? "")]
      .join(" ").toLowerCase().includes(query.toLowerCase().trim()),
  );
  const visible = matches.filter((device) => advanced || !["unrelated_device", "unknown"].includes(device.category ?? "unknown"));
  const candidateDevices = visible.filter((device) => ["router_mifi_candidate", "usb_modem_candidate", "network_gateway_candidate"].includes(device.category ?? "unknown"));
  const otherDevices = visible.filter((device) => !candidateDevices.includes(device));
  const selected = devices.find((device) => device.id === selectedId);
  const infoDevice = devices.find((device) => device.id === infoId);
  const present = devices.filter((device) => device.connected).length;
  const waiting = desktop && !error && (!inventory || inventory.status === "starting");
  const unavailable = Boolean(error || inventory?.status === "failed");
  return (
    <>
      {!desktop ? <section className="panel device-empty"><Usb size={32}/><h2>Open the Windows app to discover devices</h2><p>Browser preview has no hardware inventory.</p></section> : <>
        {(error || (inventory?.warnings.length ?? 0) > 0) ? <div className="inventory-warning" role="alert">
          {error ? <p>{error}</p> : null}
          {inventory?.warnings.map((warning) => <p key={warning}>{warning}</p>)}
        </div> : null}
        <div className="device-toolbar">
          <label className="device-search"><Search size={16}/><input aria-label="Search devices" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search name, USB ID, or COM port" /></label>
          <select aria-label="Filter by connection" value={transport} onChange={(event) => setTransport(event.target.value)}>
            <option value="all">All connections</option><option value="usb">USB</option><option value="serial">Serial</option><option value="network">Network</option>
          </select>
          <label className="disconnected-toggle"><input type="checkbox" checked={showDisconnected} onChange={(event) => setShowDisconnected(event.target.checked)} />Show disconnected</label>
          <button className="secondary-button" disabled={refreshing || waiting} onClick={() => void onRefresh()}><RefreshCw size={15} className={refreshing ? "spin" : undefined}/>{refreshing ? "Refreshing…" : "Refresh devices"}</button>
          {refreshing && onCancel ? <button className="secondary-button" onClick={() => void onCancel()}>Cancel refresh</button> : null}
          <button className="primary-button" onClick={() => {setDraft({address: "", interfaceId: null}); setManualOpen(true);}}>Add Router</button>
          <button className="secondary-button" onClick={() => setManualOpen(true)}>Manage saved connections</button>
        </div>
        {inventory?.cancelled ? <p role="status">Discovery cancelled or superseded. Last successful inventory retained; refresh to try again.</p> : null}
        <label className="disconnected-toggle"><input type="checkbox" checked={advanced} onChange={(e) => setAdvanced(e.target.checked)}/>Show advanced Windows diagnostics and unrelated devices</label>
        <div className="section-heading"><h2>{present} present · {devices.length - present} disconnected</h2><span className="muted small">{inventory?.scannedAt ? `Last checked ${new Date(inventory.scannedAt).toLocaleTimeString()}` : "Waiting for first successful scan"}</span></div>
        {waiting ? <div className="panel device-empty" role="status">Reading Windows device inventory…</div> : visible.length === 0 ? <section className="panel device-empty"><Usb size={32}/><h2>{devices.length === 0 ? (unavailable ? "Inventory unavailable" : "No devices to show") : "No relevant candidates"}</h2><p>{devices.length === 0 ? (unavailable ? "Windows inventory could not be read. Use Refresh devices to try again." : "Connect a USB device or refresh the inventory. A failed scan keeps the last successful result.") : "Try another search or connection filter, add a router manually, or open advanced Windows diagnostics."}</p></section> :
          <>{candidateDevices.length > 0 ? <section aria-label="Router and MiFi candidates"><h2 className="device-group-heading">Router / MiFi candidates | identity unverified</h2><div className="device-grid">{candidateDevices.map((device) => <DeviceCard key={device.id} device={device} selected={selectedId === device.id} onSelect={() => setSelectedId(device.id)} onMoreInfo={() => setInfoId(device.id)}/>)}</div></section> : <p className="muted">No router candidates identified. Generic interfaces do not establish router identity. Add a management address manually.</p>}
          {otherDevices.length > 0 ? <details className="other-interfaces" open={advanced}><summary>Other Windows interfaces{advanced ? " and diagnostics" : ""} ({otherDevices.length})</summary><div className="device-grid">{otherDevices.map((device) => <DeviceCard key={device.id} device={device} selected={selectedId === device.id} onSelect={() => setSelectedId(device.id)} onMoreInfo={() => setInfoId(device.id)}/>)}</div></details> : null}</>}
        {infoDevice ? <DeviceDetails device={infoDevice} onClose={() => setInfoId(null)} onGateway={(address, interfaceId) => {setDraft({address, interfaceId}); setManualOpen(true); setInfoId(null);}}/> : null}
        {selected ? <CompatibilityPanel key={selected.id} deviceId={selected.id} revision={inventory?.revision ?? 0} desktop={desktop}/> : null}
        <ManualRouters devices={devices} draft={draft} open={manualOpen} onClose={() => {setDraft(null); setManualOpen(false);}} onDraftClose={() => setDraft(null)}/>
      </>}
      <div className="safety-note"><ShieldCheck size={17}/><span>Detection does not establish firmware access. Network checks require your action and ownership confirmation. Gateway identity and ownership remain unverified.</span></div>
    </>
  );
}

function DeviceCard({device, selected, onSelect, onMoreInfo}: {device: Device; selected: boolean; onSelect: () => void; onMoreInfo: () => void}) {
  const classification = deviceStatus(device);
  const Icon = device.transports.includes("usb") ? Usb : device.transports.includes("serial") ? Cable : Network;
  return <article className={`panel device-card device-${classification.tone}${selected ? " selected" : ""}${device.connected ? "" : " disconnected"}`}>
    <div className="device-card-top"><Icon size={23}/><span className={`device-state${device.connected ? " present" : ""}`}>{device.connected ? "Present" : "Disconnected"}</span></div>
    <h3>{device.displayName || "Unknown Windows device"}</h3><span className={`badge ${classification.tone}`}>{classification.label}</span>
    <div className="device-tags">{device.transports.map((type) => <span key={type}>{labels[type]}</span>)}</div>
    <div className="device-card-actions"><button className="secondary-button" onClick={onMoreInfo} aria-label={`More info for ${device.displayName || "unknown device"}`}>More info</button><button className="text-button" onClick={onSelect} aria-label={`Check compatibility for ${device.displayName || "unknown device"}`} aria-pressed={selected}>{selected ? "Selected for compatibility" : "Check compatibility"}</button></div>
  </article>;
}

function DeviceDetails({device, onClose, onGateway}: {device: Device; onClose: () => void; onGateway: (address: string, interfaceId: string) => void}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const titleId = useId();
  const eligibility=extractionEligibility(device);
  useEffect(() => {
    const element = dialog.current;
    const previous = document.activeElement as HTMLElement | null;
    element?.showModal();
    closeButton.current?.focus();
    return () => { element?.close(); previous?.focus(); };
  }, []);
  return <dialog ref={dialog} className="device-info-dialog" aria-labelledby={titleId} onCancel={(event) => {event.preventDefault(); onClose();}} onClick={(event) => {if (event.target === event.currentTarget) onClose();}}>
    <section className="device-details" aria-label="Device details">
    <div className="section-heading"><div><h2 id={titleId}>{device.displayName}</h2><span className="muted small">{device.connected ? "Currently present" : `Last seen ${new Date(device.lastSeenAt).toLocaleString()}`}</span></div><button ref={closeButton} className="icon-button" aria-label="Close device information" onClick={onClose}><X size={18}/></button></div>
    <div className="extraction-unavailable" aria-label="Firmware extraction"><div><h3>Firmware extraction</h3><p>{eligibility.reason}</p></div><button className="primary-button" disabled={!eligibility.allowed}><LockKeyhole size={15}/>Extract Firmware</button></div>
    <div className="device-detail-grid">
      <div><h3>Identification</h3><p>{device.assessment}</p><p>Identity confidence: <strong>{device.identity?.confidence ?? "unknown"}</strong>. {device.identity?.explanation ?? "No supported management identity response has been collected."}</p>
        <p>Verified manufacturer: {device.identity?.manufacturer ?? "Unknown"} | Model: {device.identity?.model ?? "Unknown"} | Firmware: {device.identity?.firmware ?? "Unknown"}</p>
        <ul>{device.identity?.observations.map((item, index) => <li key={index}>{item.kind.replaceAll("_", " ")}: {item.value} | {item.source} | {item.reliability}</li>)}</ul><ul>{device.evidence.map((item) => <li key={item}>{item}</li>)}</ul>
        <dl className="capability-list"><div><dt>Inventory</dt><dd>{device.capabilities.inventory}</dd></div><div><dt>Firmware backup</dt><dd>{device.capabilities.backup}</dd></div><div><dt>Firmware installation</dt><dd>{device.capabilities.flash}</dd></div></dl>
        <p>Feature support is evaluated below from current evidence and exact device profiles.</p>
      </div>
      <div><h3>Interfaces</h3>{device.interfaces.length === 0 ? <p>No matching Windows device record.</p> : device.interfaces.map((item) => <details key={item.instanceId} className="interface-detail">
        <summary>{item.className || "Device"}{item.portName ? ` · ${item.portName}` : ""} · {item.present ? "Present" : "Disconnected"}</summary>
        <dl><dt>Instance</dt><dd>{maskIdentifier(item.instanceId)}</dd><dt>Parent</dt><dd>{item.parentInstanceId ? maskIdentifier(item.parentInstanceId) : "Not reported"}</dd><dt>Setup class</dt><dd>{item.classGuid}</dd><dt>Interface classes</dt><dd>{item.interfaceClasses.join(", ") || "Not reported"}</dd><dt>Windows manufacturer</dt><dd>{item.manufacturer ?? "Not reported"} (unverified device identity)</dd><dt>Compatible IDs</dt><dd>{item.compatibleIds?.join(", ") || "Not reported"}</dd><dt>Interface paths (serial portion hidden)</dt><dd>{item.interfacePaths?.map(maskIdentifier).join(", ") || "Not reported"}</dd><dt>Hardware IDs</dt><dd>{item.hardwareIds.join(", ") || "Not reported"}</dd></dl>
      </details>)}{device.containerId ? <p className="device-container">Container: {device.containerId}</p> : null}</div>
    </div>
    {device.networks.map((network) => <div className="network-detail" key={network.id}><h3>{network.displayName || "Network adapter"} · {network.linkUp ? "Link up" : "Link down"}</h3><p>{network.description}</p><dl><div><dt>Local addresses</dt><dd>{network.addresses.join(", ") || "None reported"}</dd></div><div><dt>Reported gateways</dt><dd>{network.gateways.join(", ") || "None reported"}</dd></div></dl><dl><div><dt>Windows indexes / LUID</dt><dd>{network.interfaceIndex} / {network.ipv6Index ?? "Unknown"} / {network.luid ?? "Unknown"}</dd></div><div><dt>Local prefixes</dt><dd>{network.prefixes?.map((p) => `${p.address}/${p.length}`).join(", ") || "None reported"}</dd></div><div><dt>DNS servers</dt><dd>{network.dnsServers?.join(", ") || "None reported"}</dd></div><div><dt>Default routes (route metric offset)</dt><dd>{network.defaultRoutes?.map((r) => `${r.gateway} | ${r.family} | ${r.metric}`).join(", ") || "None reported"}</dd></div><div><dt>Windows interface kind</dt><dd>{network.virtualInterface ? "Virtual / software interface" : "Physical or unclassified"}</dd></div></dl>
      {network.linkUp && !network.virtualInterface ? <div className="router-actions">{network.gateways.map((gateway) => <button className="secondary-button" key={gateway} onClick={() => onGateway(gateway.split("%")[0], network.id)}>Use gateway {gateway}</button>)}</div> : null}
      </div>)}
    </section>
  </dialog>;
}

function maskIdentifier(value: string) {
  // VID/PID and class evidence remain visible; potential device serial segment stays hidden.
  const separator = value.includes("#") ? "#" : "\\";
  const parts = value.split(separator);
  if (parts.length >= 3) parts[2] = "[identifier hidden]";
  return parts.join(separator);
}
