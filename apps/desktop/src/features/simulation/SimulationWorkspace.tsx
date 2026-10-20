import { useCallback,useEffect,useRef,useState } from "react";
import { simulationControl,simulationRequest,simulationStatus } from "../../lib/simulator";
import { errorMessage } from "../../lib/backend";
import type { Scenario,SimulationAction,SimulationControl,SimulationStatus } from "../../types/simulator";
import { DnsSimulator } from "./DnsSimulator";
import "./simulation.css";

export function SimulationWorkspace(){
 const [status,setStatus]=useState<SimulationStatus|null>(null);
 const [busy,setBusy]=useState(false);const [error,setError]=useState<string|null>(null);
 const [approved,setApproved]=useState(false);const [credential,setCredential]=useState("");
 const [ussdCode,setUssdCode]=useState("");const [reply,setReply]=useState("");
 const [wifiLimits,setWifiLimits]=useState<Record<string,string>>({});
 const [apn,setApn]=useState("");const [apnSource,setApnSource]=useState<"automatic"|"manual">("manual");
 const generation=useRef(0);
 const mounted=useRef(false);
 useEffect(()=>{mounted.current=true;let active=true;const id=++generation.current;void simulationStatus().then(s=>{if(active&&id===generation.current)setStatus(s);}).catch(e=>{if(active&&id===generation.current)setError(errorMessage(e));});return()=>{active=false;mounted.current=false;};},[]);
 const operation=useCallback(async(work:()=>Promise<SimulationStatus>)=>{
  const id=++generation.current;setBusy(true);setError(null);
  try{const next=await work();if(mounted.current&&id===generation.current)setStatus(next);}
  catch(e){if(mounted.current&&id===generation.current){setError(typeof e==="object"&&e&&"code" in e?`${String(e.code)}: ${errorMessage(e)}`:errorMessage(e));try{const next=await simulationStatus();if(mounted.current&&id===generation.current)setStatus(next);}catch{/* Preserve the actual operation error. */}}}
  finally{if(mounted.current&&id===generation.current)setBusy(false);}
 },[]);
 const control=(input:SimulationControl)=>operation(()=>simulationControl(input));
 const perform=async(action:SimulationAction)=>{
  if(!status)return;
  await operation(async()=>{let next=await simulationRequest(status,action);
   if(["apply_dns","restore","set_wifi","set_apn"].includes(action.kind)&&next.readAuthorized)next=await simulationRequest(next,{kind:"read_features"});
   return next;});
 };
 const connected=status?.connection==="connected_read_only";
 const features=status?.features;
 const cancel=()=>{void control({kind:"cancel"});};
 return <section className="simulation-workspace" aria-label="Simulation workspace">
  <div className="simulation-banner"><strong>SIMULATION MODE</strong><p>This is a virtual router. Changes made here do not affect connected physical devices.</p></div>
  {error?<div role="alert" className="inventory-warning">{error}</div>:null}
  {!status?<p role="status">Loading isolated simulator…</p>:<>
   <section className="panel"><div className="section-heading"><h2>Virtual Router Lab</h2><span>{connected?"Simulated Connected":status.active?"Simulation ready":"Simulation stopped"}</span></div>
    <div className="simulation-actions"><button className="primary-button" disabled={busy||status.active} onClick={()=>void control({kind:"start"})}>Start simulation</button><button className="secondary-button" disabled={!status.active} onClick={()=>void control({kind:"stop"})}>Stop simulation</button><button className="secondary-button" disabled={!status.active} onClick={()=>{setApproved(false);setCredential("");void control({kind:"reset"});}}>Reset simulator</button>{busy?<><span role="status">Running virtual operation…</span><button className="secondary-button" onClick={cancel}>Cancel virtual operation</button></>:null}</div>
    {status.device?<><label>Virtual device<select aria-label="Virtual device" defaultValue={status.device.id} disabled={busy}><option value={status.device.id}>{status.device.model} · {status.device.id}</option></select></label>
     <p>Firmware {status.device.firmwareVersion} · hardware {status.device.hardwareRevision} · configuration revision {status.revision} · generation {status.epoch}</p><code className="simulation-endpoint">{status.device.endpoint}</code>
     <details><summary>Virtual endpoint and identity</summary><p>{status.device.manufacturer} · {status.device.firmwareFamily} · {status.device.operatingSystem}</p><p>Interface: {status.device.interfaceId}</p><p className="mono">Peer pin: {status.device.peerPin}</p><p>No Windows interface, physical device identity or carrier service is represented.</p></details>
     <div className="simulation-grid"><label>Scenario<select aria-label="Simulation scenario" disabled={busy} value={status.scenario} onChange={e=>void control({kind:"scenario",scenario:e.target.value as Scenario,repeat:false})}>{status.scenarios.map(s=><option value={s} key={s}>{s.replaceAll("_"," ")}</option>)}</select></label><div><span>Virtual time: {status.virtualTime} ms</span><div className="simulation-actions"><button className="text-button" onClick={()=>void control({kind:"advance",milliseconds:1000})}>Advance 1 second</button><button className="text-button" onClick={()=>void control({kind:"advance",milliseconds:31000})}>Advance 31 seconds</button></div></div></div>
     {!connected?<div className="simulation-auth"><label className="checkbox-label"><input type="checkbox" aria-label="Approve virtual endpoint handshake" checked={approved} onChange={e=>setApproved(e.target.checked)}/>I approve one bounded identity/authentication handshake to this virtual endpoint.</label><div className="simulation-actions"><button className="secondary-button" disabled={busy||!approved} onClick={()=>void perform({kind:"approve_handshake"})}>Approve virtual handshake</button><label>Simulation password<input type="password" autoComplete="off" aria-label="Simulation password" value={credential} onChange={e=>setCredential(e.target.value)}/></label><button className="primary-button" disabled={busy||!status.approvalId||!credential} onClick={()=>{const approvalId=status.approvalId;if(approvalId){const secret=credential;setCredential("");void perform({kind:"connect",approvalId,credential:secret});}}}>Connect virtual router</button></div></div>:<div className="simulation-actions"><span>Authentication: {status.authentication}</span><button className="secondary-button" disabled={busy||status.readAuthorized} onClick={()=>void perform({kind:"approve_reads"})}>Authorize feature reads</button><button className="primary-button" disabled={busy||!status.readAuthorized} onClick={()=>void perform({kind:"read_features"})}>Read all features</button><button className="text-button" disabled={busy} onClick={()=>void perform({kind:"disconnect"})}>Disconnect virtual router</button></div>}
    </>:null}
   </section>
   {status.active?<><DnsSimulator key={status.epoch} status={status} busy={busy} perform={perform}/>
    <div className="simulation-grid"><section className="panel" aria-label="Simulated Wi-Fi"><h3>Wi-Fi · virtual client limits</h3>{features?.wifi.value?<><p>Virtual wireless service: {features.wifiService}</p>{features.wifi.value.map(w=><article key={w.radio}><h4>{w.ssid} · {w.radio}</h4><p>BSS {w.bssId} · {w.connectedClients} associated clients · {features.wifiService==="running"?"Enabled":"Disabled"}</p><p>Limit {w.stationLimit} · virtual range {w.verifiedMinimum}–{w.verifiedMaximum}</p><label>Virtual limit for {w.radio}<input type="number" aria-label={`Virtual limit ${w.radio}`} min={w.verifiedMinimum??1} max={w.verifiedMaximum??32} value={wifiLimits[w.radio]??String(w.stationLimit??"")} onChange={e=>setWifiLimits({...wifiLimits,[w.radio]:e.target.value})}/></label><button className="secondary-button" disabled={busy||!status.readAuthorized} onClick={()=>void perform({kind:"set_wifi",radio:w.radio,limit:Number(wifiLimits[w.radio]??w.stationLimit)})}>Set virtual limit {w.radio}</button></article>)}</>:<p>{features?.wifi.error?.message??"Read all features to inspect virtual radios."}</p>}</section>
     <section className="panel" aria-label="Simulated APN"><h3>APN · virtual cellular modem</h3>{features?<><p>Modem {features.modem.present?"present":"absent"} · SIM {features.modem.simPresent?features.modem.pinRequired?"PIN required":"present":"missing"} · operator {features.modem.operator??"unknown"}</p><p>Registration {features.modem.registered?"registered":"failed"} · data {features.modem.dataConnected?"connected":"disconnected"} · service {features.modem.service}</p>{features.apn.value?<><p>Current APN: {features.apn.value.apn} · {features.apn.value.source} · {features.apn.value.ipFamily} · {features.apn.value.authenticationMethod}</p><p>{features.apn.value.observedBy}</p><label>Virtual APN source<select aria-label="Virtual APN source" value={apnSource} onChange={e=>setApnSource(e.target.value as "manual"|"automatic")}><option value="manual">Manual</option><option value="automatic">Automatic synthetic operator lookup</option></select></label><label>Virtual APN<input aria-label="Virtual APN" value={apn} placeholder={features.apn.value.apn??""} onChange={e=>setApn(e.target.value)}/></label><button className="secondary-button" disabled={busy||!status.readAuthorized} onClick={()=>void perform({kind:"set_apn",source:apnSource,apn,authentication:"none",ipFamily:"dual_stack"})}>Set simulated APN</button></>:<p role="status">{features.apn.error?.code}: {features.apn.error?.message}</p>}</>:<p>Read all features to inspect virtual cellular state.</p>}</section>
    </div>
    <section className="panel" aria-label="Simulated USSD"><h3>USSD · scripted virtual modem</h3><p>{features?.ussd.value?`${features.ussd.value.assessment} · ${features.ussd.value.interactiveSession?"Interactive script available":"Interactive script unavailable"}`:features?.ussd.error?.message??"Read all features to inspect USSD capability."}</p><p>No mobile network, charges or AT commands. Scripted codes: *123# menu, *100# fictional balance.</p><label>Scripted USSD code<input aria-label="Scripted USSD code" value={ussdCode} maxLength={64} onChange={e=>setUssdCode(e.target.value)}/></label><button className="secondary-button" disabled={busy||!status.readAuthorized||!ussdCode||features?.ussd.value?.assessment!=="SUPPORTED"} onClick={()=>void perform({kind:"ussd_start",code:ussdCode})}>Send scripted USSD</button>
     {status.ussd?<div role="status"><p>{status.ussd.state}: {status.ussd.text}</p>{status.ussd.state==="active"?<><label>USSD menu reply<input aria-label="USSD menu reply" value={reply} maxLength={32} onChange={e=>setReply(e.target.value)}/></label><button className="secondary-button" disabled={busy||!reply} onClick={()=>status.ussd&&void perform({kind:"ussd_reply",sessionId:status.ussd.id,text:reply})}>Send menu reply</button><button className="text-button" disabled={busy} onClick={()=>status.ussd&&void perform({kind:"ussd_cancel",sessionId:status.ussd.id})}>Cancel scripted session</button></>:null}</div>:null}
    </section>
    <details className="panel"><summary>Simulation diagnostics and snapshots</summary><p>{status.activeConnections} active virtual connections · {status.snapshots.length} configuration snapshots. All state is ephemeral and resets on app restart.</p><ul>{status.events.map((e,i)=><li key={`${e.time}-${i}`}>{e.time}: {e.message}</li>)}</ul></details>
   </>:null}
  </>}
 </section>;
}
