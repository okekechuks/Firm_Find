import { useState } from "react";
import { Cable, Network, Search, ShieldCheck, Usb, RefreshCw } from "lucide-react";
import { ManualRouters, type RouterDraft } from "./ManualRouters";
import type { Device, DiscoverySnapshot } from "../types/contracts";

const labels = {usb: "USB", serial: "Serial", network: "Network"};
const featureLabels = [
  ["apn", "APN mode"],
  ["wifiUserLimit", "Wi-Fi user limit"],
  ["customDns", "Custom DNS"],
  ["ussd", "USSD"],
] as const;
export function Devices({ inventory, desktop, error, refreshing, onRefresh, onCancel }: {
  inventory: DiscoverySnapshot | null;
  desktop: boolean;
  error: string | null;
  refreshing: boolean;
  onRefresh: () => Promise<void>;
  onCancel?: () => Promise<void>;
}) {
  const [draft, setDraft] = useState<RouterDraft | null>(null);
  const [advanced, setAdvanced] = useState(false);
  const [query, setQuery] = useState("");
  const [transport, setTransport] = useState("all");
  const [showDisconnected, setShowDisconnected] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);
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
  const present = devices.filter((device) => device.connected).length;
  const waiting = desktop && !error && (!inventory || inventory.status === "starting");
  const unavailable = Boolean(error || inventory?.status === "failed");
  return (
    <>
      <section className="panel inventory-summary">
        <div><span className="eyebrow">DEVICE DISCOVERY</span><h2>What Windows can see</h2>
          <p>USB devices, serial ports, and network adapters on this computer. Related interfaces are grouped where Windows provides a matching identity.</p></div>
        <span className="inventory-monitor" role="status">
          {!desktop ? "Windows app required" : waiting ? "Reading inventory…" : inventory?.status === "failed" ? "Inventory needs attention" : inventory?.watching && !error ? "Live updates on" : "Periodic updates"}
        </span>
      </section>
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
          <button className="primary-button" onClick={() => setDraft({address: "", interfaceId: null})}>Add Router</button>
        </div>
        {inventory?.cancelled ? <p role="status">Discovery cancelled or superseded. Last successful inventory retained; refresh to try again.</p> : null}
        <label className="disconnected-toggle"><input type="checkbox" checked={advanced} onChange={(e) => setAdvanced(e.target.checked)}/>Show advanced Windows diagnostics and unrelated devices</label>
        <div className="section-heading"><h2>{present} present · {devices.length - present} disconnected</h2><span className="muted small">{inventory?.scannedAt ? `Last checked ${new Date(inventory.scannedAt).toLocaleTimeString()}` : "Waiting for first successful scan"}</span></div>
        {waiting ? <div className="panel device-empty" role="status">Reading Windows device inventory…</div> : visible.length === 0 ? <section className="panel device-empty"><Usb size={32}/><h2>{devices.length === 0 ? (unavailable ? "Inventory unavailable" : "No devices to show") : "No relevant candidates"}</h2><p>{devices.length === 0 ? (unavailable ? "Windows inventory could not be read. Use Refresh devices to try again." : "Connect a USB device or refresh the inventory. A failed scan keeps the last successful result.") : "Try another search or connection filter, add a router manually, or open advanced Windows diagnostics."}</p></section> :
          <>{candidateDevices.length > 0 ? <section aria-label="Router and MiFi candidates"><h2 className="device-group-heading">Router / MiFi candidates | identity unverified</h2><div className="device-grid">{candidateDevices.map((device) => <DeviceCard key={device.id} device={device} selected={selectedId === device.id} onSelect={() => setSelectedId(device.id)}/>)}</div></section> : <p className="muted">No router candidates identified. Generic interfaces do not establish router identity. Add a management address manually.</p>}
          {otherDevices.length > 0 ? <details className="other-interfaces" open={advanced}><summary>Other Windows interfaces{advanced ? " and diagnostics" : ""} ({otherDevices.length})</summary><div className="device-grid">{otherDevices.map((device) => <DeviceCard key={device.id} device={device} selected={selectedId === device.id} onSelect={() => setSelectedId(device.id)}/>)}</div></details> : null}</>}
        {selected ? <DeviceDetails device={selected} onGateway={(address, interfaceId) => setDraft({address, interfaceId})}/> : null}
        <ManualRouters devices={devices} draft={draft} onClose={() => setDraft(null)}/>
      </>}
      <div className="safety-note"><ShieldCheck size={17}/><span>Detection does not establish firmware access. Network checks require your action and ownership confirmation. Gateway identity and ownership remain unverified.</span></div>
    </>
  );
}

function DeviceCard({device, selected, onSelect}: {device: Device; selected: boolean; onSelect: () => void}) {
  const Icon = device.transports.includes("usb") ? Usb : device.transports.includes("serial") ? Cable : Network;
  return <button className={`panel device-card${selected ? " selected" : ""}${device.connected ? "" : " disconnected"}`} aria-pressed={selected} onClick={onSelect}>
    <div className="device-card-top"><Icon size={23}/><span className={`device-state${device.connected ? " present" : ""}`}>{device.connected ? "Present" : "Disconnected"}</span></div>
    <h3>{device.displayName || "Unknown Windows device"}</h3><p>{device.assessment}</p><p className="muted small">{device.category?.replaceAll("_", " ") ?? "Unknown"} | {device.identity?.confidence ?? "unknown"} identity | {device.lifecycle ?? "discovered"}</p>
    <div className="device-tags">{device.transports.map((type) => <span key={type}>{labels[type]}</span>)}{device.interfaces.filter((item) => item.portName).map((item) => <span key={item.instanceId}>{item.portName}</span>)}</div>
    <span className="device-card-bottom">{device.vendorId && device.productId ? `VID ${device.vendorId} · PID ${device.productId}` : "Identity not verified"}<span>Details →</span></span>
  </button>;
}

function DeviceDetails({device, onGateway}: {device: Device; onGateway: (address: string, interfaceId: string) => void}) {
  return <section className="panel device-details" aria-label="Device details">
    <div className="section-heading"><h2>{device.displayName}</h2><span className="muted small">{device.connected ? "Currently present" : `Last seen ${new Date(device.lastSeenAt).toLocaleString()}`}</span></div>
    <div className="device-detail-grid">
      <div><h3>Identification</h3><p>{device.assessment}</p><p>Identity confidence: <strong>{device.identity?.confidence ?? "unknown"}</strong>. {device.identity?.explanation ?? "No supported management identity response has been collected."}</p>
        <p>Verified manufacturer: {device.identity?.manufacturer ?? "Unknown"} | Model: {device.identity?.model ?? "Unknown"} | Firmware: {device.identity?.firmware ?? "Unknown"}</p>
        <ul>{device.identity?.observations.map((item, index) => <li key={index}>{item.kind.replaceAll("_", " ")}: {item.value} | {item.source} | {item.reliability}</li>)}</ul><ul>{device.evidence.map((item) => <li key={item}>{item}</li>)}</ul>
        <dl className="capability-list"><div><dt>Inventory</dt><dd>{device.capabilities.inventory}</dd></div><div><dt>Firmware backup</dt><dd>{device.capabilities.backup}</dd></div><div><dt>Firmware installation</dt><dd>{device.capabilities.flash}</dd></div></dl>
        <h3>Feature support | pending Phase 3</h3>
        <dl className="capability-list">{featureLabels.map(([key, label]) => {
          const feature = device.capabilities.features[key];
          return <div key={key}><dt>{label}</dt><dd>{feature.state}{feature.controlPlane ? ` · ${feature.controlPlane}` : ""}</dd></div>;
        })}</dl>
        <p>Support stays unknown until a verified device profile and method provide evidence.</p>
      </div>
      <div><h3>Interfaces</h3>{device.interfaces.length === 0 ? <p>No matching Windows device record.</p> : device.interfaces.map((item) => <details key={item.instanceId} className="interface-detail">
        <summary>{item.className || "Device"}{item.portName ? ` · ${item.portName}` : ""} · {item.present ? "Present" : "Disconnected"}</summary>
        <dl><dt>Instance</dt><dd>{maskIdentifier(item.instanceId)}</dd><dt>Parent</dt><dd>{item.parentInstanceId ? maskIdentifier(item.parentInstanceId) : "Not reported"}</dd><dt>Setup class</dt><dd>{item.classGuid}</dd><dt>Interface classes</dt><dd>{item.interfaceClasses.join(", ") || "Not reported"}</dd><dt>Windows manufacturer</dt><dd>{item.manufacturer ?? "Not reported"} (unverified device identity)</dd><dt>Compatible IDs</dt><dd>{item.compatibleIds?.join(", ") || "Not reported"}</dd><dt>Interface paths (serial portion hidden)</dt><dd>{item.interfacePaths?.map(maskIdentifier).join(", ") || "Not reported"}</dd><dt>Hardware IDs</dt><dd>{item.hardwareIds.join(", ") || "Not reported"}</dd></dl>
      </details>)}{device.containerId ? <p className="device-container">Container: {device.containerId}</p> : null}</div>
    </div>
    {device.networks.map((network) => <div className="network-detail" key={network.id}><h3>{network.displayName || "Network adapter"} · {network.linkUp ? "Link up" : "Link down"}</h3><p>{network.description}</p><dl><div><dt>Local addresses</dt><dd>{network.addresses.join(", ") || "None reported"}</dd></div><div><dt>Reported gateways</dt><dd>{network.gateways.join(", ") || "None reported"}</dd></div></dl><dl><div><dt>Windows indexes / LUID</dt><dd>{network.interfaceIndex} / {network.ipv6Index ?? "Unknown"} / {network.luid ?? "Unknown"}</dd></div><div><dt>Local prefixes</dt><dd>{network.prefixes?.map((p) => `${p.address}/${p.length}`).join(", ") || "None reported"}</dd></div><div><dt>DNS servers</dt><dd>{network.dnsServers?.join(", ") || "None reported"}</dd></div><div><dt>Default routes (route metric offset)</dt><dd>{network.defaultRoutes?.map((r) => `${r.gateway} | ${r.family} | ${r.metric}`).join(", ") || "None reported"}</dd></div><div><dt>Windows interface kind</dt><dd>{network.virtualInterface ? "Virtual / software interface" : "Physical or unclassified"}</dd></div></dl>
      {network.linkUp && !network.virtualInterface ? <div className="router-actions">{network.gateways.map((gateway) => <button className="secondary-button" key={gateway} onClick={() => onGateway(gateway.split("%")[0], network.id)}>Use gateway {gateway}</button>)}</div> : null}
      </div>)}
  </section>;
}

function maskIdentifier(value: string) {
  // VID/PID and class evidence remain visible; potential device serial segment stays hidden.
  const separator = value.includes("#") ? "#" : "\\";
  const parts = value.split(separator);
  if (parts.length >= 3) parts[2] = "[identifier hidden]";
  return parts.join(separator);
}
