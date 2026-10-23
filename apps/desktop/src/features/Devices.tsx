import { useEffect, useId, useRef, useState } from "react";
import { Search, Usb, RefreshCw, X, LockKeyhole } from "lucide-react";
import { deviceStatus } from "../lib/device-status";
import { approveAuthChallenge, readAuthChallenge, approveStaticMenu, readStaticMenu, approveFirmwareRouteProbe, cancelFirmwareRouteDiscovery, discoverFirmwareRoutes, errorMessage, exportAcquisitionDiagnostics } from "../lib/backend";
import type { Device, DiscoverySnapshot } from "../types/contracts";
import type { AcquisitionReport, CompatibilityStageId } from "../types/acquisition";

const labels = {usb: "USB", serial: "Serial", network: "Network"};
const stageLabels: Record<CompatibilityStageId,string> = {
  detected:"Detected", exact_identity:"Exact identity", management_api:"Management API",
  official_image:"Official image", firmware_export:"Firmware export", physical_read:"Physical read",
  firmware_acquired:"Firmware acquired", filesystem_extracted:"Filesystem extracted",
  customization:"Customization", rebuild:"Rebuild", installation:"Installation",
};
const stageRow = (stage: AcquisitionReport["stages"][number]) => <div className="compatibility-stage" data-stage-state={stage.state} key={stage.id}>
  <dt>{stageLabels[stage.id]}: {stage.state.replaceAll("_"," ")}</dt><dd>{stage.reason}</dd>
</div>;
const gatewayTarget = (device:Device) => device.category === "generic_network_interface" && device.networks.some(n => n.linkUp && !n.virtualInterface && n.gateways.length > 0);
const deviceName = (device:Device) => gatewayTarget(device) ? `Router via ${device.networks.find(n => n.linkUp && !n.virtualInterface)?.displayName || device.displayName}` : device.displayName || "Unknown device";
export function Devices({ inventory, desktop, error, refreshing, onRefresh, onCancel }: {
  inventory: DiscoverySnapshot | null; desktop: boolean; error: string | null; refreshing: boolean;
  onRefresh: () => Promise<void>; onCancel?: () => Promise<void>;
}) {
  const [advanced, setAdvanced] = useState(false);
  const [query, setQuery] = useState("");
  const [transport, setTransport] = useState("all");
  const [showDisconnected, setShowDisconnected] = useState(false);
  const [infoId, setInfoId] = useState<string | null>(null);
  const devices = inventory?.devices ?? [];
  const revision = inventory?.revision ?? 0;
  const matches = devices.filter(device =>
    (showDisconnected || device.connected) &&
    (transport === "all" || device.transports.includes(transport as "usb" | "serial" | "network")) &&
    [device.displayName, device.vendorId, device.productId, ...device.interfaces.map(item => item.portName ?? "")]
      .join(" ").toLowerCase().includes(query.toLowerCase().trim()));
  const candidates = matches.filter(device => ["router_mifi_candidate", "usb_modem_candidate", "network_gateway_candidate"].includes(device.category ?? "") || gatewayTarget(device));
  const diagnostics = matches.filter(device => !candidates.includes(device));
  const infoDevice = devices.find(device => device.id === infoId);
  const waiting = desktop && !error && (!inventory || inventory.status === "starting");
  const card = (device: Device) => {
    const tone = !gatewayTarget(device) && deviceStatus(device).tone === "error" ? "error" : "warning";
    return <article key={device.id} data-device-id={device.id} className={`panel device-card device-${tone}${device.connected ? "" : " disconnected"}`} aria-label={`${device.displayName}: ${tone === "error" ? "firmware extraction unavailable" : "compatibility not verified"}`}>
      <h3>{deviceName(device)}</h3>
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
      {infoDevice ? <DeviceDetails key={infoDevice.id} device={infoDevice} revision={revision} onClose={() => setInfoId(null)}/> : null}
    </>}
  </>;
}

function DeviceDetails({device, revision, onClose}: {device: Device; revision: number; onClose: () => void}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const generation = useRef(0);
  const [state, setState] = useState<"idle" | "checking" | "done" | "error">("idle");
  const [message, setMessage] = useState("");
  const [report, setReport] = useState<AcquisitionReport|null>(null);
  const networks = device.networks.filter(n => n.linkUp && !n.virtualInterface);
  const [interfaceId, setInterfaceId] = useState(networks.length === 1 ? networks[0].id : "");
  const [endpoint, setEndpoint] = useState(networks.length === 1 && networks[0].gateways.length === 1 ? `http://${networks[0].gateways[0].includes(":") ? `[${networks[0].gateways[0]}]` : networks[0].gateways[0]}/` : "");
  const [authorized, setAuthorized] = useState(false);
  const [advancedEndpoint, setAdvancedEndpoint] = useState(false);
  const [exporting, setExporting] = useState(false);
  useEffect(() => {
    const element = dialog.current;
    const previous = document.activeElement as HTMLElement | null;
    element?.showModal();
    return () => {element?.close(); previous?.focus();};
  }, []);
  useEffect(() => {
    const requests=generation; requests.current++; setState("idle"); setMessage(""); setReport(null);
    return () => {requests.current++; void cancelFirmwareRouteDiscovery(device.id).catch(() => {});};
  }, [revision, device.connected, device.id, endpoint, interfaceId, authorized, advancedEndpoint]);
  useEffect(() => {
    if (!report) return;
    const timer = setTimeout(() => {setReport(null); setMessage("Result expired. Check current compatibility again.");}, Math.max(0,report.expiresAt-Date.now()));
    return () => clearTimeout(timer);
  }, [report]);
  const verify = async () => {
    const run = ++generation.current;
    setState("checking"); setMessage(""); setReport(null);
    try {
      const input = {deviceId:device.id,endpoint:endpoint || null,interfaceId:interfaceId || null,approvalToken:null as string|null,advancedEndpoint};
      if (authorized) {
        input.approvalToken = await approveFirmwareRouteProbe(input);
        if (run !== generation.current) return;
      }
      const result = await discoverFirmwareRoutes(input);
      if (run !== generation.current) return;
      if (result.deviceId !== device.id || result.discoveryRevision !== revision || result.expiresAt <= Date.now()) throw new Error("Device information changed. Refresh devices and try again.");
      setReport(result); setState("done");
    } catch (failure) {
      if (run === generation.current) {setState("error"); setMessage(errorMessage(failure));}
    }
  };
  const readMenu = async () => {
    const run=++generation.current;setState("checking");setMessage("");
    try {
      const input={deviceId:device.id,endpoint:endpoint||null,interfaceId:interfaceId||null,approvalToken:null as string|null,advancedEndpoint};
      input.approvalToken=await approveStaticMenu(input);
      if(run!==generation.current)return;
      const result=await readStaticMenu(input);
      if(run!==generation.current)return;
      if(result.discoveryRevision!==revision || result.expiresAt<=Date.now())throw new Error("Menu result expired. Check compatibility again.");
      setReport(result);setState("done");
    }catch(failure){if(run===generation.current){setState("error");setMessage(errorMessage(failure));}}
  };
  const readChallenge = async () => {
    const run=++generation.current;setState("checking");setMessage("");
    try {
      const input={deviceId:device.id,endpoint:endpoint||null,interfaceId:interfaceId||null,approvalToken:null as string|null,advancedEndpoint};
      input.approvalToken=await approveAuthChallenge(input);
      if(run!==generation.current)return;
      const result=await readAuthChallenge(input);
      if(run!==generation.current)return;
      if(result.discoveryRevision!==revision||result.expiresAt<=Date.now())throw new Error("Challenge result expired. Check compatibility again.");
      setReport(result);setState("done");
    }catch(failure){if(run===generation.current){setState("error");setMessage(errorMessage(failure));}}
  };
  const exportReport = async () => {
    setExporting(true); setMessage("");
    try {if (await exportAcquisitionDiagnostics(device.id)) setMessage("Redacted diagnostics saved.");}
    catch (failure) {setMessage(errorMessage(failure));}
    finally {setExporting(false);}
  };
  const addresses = [...new Set(device.networks.flatMap(network => network.addresses))];
  const dns = [...new Set(device.networks.flatMap(network => network.dnsServers ?? []))];
  const routes = [...new Set(device.networks.flatMap(network => network.defaultRoutes?.map(route => route.gateway) ?? network.gateways))];
  return <dialog ref={dialog} className="device-info-dialog device-summary-dialog" aria-labelledby={titleId} onCancel={event => {event.preventDefault(); onClose();}}>
    <div className="section-heading"><h2 id={titleId}>{deviceName(device)}</h2><button className="icon-button" aria-label="Close device information" onClick={onClose}><X size={18}/></button></div>
    <section className="device-network-summary" aria-label="Network info"><h3>Network info</h3><dl><div><dt>Local addresses</dt><dd>{addresses.join(", ") || "Not reported"}</dd></div><div><dt>DNS</dt><dd>{dns.join(", ") || "Not reported"}</dd></div><div><dt>Default routes</dt><dd>{routes.join(", ") || "Not reported"}</dd></div></dl></section>
    {state === "checking" ? <progress className="compatibility-progress" aria-label="Verifying firmware extraction compatibility"/> : null}
    <details><summary>Device observations and management endpoint</summary>
      <p>Windows name: {device.displayName}. USB: {device.vendorId || "Unknown"} / {device.productId || "Unknown"}.</p>
      <label>Management interface<select value={interfaceId} onChange={e => {setInterfaceId(e.target.value);setAuthorized(false);}}><option value="">Select interface</option>{networks.map(n => <option key={n.id} value={n.id}>{n.displayName}</option>)}</select></label>
      <label>Management address<input value={endpoint} maxLength={2048} onChange={e => {setEndpoint(e.target.value);setAuthorized(false);}} placeholder="http://192.168.100.1/"/></label>
      <label><input type="checkbox" checked={advancedEndpoint} onChange={e => {setAdvancedEndpoint(e.target.checked);setAuthorized(false);}}/>Advanced: allow a different local endpoint</label>
      <label><input type="checkbox" checked={authorized} disabled={!endpoint || !interfaceId} onChange={e => setAuthorized(e.target.checked)}/>I own or administer this endpoint and allow a read-only root check and recognized-family static version check.</label>
      <p className="muted small">Without approval, Check Compatibility evaluates local evidence and registered acquisition routes only.</p>
    </details>
    {report ? <section className="acquisition-results" aria-label="Firmware acquisition routes">
      <h3>No currently qualified acquisition method</h3>
      <p>Identity: {report.identity.confidence}. Verified model: {report.identity.verifiedModel || "Unknown"}. Hardware revision: {report.identity.verifiedHardwareRevision || "Unknown"}. Firmware: {report.identity.verifiedFirmware || "Unknown"}.</p>
      {report.identity.reportedModel ? <p>User-reported model: {report.identity.reportedModel} (unverified).</p> : null}
      <p>Management family: {report.protocols.map(p => p.family.replaceAll("_"," ")).join(", ") || "Not established"}. {report.probe.attempted ? `Read-only endpoint check: ${report.probe.code.replaceAll("_"," ")}${report.probe.httpStatus ? ` (HTTP ${report.probe.httpStatus})` : ""}.` : "No network request made."}</p>
      {report.identity.evidence?.filter(e => e.attribute === "firmware_version" && (e.verification === "observed" || e.verification === "user_reported")).map(e => <p key={`${e.id}-${e.source}`}>{e.verification === "observed" ? "Endpoint-observed version" : "User-reported firmware"}: {e.value} (unverified).</p>)}
      {report.probe.code === "subnet_conflict" ? <p role="status">{report.probe.message}</p> : null}
      {report.probe.metadataTruncated ? <p className="muted small">Metadata limited to first 16 KiB.</p> : null}
      {report.probe.advancedEndpoint ? <p>Advanced local endpoint approved.</p> : null}
      <p className="muted small">Each stage is assessed separately. A verified stage does not enable another operation.</p>
      <dl aria-label="Compatibility summary">{report.stages.slice(0,5).map(stage=><div key={stage.id}>{stageRow(stage)}{stage.id==="management_api" ? <>
        {report.staticMenu ? <p>{report.staticMenu.fileName ? `${report.staticMenu.label}: ${report.staticMenu.fileName}` : report.staticMenu.code==="menu_not_readable_without_login" ? "Menu not readable without login" : report.staticMenu.code==="wrong_content_type" ? `Static menu requires XML; observed media type: ${report.staticMenu.observedMediaType || "unavailable"}.` : report.staticMenu.code.replaceAll("_"," ")}</p> : null}
        {report.protocols.some(p=>p.family==="asr_duster_xml") ? <button className="secondary-button" disabled={state==="checking" || !device.connected} onClick={()=>void readMenu()}>Read static menu</button> : null}
        {report.authChallenge ? <p>{report.authChallenge.code==="observed" ? `${report.authChallenge.label}: realm ${report.authChallenge.realm}; qop ${report.authChallenge.qop}; algorithm ${report.authChallenge.algorithm || "not supplied"}. Cookie header: ${report.authChallenge.setCookiePresent ? "present; authenticated work blocked" : "not present"}.` : `Authentication challenge: ${report.authChallenge.code.replaceAll("_"," ")}.`}</p> : null}
        {report.protocols.some(p=>p.family==="asr_duster_xml") ? <button className="secondary-button" disabled={state==="checking"||!device.connected} onClick={()=>void readChallenge()}>Observe authentication challenge</button> : null}
      </> : null}</div>)}</dl>
      <details><summary>Remaining compatibility stages</summary><dl>{report.stages.slice(5).map(stageRow)}</dl></details>
      {report.candidates.map(route => <details key={route.strategy}><summary>{route.name}: {route.capability.state.replaceAll("_"," ")}</summary><p>{route.capability.reason}</p><p className="muted small">Required: {route.capability.missingRequirements.join("; ")}</p></details>)}
      <button className="secondary-button" disabled={exporting} onClick={() => void exportReport()}>{exporting ? "Exporting…" : "Export Diagnostics"}</button>
    </section> : null}
    {message ? <p role={state === "error" ? "alert" : "status"}>{message}</p> : null}
    {!device.connected ? <p role="status">Device disconnected. Reconnect it to verify compatibility.</p> : null}
    <div className="dialog-actions">{state === "checking" ? <button className="secondary-button" onClick={() => void cancelFirmwareRouteDiscovery(device.id).catch(e => setMessage(errorMessage(e)))}>Cancel check</button> : null}<button className="secondary-button" disabled={!device.connected || state === "checking"} onClick={() => void verify()}>Check Compatibility</button><button className="primary-button" disabled={!report?.canObtain}><LockKeyhole size={15}/>Obtain Firmware</button></div>
  </dialog>;
}
