// Native Phase 6 vertical slice, invoked only by the owned isolated foundation runner.
import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve(import.meta.dirname,'..');
async function until(check,message,timeout=15000){const end=Date.now()+timeout;while(Date.now()<end){if(await check())return;await new Promise(r=>setTimeout(r,80));}throw new Error(message);}
export async function verifySimulator(client,{label,evidence,record}){
 const invoke=(command,args={})=>client.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
 const status=()=>invoke('simulation_status');
 const idle=()=>until(()=>client.evaluate("!document.querySelector('.simulation-workspace')?.textContent.includes('Running virtual operation')"),'Simulator UI did not settle');
 const click=async(text)=>{await until(()=>client.evaluate(`Boolean([...document.querySelectorAll('.simulation-workspace button')].find(b=>b.textContent.trim()===${JSON.stringify(text)}&&!b.disabled))`),'Enabled simulation button missing: '+text);await client.evaluate(`[...document.querySelectorAll('.simulation-workspace button')].find(b=>b.textContent.trim()===${JSON.stringify(text)}&&!b.disabled).click()`);};
 const text=async(label,value)=>client.evaluate(`(()=>{const el=document.querySelector('.simulation-workspace input[aria-label='+${JSON.stringify(JSON.stringify(label))}+']');if(!el)throw new Error('Simulation input missing');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(el,${JSON.stringify(value)});el.dispatchEvent(new Event('input',{bubbles:true}));})()`);
 const select=async(label,value)=>client.evaluate(`(()=>{const el=[...document.querySelectorAll('.simulation-workspace select')].find(e=>e.getAttribute('aria-label')===${JSON.stringify(label)});if(!el)throw new Error('Simulation select missing');el.value=${JSON.stringify(value)};el.dispatchEvent(new Event('change',{bubbles:true}));})()`);
 const expectReject=async(command,args)=>{let rejected=false;try{await invoke(command,args);}catch{rejected=true;}assert(rejected,'Unexpected direct IPC acceptance: '+command);};
 const before=await invoke('get_snapshot');assert.equal(before.testProfile.simulationAvailable,true);assert.equal(before.testProfile.simulatedAdapters,false);
 const inventory=(await invoke('get_discovery')).devices.map(d=>d.id).sort();
 const artifacts=(await invoke('list_firmware_artifacts',{query:{}})).total;
 const imageDir=path.join(root,'docs/images',`phase6-${before.testProfile.runId}`);await mkdir(imageDir,{recursive:true});
 async function shot(name,selector='.simulation-workspace'){
  await client.evaluate(`document.querySelector(${JSON.stringify(selector)})?.scrollIntoView({block:'start'})`);
  if(selector!=='.simulation-workspace')await client.evaluate("window.scrollBy(0,-((document.querySelector('.simulation-banner')?.getBoundingClientRect().height??0)+32))");
  await client.evaluate('new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
  const capture=await client.send('Page.captureScreenshot',{format:'png'});await writeFile(path.join(imageDir,`${label}-${name}.png`),Buffer.from(capture.data,'base64'));
 }
 await client.evaluate("document.querySelector('nav a[href=\"#devices\"]').click()");
 await until(()=>client.evaluate("Boolean(document.querySelector('.simulation-workspace .simulation-banner'))"),'Simulation workspace absent');
 assert(await client.evaluate("document.querySelector('.simulation-banner').innerText.includes('SIMULATION MODE')"));
 assert.equal((await status()).active,false,'Application restart must not silently reactivate simulation');
 await click('Start simulation');await until(async()=>(await status()).active,'Simulation did not start');await idle();
 const initial=await status();assert.equal(initial.device.id,'sim-router-001');assert(initial.device.simulated);assert.equal(initial.readAuthorized,false);assert.equal(initial.features,null);
 await shot('device-selection');
 await expectReject('simulation_request',{input:{target:'a'.repeat(64),endpoint:initial.device.endpoint,epoch:initial.epoch,revision:initial.revision,action:{kind:'approve_handshake'}}});
 await expectReject('simulation_request',{input:{target:initial.device.id,endpoint:'http://192.168.100.1/',epoch:initial.epoch,revision:initial.revision,action:{kind:'approve_handshake'},simulated:true}});
 await expectReject('simulation_control',{input:{kind:'execute',command:'AT+CUSD'}});
 await expectReject('get_device_adapter_matches',{input:{deviceId:initial.device.id,targetKind:'discovered'}});
 assert((await invoke('list_available_adapters')).every(a=>!['test_only','simulator'].includes(a.implementationStatus)));
 record(label+': explicit simulator activation, namespace/provenance and direct IPC isolation; physical registry unchanged');
 async function connect(){
  await client.evaluate("(()=>{const e=document.querySelector('input[aria-label=\"Approve virtual endpoint handshake\"]');if(e&&!e.checked)e.click();})()");
  await click('Approve virtual handshake');await until(async()=>!!(await status()).approvalId,'Virtual approval missing');
  await text('Simulation password','lab-only');await click('Connect virtual router');
  await until(async()=>(await status()).connection==='connected_read_only','Virtual authentication failed');await idle();
  assert(await client.evaluate("document.querySelector('.simulation-workspace').textContent.includes('Simulated Connected')"));
  assert.equal((await status()).readAuthorized,false);await shot('connected-device');
  await click('Authorize feature reads');await until(async()=>(await status()).readAuthorized,'Separate read grant missing');
  await click('Read all features');await until(async()=>!!(await status()).features,'Adapter feature reads absent');await idle();
 }
 await connect();let read=await status();assert.equal(read.authentication,'authenticated');assert.equal(read.activeConnections,1);
 assert.equal(read.features.dns.value.upstreamIpv4[0],'1.1.1.1');assert.equal(read.features.wifi.value.length,2);assert.equal(read.features.apn.value.apn,'lab.example');assert.equal(read.features.ussd.value.assessment,'SUPPORTED');
 assert(await client.evaluate("document.querySelector('[aria-label=\"Simulated APN\"]').innerText.includes('lab.example')"));
 await shot('feature-status');await shot('dns-before','[aria-label="Simulated DNS"]');
 record(label+': real React/Rust bootstrap, separate read approval and all four state-backed adapter reads');
 async function preview(ip){await select('Upstream IPv4 mode','manual');await text('Upstream IPv4 resolvers',ip);await click('Preview DNS changes');await until(async()=>{const p=(await status()).preview;return p&&!p.id.startsWith('consumed-')&&p.proposed.upstreamIpv4[0]===ip;},'DNS preview did not match draft');await idle();}
 await preview('9.9.9.9');let p=(await status()).preview;assert.equal(p.previous.upstreamIpv4[0],'1.1.1.1');assert(p.changes.length>=2);await shot('dns-preview','[aria-label="DNS change preview"]');
 await click('Apply DNS to Simulator');await until(async()=>{const s=await status();return s.outcome?.verified&&s.features?.dns.value?.upstreamIpv4[0]==='9.9.9.9';},'Applied DNS did not read back through adapter');await idle();
 read=await status();assert.equal(read.revision,2);assert.equal(read.snapshots.length,1);assert(read.outcome.configurationMatches&&read.outcome.serviceHealthy&&read.outcome.resolutionSuccessful);
 assert(await client.evaluate("document.querySelector('[aria-label=\"Current simulated DNS\"]').innerText.includes('9.9.9.9')"));await shot('dns-apply-result','[aria-label="Simulated DNS result"]');
 await click('Verify simulated DNS');await until(async()=>(await status()).outcome.operation==='verify','Independent verify did not run');await idle();assert((await status()).outcome.verified);
 await click('Restore previous virtual state');await until(async()=>(await status()).outcome.rollback==='succeeded','Virtual restoration failed');await idle();assert.equal((await status()).features.dns.value.upstreamIpv4[0],'1.1.1.1');await shot('dns-restored','[aria-label="Simulated DNS result"]');
 record(label+': native DNS read/edit/validate/preview/snapshot/apply/readback/independent verify/restore passes');
 async function scenario(value){await select('Simulation scenario',value);await until(async()=>(await status()).scenario===value,'Scenario selection did not reach Rust');await idle();}
 await scenario('dns_verification_mismatch');await preview('1.0.0.1');await click('Apply DNS to Simulator');await until(async()=>!!(await status()).outcome?.applied,'Faulted DNS apply missing');await idle();
 read=await status();assert.equal(read.outcome.verified,false);assert.equal(read.outcome.configurationMatches,false);assert.equal(read.features.dns.value.upstreamIpv4[0],'203.0.113.53');await shot('dns-verification-failure','[aria-label="Simulated DNS result"]');
 await click('Restore previous virtual state');await until(async()=>(await status()).outcome.rollback==='succeeded','Fault restoration missing');await idle();assert.equal((await status()).features.dns.value.upstreamIpv4[0],'1.1.1.1');
 await scenario('rollback_failure');await preview('9.9.9.9');await click('Apply DNS to Simulator');await until(async()=>(await status()).outcome?.configurationMatches===false,'Rollback scenario did not fail verification');await idle();await click('Restore previous virtual state');await until(async()=>(await status()).outcome.rollback==='failed','Restore failure falsely succeeded');await idle();assert((await status()).outcome.requiresReset);assert(await client.evaluate("document.querySelector('[aria-label=\"Simulated DNS result\"]').textContent.includes('Manual simulator reset required')"));await shot('dns-restore-failure','[aria-label="Simulated DNS result"]');
 record(label+': deterministic DNS mismatch, successful restoration and failed rollback with reset-required result');
 await click('Reset simulator');await until(async()=>(await status()).scenario==='healthy'&&!(await status()).readAuthorized,'Reset did not expire authority');await idle();await connect();
 await text('Virtual limit sim-radio-2g','12');await click('Set virtual limit sim-radio-2g');await until(async()=>(await status()).features?.wifi.value?.[0].stationLimit===12,'Virtual Wi-Fi mutation/readback missing');await idle();await shot('wifi-configuration','[aria-label="Simulated Wi-Fi"]');
 await text('Virtual APN','native.lab');await click('Set simulated APN');await until(async()=>(await status()).features?.apn.value?.apn==='native.lab','Virtual APN state/readback missing');await idle();await shot('apn-modem','[aria-label="Simulated APN"]');
 await text('Scripted USSD code','*123#');await click('Send scripted USSD');await until(async()=>(await status()).ussd?.state==='active','Scripted USSD menu missing');await idle();await text('USSD menu reply','1');await click('Send menu reply');await until(async()=>(await status()).ussd?.state==='completed','Scripted USSD response missing');await idle();await shot('ussd-session','[aria-label="Simulated USSD"]');assert((await status()).ussd.text.includes('fictional balance'));
 record(label+': real GUI Wi-Fi/APN state mutation and readback, fictional USSD interactive completion; no physical dispatch');
 await scenario('cellular_restart');await click('Read all features');await until(async()=>(await status()).features?.modem.service==='restarting','Virtual cellular restart not observed');await idle();
 assert.equal((await status()).features.apn.error.code,'DATA_CONNECTION_FAILED');assert.equal((await status()).readAuthorized,false);
 for(let tick=0;tick<3;tick++){await click('Advance 1 second');await idle();}
 await connect();assert.equal((await status()).features.modem.service,'running');assert.equal((await status()).features.apn.value.apn,'native.lab');await shot('cellular-restart-recovered','[aria-label="Simulated APN"]');
 record(label+': virtual cellular restart reports unavailable data, scheduled recovery and fresh adapter readback');
 const live=await status();await scenario('wrong_identity');const invalidated=await status();assert(invalidated.epoch>live.epoch);assert.equal(invalidated.activeConnections,0);assert.equal(invalidated.readAuthorized,false);assert.equal(invalidated.outcome,null);assert.equal(invalidated.device.model,'Wrong Virtual Device');
 await expectReject('simulation_request',{input:{target:live.device.id,endpoint:live.device.endpoint,epoch:live.epoch,revision:live.revision,action:{kind:'read_features'}}});
 await click('Approve virtual handshake');await until(async()=>!!(await status()).approvalId,'Faulted approval missing');await text('Simulation password','lab-only');await click('Connect virtual router');await until(()=>client.evaluate("document.querySelector('.simulation-workspace').textContent.includes('IDENTITY_CONFLICT')"),'Changed identity was not rejected');await idle();assert.equal((await status()).activeConnections,0);
 await scenario('healthy');await connect();
 record(label+': live identity change immediately invalidates grants, cached results and stale requests; fresh wrong-identity authentication rejects');
 await scenario('slow_read');await click('Read all features');
 await until(()=>client.evaluate("Boolean([...document.querySelectorAll('.simulation-workspace button')].find(b=>b.textContent.trim()==='Cancel virtual operation'))"),'GUI Cancel control did not appear');
 const cancelPoint=await client.evaluate("(()=>{const b=[...document.querySelectorAll('.simulation-workspace button')].find(b=>b.textContent.trim()==='Cancel virtual operation');b.scrollIntoView({block:'center'});const r=b.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()");
 await client.send('Input.dispatchMouseEvent',{type:'mousePressed',...cancelPoint,button:'left',clickCount:1});
 await client.send('Input.dispatchMouseEvent',{type:'mouseReleased',...cancelPoint,button:'left',clickCount:1});
 await idle();let cancelled=await status();assert.equal(cancelled.busy,false);assert.equal(cancelled.activeConnections,0);assert.equal(cancelled.readAuthorized,false);
 assert(cancelled.events.some(e=>e.message.includes('Cancelled')),'GUI Cancel must cancel the native read, not let its timeout finish');
 await shot('cancel-recovered');await scenario('healthy');await connect();const recoveredAfterCancel=await status();assert(recoveredAfterCancel.features.dns.value);
 record(label+': real native GUI Cancel input interrupts slow read, releases busy/resources and permits scenario selection, fresh authentication and successful feature reads');
 await scenario('slow_read');await click('Read all features');await until(()=>client.evaluate("document.querySelector('.simulation-workspace').textContent.includes('TIMEOUT')"),'Bounded read timeout not shown');await idle();assert.equal((await status()).activeConnections,0);
 await scenario('healthy');await connect();await scenario('router_disconnected');assert.equal((await status()).activeConnections,0);await shot('disconnected');
 await click('Reset simulator');await idle();assert.equal((await status()).features,null);assert.equal((await status()).activeConnections,0);
 await click('Stop simulation');await until(async()=>!(await status()).active,'Simulation did not stop');await idle();
 assert.deepEqual((await invoke('get_discovery')).devices.map(d=>d.id).sort(),inventory);assert.equal((await invoke('list_firmware_artifacts',{query:{}})).total,artifacts);
 await writeFile(path.join(evidence,`${label}-simulation.json`),JSON.stringify({initial,afterGuiCancel:cancelled,recoveredAfterCancel,final:await status(),images:path.relative(root,imageDir),physicalInventoryIds:inventory,artifactCount:artifacts},null,2));
 record(label+': timeout/disconnect/reset/stop release all virtual connections; Windows inventory and firmware Library unchanged');
}
