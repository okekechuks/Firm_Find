// Real Windows WebView -> Rust -> pinned SSH -> genuine disposable OpenWrt OS.
// The lab endpoint is sealed in the native-test binary; no physical router is contacted.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve(import.meta.dirname,'..');
const lab='8888888888888888888888888888888888888888888888888888888888888888';
const api=client=>(command,args={})=>client.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
async function until(check,message,timeout=30000){const end=Date.now()+timeout;while(Date.now()<end){if(await check())return;await new Promise(r=>setTimeout(r,50));}throw new Error(message);}
async function button(client,text){
  const scope=text==='Inspect OpenWrt router'?'button':'.openwrt-dialog button';
  await until(()=>client.evaluate(`Boolean([...document.querySelectorAll(${JSON.stringify(scope)})].find(b=>b.textContent.trim()===${JSON.stringify(text)}&&!b.disabled))`),'Enabled OpenWrt button did not render: '+text);
  await client.evaluate(`(()=>{const b=[...document.querySelectorAll(${JSON.stringify(scope)})].find(b=>b.textContent.trim()===${JSON.stringify(text)}&&!b.disabled);if(!b)throw Error('Enabled OpenWrt button disappeared: '+${JSON.stringify(text)});b.click();})()`);
}
async function checkbox(client,prefix){
  await until(()=>client.evaluate(`Boolean([...document.querySelectorAll('.openwrt-dialog label')].find(l=>l.textContent.startsWith(${JSON.stringify(prefix)}))?.querySelector('input[type=checkbox]:not(:disabled)'))`),'Enabled OpenWrt approval did not render: '+prefix);
  await client.evaluate(`(()=>{const l=[...document.querySelectorAll('.openwrt-dialog label')].find(l=>l.textContent.startsWith(${JSON.stringify(prefix)}));const c=l?.querySelector('input[type=checkbox]');if(!c||c.disabled)throw Error('Enabled OpenWrt approval disappeared');if(!c.checked)c.click();})()`);
}
async function input(client,label,value){
  await until(()=>client.evaluate(`Boolean([...document.querySelectorAll('.openwrt-dialog input')].find(e=>e.getAttribute('aria-label')===${JSON.stringify(label)}&&!e.disabled))`),'Enabled OpenWrt input did not render: '+label);
  await client.evaluate(`(()=>{const e=[...document.querySelectorAll('.openwrt-dialog input')].find(e=>e.getAttribute('aria-label')===${JSON.stringify(label)}&&!e.disabled);if(!e)throw Error('Enabled OpenWrt input disappeared');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(e,${JSON.stringify(value)});e.dispatchEvent(new Event('input',{bubbles:true}));})()`);
}
export async function verifyOpenWrt(client,{label,evidence,record,pid}){
  const invoke=api(client);
  // Independent serial-console evidence; never copy the fingerprint observed by network IPC.
  const consoleLog=await readFile(path.join(root,'.tools/phase7-vm/console-identity.log'),'utf8');
  const pin=consoleLog.match(/Fingerprint: (SHA256:[A-Za-z0-9+/]{43})/)?.[1];
  assert(pin,'Owned VM console fingerprint absent');
  const vm=JSON.parse(await readFile(path.join(root,'.tools/phase7-vm/state.json'),'utf8'));
  assert.equal(vm.hardware,false);assert.equal(vm.port,22270);
  await invoke('openwrt_disconnect');
  const denied=await client.evaluate("window.__TAURI_INTERNALS__.invoke('openwrt_inspect').then(()=>({unexpected:true}),e=>({code:e.code}))");
  assert.equal(denied.code,'OPERATION_NOT_AUTHORIZED');
  await client.evaluate("document.querySelector('a[href=\"#devices\"]').click()");
  await until(()=>client.evaluate("Boolean([...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Inspect OpenWrt router'))"),'OpenWrt entry missing');
  await button(client,'Inspect OpenWrt router');
  await until(()=>client.evaluate("Boolean(document.querySelector('.openwrt-dialog [aria-label=\"OpenWrt target\"]'))"),'Inspection dialog did not load');
  await client.evaluate(`(()=>{const e=document.querySelector('[aria-label="OpenWrt target"]');Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value').set.call(e,${JSON.stringify(lab)});e.dispatchEvent(new Event('change',{bubbles:true}));})()`);
  await input(client,'OpenWrt SSH port','22270');
  await checkbox(client,'I own or administer');
  // Cancel an actual GUI-owned operation, then re-use the same service successfully.
  await button(client,'Observe SSH host key');
  await until(()=>client.evaluate("Boolean([...document.querySelectorAll('.openwrt-dialog button')].find(b=>b.textContent==='Cancel OpenWrt operation'))"),'GUI Cancel never became available');
  const cancelPoint=await client.evaluate("(()=>{const b=[...document.querySelectorAll('.openwrt-dialog button')].find(b=>b.textContent==='Cancel OpenWrt operation'&&!b.disabled);if(!b)throw Error('GUI operation completed before the Cancel click; cancellation is not verified');b.scrollIntoView({block:'center'});const r=b.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()");
  await client.send('Input.dispatchMouseEvent',{type:'mousePressed',button:'left',clickCount:1,x:cancelPoint.x,y:cancelPoint.y});
  await client.send('Input.dispatchMouseEvent',{type:'mouseReleased',button:'left',clickCount:1,x:cancelPoint.x,y:cancelPoint.y});
  await until(async()=>{const s=await invoke('openwrt_status');return !s.busy&&s.state==='disconnected';},'GUI cancellation did not release OpenWrt ownership');
  await until(()=>client.evaluate("Boolean([...document.querySelectorAll('.openwrt-dialog button')].find(b=>b.textContent==='Observe SSH host key'&&!b.disabled))"),'GUI did not recover after cancellation');
  await button(client,'Observe SSH host key');
  await until(async()=>{const s=await invoke('openwrt_status');return !s.busy&&s.state==='host_observed';},'Host observation failed');
  const observed=await invoke('openwrt_status');assert.equal(observed.hostFingerprint,pin);assert.equal(observed.targetId,lab);
  assert(observed.targets.find(t=>t.id===lab)?.lab);assert.equal(observed.identity,null);assert.equal(observed.readApproved,false);
  await input(client,'Independently verified SSH fingerprint',pin);
  await checkbox(client,'I independently checked');
  await button(client,'Select SSH key and authenticate');
  execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(root,'scripts/drive-file-dialog.ps1'),'-DialogTitle','Select existing SSH private key — contents stay in backend','-SelectedPath',path.join(root,'.tools/phase7-vm/test-key'),'-OwnerPid',String(pid)],{windowsHide:true,encoding:'utf8',timeout:30000});
  await until(async()=>{const s=await invoke('openwrt_status');if(s.state==='failed')throw Error(JSON.stringify(s.error));return !s.busy&&s.state==='identity_only';},'Authenticated identity did not reach native UI');
  const identity=await invoke('openwrt_status');assert.equal(identity.profileQualified,true);assert.equal(identity.readApproved,false);assert.equal(identity.identity.firmwareVersion,'25.12.6');assert.equal(identity.identity.target,'x86/64');assert.equal(identity.identity.hardwareRevision,null);assert.equal(identity.inspection,null);
  await until(()=>client.evaluate("Boolean(document.querySelector('[aria-label=\"Authenticated OpenWrt identity\"]'))"),'Authenticated identity did not render');
  await checkbox(client,'I separately approve');await button(client,'Authorize read-only inspection');
  await until(()=>client.evaluate("Boolean([...document.querySelectorAll('.openwrt-dialog button')].find(b=>b.textContent==='Read configuration and firmware feasibility'&&!b.disabled))"),'Separate read approval did not enable inspection');
  await button(client,'Read configuration and firmware feasibility');
  await until(async()=>{const s=await invoke('openwrt_status');if(s.state==='failed')throw Error(JSON.stringify(s.error));return !s.busy&&s.state==='inspected';},'Real OpenWrt inspection did not finish');
  const inspected=await invoke('openwrt_status');assert(inspected.inspection);assert(inspected.inspection.networks.length>0);assert.deepEqual(inspected.inspection.wireless,[]);assert.equal(inspected.inspection.ussd.assessment,'UNKNOWN');
  assert(inspected.inspection.runtimeDns!==null,'Genuine ubus runtime DNS query unavailable');assert(inspected.inspection.dhcpDns!==null,'Genuine DHCP projection unavailable');assert.deepEqual(inspected.inspection.radios,[]);assert.deepEqual(inspected.inspection.wirelessRuntime,[]);
  for(const operation of ['acquisition','extraction','build','installation','recovery'])assert.equal(inspected.inspection.firmware[operation].executable,false);
  await until(()=>client.evaluate("Boolean(document.querySelector('[aria-label=\"Firmware Operations\"]'))"),'Firmware feasibility panel did not render');
  assert.equal(await client.evaluate("[...document.querySelectorAll('.openwrt-dialog button')].some(b=>/^(Apply|Flash|Extract Firmware|Send USSD)$/.test(b.textContent)&&!b.disabled)"),false);
  const serialized=JSON.stringify(inspected);assert(!serialized.includes('test-key'));assert(!serialized.includes('PRIVATE KEY'));assert(!serialized.includes(root));
  await writeFile(path.join(evidence,label+'-openwrt.json'),JSON.stringify(inspected,null,2));
  const image=await client.send('Page.captureScreenshot',{format:'png'});await writeFile(path.join(evidence,label+'-openwrt.png'),Buffer.from(image.data,'base64'));
  for(const [name,region] of [['configuration','OpenWrt configuration inspection'],['firmware','Firmware Operations']]){
    await client.evaluate(`document.querySelector('[aria-label='+${JSON.stringify(JSON.stringify(region))}+']').scrollIntoView({block:'start'})`);
    await client.evaluate('new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
    const capture=await client.send('Page.captureScreenshot',{format:'png'});await writeFile(path.join(evidence,label+'-openwrt-'+name+'.png'),Buffer.from(capture.data,'base64'));
  }
  record(label+': genuine OpenWrt OS, independent console pin, native key dialog, identity-only bootstrap, exact OS profile, separate read approval, UCI/ubus projections and feasibility GUI',{version:identity.identity.firmwareVersion,target:identity.identity.target,hardwareQualified:false,limitations:inspected.inspection.limitations});
  record(label+': real GUI Cancel released resources and subsequent SSH authentication/read succeeded');
  await button(client,'Disconnect OpenWrt');
  await until(async()=>{const s=await invoke('openwrt_status');return !s.busy&&s.state==='disconnected'&&!s.readApproved&&!s.inspection;},'Disconnect retained read authority');
  await client.evaluate("document.querySelector('[aria-label=\"Close OpenWrt inspection\"]').click()");
  await until(()=>client.evaluate("!document.querySelector('.openwrt-dialog')"),'OpenWrt dialog did not close');
}
