import { useState } from "react";
import { Cable, Network, Search, ShieldCheck, Usb, RefreshCw } from "lucide-react";
import type { Device, DiscoverySnapshot } from "../types/contracts";

const labels = {usb: "USB", serial: "Serial", network: "Network"};
export function Devices({ inventory, desktop, error, refreshing, onRefresh }: {
  inventory: DiscoverySnapshot | null;
  desktop: boolean;
  error: string | null;
  refreshing: boolean;
  onRefresh: () => Promise<void>;
}) {
  const [query, setQuery] = useState("");
  const [transport, setTransport] = useState("all");
  const [showDisconnected, setShowDisconnected] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const devices = inventory?.devices ?? [];
  const visible = devices.filter((device) =>
    (showDisconnected || device.connected) &&
    (transport === "all" || device.transports.includes(transport as "usb" | "serial" | "network")) &&
    [device.displayName, device.vendorId, device.productId, ...device.interfaces.map((item) => item.portName ?? "")]
      .join(" ").toLowerCase().includes(query.toLowerCase().trim()),
  );
  const selected = devices.find((device) => device.id === selectedId);
  const present = devices.filter((device) => device.connected).length;
  const waiting = desktop && (!inventory || inventory.status === "starting");
  return (
    <>
      <section className="panel inventory-summary">
        <div><span className="eyebrow">PASSIVE DISCOVERY</span><h2>What Windows can see</h2>
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
        </div>
        <div className="section-heading"><h2>{present} present · {devices.length - present} disconnected</h2><span className="muted small">{inventory?.scannedAt ? `Last checked ${new Date(inventory.scannedAt).toLocaleTimeString()}` : "Waiting for first successful scan"}</span></div>
        {waiting ? <div className="panel device-empty" role="status">Reading Windows device inventory…</div> : visible.length === 0 ? <section className="panel device-empty"><Usb size={32}/><h2>{devices.length === 0 ? "No devices to show" : "No matching devices"}</h2><p>{devices.length === 0 ? "Connect a USB device or refresh the inventory. A failed scan keeps the last successful result." : "Try another search or connection filter."}</p></section> :
          <div className="device-grid">{visible.map((device) => <DeviceCard key={device.id} device={device} selected={selectedId === device.id} onSelect={() => setSelectedId(device.id)}/>)}</div>}
        {selected ? <DeviceDetails device={selected}/> : null}
      </>}
      <div className="safety-note"><ShieldCheck size={17}/><span>Detection does not establish firmware access. No device commands or network probes are sent. Gateway identity and ownership remain unverified.</span></div>
    </>
  );
}

function DeviceCard({device, selected, onSelect}: {device: Device; selected: boolean; onSelect: () => void}) {
  const Icon = device.transports.includes("usb") ? Usb : device.transports.includes("serial") ? Cable : Network;
  return <button className={`panel device-card${selected ? " selected" : ""}${device.connected ? "" : " disconnected"}`} aria-pressed={selected} onClick={onSelect}>
    <div className="device-card-top"><Icon size={23}/><span className={`device-state${device.connected ? " present" : ""}`}>{device.connected ? "Present" : "Disconnected"}</span></div>
    <h3>{device.displayName || "Unknown Windows device"}</h3><p>{device.assessment}</p>
    <div className="device-tags">{device.transports.map((type) => <span key={type}>{labels[type]}</span>)}{device.interfaces.filter((item) => item.portName).map((item) => <span key={item.instanceId}>{item.portName}</span>)}</div>
    <span className="device-card-bottom">{device.vendorId && device.productId ? `VID ${device.vendorId} · PID ${device.productId}` : "Identity not verified"}<span>Details →</span></span>
  </button>;
}

function DeviceDetails({device}: {device: Device}) {
  return <section className="panel device-details" aria-label="Device details">
    <div className="section-heading"><h2>{device.displayName}</h2><span className="muted small">{device.connected ? "Currently present" : `Last seen ${new Date(device.lastSeenAt).toLocaleString()}`}</span></div>
    <div className="device-detail-grid">
      <div><h3>Identification</h3><p>{device.assessment}</p><p>{device.confidence}% confidence in the device-type hint. This is not verified model support.</p><ul>{device.evidence.map((item) => <li key={item}>{item}</li>)}</ul>
        <dl className="capability-list"><div><dt>Inventory</dt><dd>{device.capabilities.inventory}</dd></div><div><dt>Firmware backup</dt><dd>{device.capabilities.backup}</dd></div><div><dt>Firmware installation</dt><dd>{device.capabilities.flash}</dd></div></dl>
      </div>
      <div><h3>Interfaces</h3>{device.interfaces.length === 0 ? <p>No matching Windows device record.</p> : device.interfaces.map((item) => <details key={item.instanceId} className="interface-detail">
        <summary>{item.className || "Device"}{item.portName ? ` · ${item.portName}` : ""} · {item.present ? "Present" : "Disconnected"}</summary>
        <dl><dt>Instance</dt><dd>{item.instanceId}</dd><dt>Parent</dt><dd>{item.parentInstanceId ?? "Not reported"}</dd><dt>Setup class</dt><dd>{item.classGuid}</dd><dt>Interface classes</dt><dd>{item.interfaceClasses.join(", ") || "Not reported"}</dd><dt>Hardware IDs</dt><dd>{item.hardwareIds.join(", ") || "Not reported"}</dd></dl>
      </details>)}{device.containerId ? <p className="device-container">Container: {device.containerId}</p> : null}</div>
    </div>
    {device.networks.map((network) => <div className="network-detail" key={network.id}><h3>{network.displayName || "Network adapter"} · {network.linkUp ? "Link up" : "Link down"}</h3><p>{network.description}</p><dl><div><dt>Local addresses</dt><dd>{network.addresses.join(", ") || "None reported"}</dd></div><div><dt>Reported gateways</dt><dd>{network.gateways.join(", ") || "None reported"}</dd></div></dl></div>)}
  </section>;
}
