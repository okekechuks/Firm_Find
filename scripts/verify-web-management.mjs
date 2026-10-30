// Synthetic local captures through the real native picker and React/Rust IPC.
// This neither contacts a router nor represents physical qualification.
import assert from 'node:assert/strict';
import {lstat, readFile, readdir, writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import path from 'node:path';
import {assertOwned} from './native-test-profile.mjs';

const id='1'.repeat(64);
const sentinel='SYNTHETIC-SECRET-DO-NOT-RETAIN';
const until=async(check,message)=>{const deadline=Date.now()+45000;while(Date.now()<deadline){if(await check())return;await new Promise(resolve=>setTimeout(resolve,150));}throw new Error(message);};

export async function verifyWebManagement(client,{root,profile,evidence,pid,record,screenshot,reopened=false}) {
  const evaluate=source=>client.evaluate(source);
  const invoke=(command,args={})=>evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
  const click=label=>evaluate(`Array.from(document.querySelectorAll('dialog button')).find(button=>button.textContent.trim()===${JSON.stringify(label)}).click()`);
  const inventory=await invoke('get_discovery');
  assert.equal(inventory.devices.length,1);assert.equal(inventory.devices[0].id,id);
  assert(inventory.warnings.some(value=>value.includes('Synthetic')));
  await evaluate("window.location.hash='devices'");
  await until(()=>evaluate("document.querySelector('.device-card button')!==null"),'Selected-device card missing');
  await evaluate("document.querySelector('.device-card button').click()");
  await until(()=>evaluate("Array.from(document.querySelectorAll('dialog summary')).some(node=>node.textContent.trim()==='Web UI Evidence')"),'Web UI Evidence workflow missing');
  await evaluate("Array.from(document.querySelectorAll('dialog summary')).find(node=>node.textContent.trim()==='Web UI Evidence').click()");
  await until(()=>evaluate("Array.from(document.querySelectorAll('dialog button')).some(node=>node.textContent.trim()==='Import Web UI Capture (HAR)'&&!node.disabled)"),'Backend evidence permission did not render');
  const empty=await invoke('get_web_ui_evidence',{deviceId:id});
  assert.equal(empty.entries.length,0,'Session-only evidence unexpectedly survived restart');
  assert.equal(empty.actions.authenticate,false);assert.equal(empty.actions.collectResources,false);assert.equal(empty.canAcquire,false);
  if(reopened) {
    await screenshot('web-evidence-reopened-empty');
    record('native restart: management evidence remains session-only; no credentials or endpoint authority retained',{entries:0,physicalDeviceSuccess:false});
    return;
  }
  await click('Check selected route');
  const route=await invoke('assess_web_management_binding',{deviceId:id,endpoint:'http://192.0.2.1/',interfaceId:'synthetic-workflow-interface'});
  assert.equal(route.state,'blocked');assert.equal(route.sourceIp,null);assert.equal(route.canAuthenticate,false);assert.equal(route.cryptographicallyVerified,false);
  await until(()=>evaluate("document.querySelector('[aria-label=\"Selected management route\"]')?.innerText.includes('Select one connected physical interface')"),'Synthetic interface incorrectly received a real Windows route proof');
  const capture=path.join(profile.root,'fixtures','synthetic-management.har');
  const bundle=path.join(profile.root,'fixtures','synthetic-bundle.js');
  const malformed=path.join(profile.root,'fixtures','synthetic-malformed.har');
  for(const item of [capture,bundle,malformed])await assertOwned(profile,item);
  const entry=(uri,method='GET')=>({request:{method,url:`http://192.0.2.1${uri}`,headers:[{name:'Authorization',value:`Basic ${sentinel}`},{name:'Cookie',value:sentinel}],postData:{mimeType:'application/json',text:JSON.stringify({password:sentinel,imei:sentinel})}},response:{status:200,headers:[{name:'Set-Cookie',value:sentinel}],content:{mimeType:'application/json',text:JSON.stringify({token:sentinel})}}});
  await writeFile(capture,JSON.stringify({log:{version:'1.2',creator:{name:'Synthetic test',version:'1'},entries:[entry(`/status?token=${sentinel}`),entry('/login.cgi','POST'),entry('/reboot'),entry('/firmware/download'),entry('/config/export')]}}),{flag:'wx'});
  await writeFile(bundle,"// Synthetic fixture; analyzed as data, never executed.\nfetch('/status'); fetch('/firmware/download'); fetch('/reboot');\n",{flag:'wx'});
  await writeFile(malformed,'{"log":',{flag:'wx'});
  const pick=async(label,title,file)=>{
    await click(label);
    execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(root,'scripts/drive-file-dialog.ps1'),'-DialogTitle',title,'-SelectedPath',file,'-OwnerPid',String(pid)],{windowsHide:true,timeout:45000});
  };
  await pick('Import Web UI Capture (HAR)','Import Web UI Capture (local evidence only)',capture);
  let report;
  await until(async()=>{report=await invoke('get_web_ui_evidence',{deviceId:id});return report.entries.length===5;},'Native HAR import did not publish sanitized evidence');
  assert(report.entries.some(item=>item.method==='GET'&&item.classification==='reboot'&&!item.readOnlyQualified));
  assert(report.entries.some(item=>item.classification==='config_backup'));
  assert(report.entries.some(item=>item.classification==='firmware_download'));
  assert(!JSON.stringify(report).includes(sentinel));
  await pick('Analyze Management UI','Analyze local management UI resource (no requests)',bundle);
  await until(async()=>{report=await invoke('get_web_ui_evidence',{deviceId:id});return report.entries.some(item=>item.evidence==='both');},'Static/HAR cross-reference did not publish');
  assert.equal(report.canAcquire,false);assert.equal(report.actions.authenticate,false);assert.equal(report.actions.collectResources,false);
  assert.equal(report.strategy.canAcquire,false);
  assert(report.strategy.sources.some(source=>source.sourceType==='config_export'&&source.state==='not_firmware'));
  assert(report.strategy.sources.some(source=>source.sourceType==='observed_firmware_download'&&source.state==='requires_qualification'));
  assert(report.entries.every(item=>!item.readOnlyQualified));
  await assert.rejects(()=>invoke('collect_management_ui_resources',{deviceId:id}),/IDENTITY_UNQUALIFIED|independently qualified/);
  const before=JSON.stringify(report);
  await pick('Import Web UI Capture (HAR)','Import Web UI Capture (local evidence only)',malformed);
  await until(()=>evaluate("!!document.querySelector('[aria-label=\"Web management evidence\"] [role=\"alert\"]')"),'Malformed HAR error missing');
  assert.equal(JSON.stringify(await invoke('get_web_ui_evidence',{deviceId:id})),before,'Rejected import destroyed earlier evidence');
  assert(await evaluate("document.querySelector('dialog').innerText.includes('Web-management acquisition: BLOCKED')"));
  assert(!await evaluate(`document.querySelector('dialog').innerText.includes(${JSON.stringify(sentinel)})`));
  await writeFile(path.join(evidence,'web-evidence-result.json'),JSON.stringify({synthetic:true,routerRequests:0,physicalDeviceSuccess:false,report},null,2));
  const viewport=await evaluate("new Promise(resolve=>{const node=document.querySelector('[aria-label=\"Web management evidence\"]');node.scrollIntoView({block:'start',behavior:'instant'});requestAnimationFrame(()=>requestAnimationFrame(()=>{const rect=node.getBoundingClientRect();resolve({top:rect.top,width:rect.width,height:rect.height,viewport:innerHeight});}));})");
  assert(viewport.width>0&&viewport.height>0&&viewport.top>=0&&viewport.top<viewport.viewport,JSON.stringify(viewport));
  await screenshot('web-evidence-sanitized');
  // Native input fixtures contain synthetic sentinel values intentionally; they
  // are separate from application-owned data. The app must not copy them there.
  const state=await invoke('get_snapshot');
  assert(state.testProfile);
  assert((await readFile(capture,'utf8')).includes(sentinel));
  assert.equal((await invoke('list_library_jobs')).length,0,'Evidence import created a firmware operation');
  record('native selected-device workflow: local HAR picker, dangerous GET, static correlation, malformed rejection and fail-closed collection',{synthetic:true,endpoints:report.entries.length,routerRequests:0,physicalDeviceSuccess:false});
}

// Audit only after clean app shutdown: the live vault deliberately denies reads
// of library.lock. Reading closed data also covers the checkpointed SQLite bytes.
export async function verifyWebManagementStorage({profile,record}) {
  const pending=[path.join(profile.root,'data')];let inspectedFiles=0;
  while(pending.length) {
    const directory=pending.pop();await assertOwned(profile,directory);
    for(const name of await readdir(directory)) {
      const file=path.join(directory,name);await assertOwned(profile,file);
      const info=await lstat(file);assert(!info.isSymbolicLink());
      if(info.isDirectory())pending.push(file);
      else {
        assert(info.isFile());const bytes=await readFile(file);inspectedFiles++;
        assert(!bytes.includes(Buffer.from(sentinel,'utf8'))&&!bytes.includes(Buffer.from(sentinel,'utf16le')),'Capture secret leaked into application-owned storage');
      }
    }
  }
  assert(inspectedFiles>0);
  record('native synthetic secret absent from closed SQLite/WAL/log/vault file bytes',{inspectedFiles});
}
