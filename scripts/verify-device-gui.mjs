import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
export async function verifyDeviceGUI(client, {label, record, screenshot,pid,profile,evidence}) {
  const evaluate = source => client.evaluate(source);
  async function until(check, message) {
    const end=Date.now()+20000;
    while(Date.now()<end){if(await check())return;await new Promise(resolve=>setTimeout(resolve,150));}
    throw new Error(message);
  }
  assert.equal(await evaluate('Array.from(document.querySelectorAll(".device-toolbar button")).some(b=>/Add Router|Manage saved connections|Inspect OpenWrt/.test(b.textContent))'),false);
  assert.equal(await evaluate('document.querySelector(".disconnected-toggle input").checked'),false);
  if(!await evaluate('!!document.querySelector(".device-card")')) {
    await evaluate('document.querySelector(".other-interfaces").open=true; document.querySelector(".other-interfaces input").click()');
    await until(()=>evaluate('!!document.querySelector(".device-card")'),'No Windows device available for native popup check');
  }
  const size=await evaluate('({width:document.querySelector(".device-card").getBoundingClientRect().width,text:document.querySelector(".device-card").innerText})');
  assert(size.width<=341,JSON.stringify(size));
  assert(!/Check compatibility|Possible candidate|Currently present/.test(size.text));
  await screenshot(label+'-compact-devices');
  await evaluate('document.querySelector(".device-card button").click()');
  await until(()=>evaluate('!!document.querySelector(".device-summary-dialog[open]")'),'Device summary did not open');
  assert.equal(await evaluate('!!document.querySelector(".device-summary-dialog .device-network-summary")'),true);
  assert.equal(await evaluate('/Identification|Windows indexes|Setup class|Hardware IDs|Container:/.test(document.querySelector(".device-summary-dialog").innerText)'),false);
  await evaluate('Array.from(document.querySelectorAll(".device-summary-dialog button")).find(b=>b.textContent==="Check Compatibility").click()');
  await until(()=>evaluate('!!document.querySelector(".acquisition-results") || !!document.querySelector(".device-summary-dialog [role=alert]")'),'Backend route discovery did not return');
  assert.equal(await evaluate('!!document.querySelector(".acquisition-results")'),true,'Native route discovery must succeed, not merely display an error');
  assert.equal(await evaluate('document.querySelector(".device-summary-dialog").textContent.includes("Cannot locate firmware")'),false);
  assert.equal(await evaluate('Array.from(document.querySelectorAll(".device-summary-dialog button")).find(b=>b.textContent.includes("Obtain Firmware")).disabled'),true);
  if(await evaluate('!!document.querySelector(".acquisition-results")')) {
    assert.equal(await evaluate('document.querySelector(".acquisition-results").textContent.includes("No network request made.")'),true);
    assert.equal(await evaluate('document.querySelectorAll(".acquisition-results > details").length'),6);
    const report=await evaluate(`window.__TAURI_INTERNALS__.invoke('get_discovery').then(async snapshot=>{const device=snapshot.devices.find(d=>d.connected&&['router_mifi_candidate','usb_modem_candidate'].includes(d.category));if(!device)return null;return window.__TAURI_INTERNALS__.invoke('discover_firmware_routes',{input:{deviceId:device.id,endpoint:null,interfaceId:null,approvalToken:null,advancedEndpoint:false}});})`);
    if(report){assert.equal(report.probe.attempted,false);assert.equal(report.canObtain,false);assert.equal(report.candidates.length,5);assert(report.candidates.every(c=>!c.capability.executable));record(label+': current USB candidate through acquisition IPC; no probe without approval; five independently unavailable routes');}
  }
  await screenshot(label+'-device-network-summary');
  if(label==='production' && await evaluate('!!document.querySelector(".acquisition-results")')) {
    const destination=path.join(profile.root,'fixtures','passive-acquisition-diagnostics.json');
    await evaluate('[...document.querySelectorAll(".device-summary-dialog button")].find(b=>b.textContent==="Export Diagnostics").click()');
    execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.resolve('scripts/drive-file-dialog.ps1'),'-DialogTitle','Export acquisition diagnostics','-SelectedPath',destination,'-OwnerPid',String(pid)],{windowsHide:true,timeout:30000});
    await until(()=>evaluate('document.querySelector(".device-summary-dialog").textContent.includes("Redacted diagnostics saved.")'),'Passive diagnostics save not confirmed');
    const saved=JSON.parse(await readFile(destination,'utf8'));assert.equal(saved.probe.attempted,false);assert.equal(saved.canObtain,false);assert.equal(saved.strategies.length,5);
    assert(!JSON.stringify(saved).includes('deviceId'));assert(!JSON.stringify(saved).includes('endpoint'));
    await writeFile(path.join(evidence,'acquisition-diagnostics-redacted.json'),JSON.stringify(saved,null,2));
    const original=await readFile(destination);
    await evaluate('[...document.querySelectorAll(".device-summary-dialog button")].find(b=>b.textContent==="Export Diagnostics").click()');
    execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.resolve('scripts/drive-file-dialog.ps1'),'-DialogTitle','Export acquisition diagnostics','-SelectedPath',destination,'-OwnerPid',String(pid)],{windowsHide:true,timeout:30000});
    await until(()=>evaluate('document.querySelector(".device-summary-dialog").textContent.includes("Choose a new report filename")'),'Existing diagnostics must not be overwritten');
    assert.deepEqual(await readFile(destination),original);
    record('production: actual native diagnostics save, backend redaction and no overwrite; no network approval fabricated');
  }
  if(label==='production' && await evaluate('!!document.querySelector(".device-summary-dialog input[placeholder]").value')) {
    await evaluate('document.querySelector(".device-summary-dialog > details").open=true; [...document.querySelectorAll(".device-summary-dialog label")].find(l=>l.textContent.includes("I own or administer")).querySelector("input").click()');
    await evaluate('[...document.querySelectorAll(".device-summary-dialog button")].find(b=>b.textContent==="Check Compatibility").click()');
    execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.resolve('scripts/reject-route-approval.ps1'),'-OwnerPid',String(pid)],{windowsHide:true,timeout:20000});
    await until(()=>evaluate('document.querySelector(".device-summary-dialog").textContent.includes("Endpoint check not approved.")'),'Native approval cancellation did not reach React');
    assert.equal(await evaluate('!!document.querySelector(".acquisition-results")'),false);
    await screenshot('production-route-approval-rejected');
    record('production: real native endpoint approval rejected with No; React recovers without network request or report');
  }
  await evaluate('document.querySelector(".device-summary-dialog .icon-button").click()');
  record(label+': compact native cards, backend route-discovery popup, no false acquisition success');
  if(label==='production' && process.argv.includes('--acquisition-mg11')) {
    const inventory=await evaluate("window.__TAURI_INTERNALS__.invoke('get_discovery')");
    const target=inventory.devices.find(d=>d.connected&&d.vendorId==='1286'&&d.productId==='4E31'&&d.networks.filter(n=>n.linkUp&&!n.virtualInterface&&n.gateways.includes('192.168.100.1')).length===1);
    if(!target){record('H01 physical acquisition probe PENDING: known USB/network candidate unavailable');return;}
    await evaluate(`document.querySelector('[data-device-id="${target.id}"] button').click()`);
    await until(()=>evaluate('!!document.querySelector(".device-summary-dialog[open]")'),'H01 popup missing');
    await evaluate('document.querySelector(".device-summary-dialog > details").open=true');
    assert.equal(await evaluate('document.querySelector(".device-summary-dialog input[placeholder]").value'),'http://192.168.100.1/');
    await evaluate('[...document.querySelectorAll(".device-summary-dialog label")].find(l=>l.textContent.includes("I own or administer")).querySelector("input").click()');
    await evaluate('[...document.querySelectorAll(".device-summary-dialog button")].find(b=>b.textContent==="Check Compatibility").click()');
    record('H01 awaiting owner confirmation in native approve-read dialog; hardware check cannot proceed silently');
    await until(()=>evaluate('!!document.querySelector(".acquisition-results")'),'Owner-authorized H01 root probe failed');
    const text=await evaluate('document.querySelector(".acquisition-results").innerText');
    assert(text.includes('Identity: unverified'));assert(text.includes('No currently qualified acquisition method'));
    const diagnostics=path.join(profile.root,'fixtures','acquisition-diagnostics.json');
    await evaluate('[...document.querySelectorAll(".device-summary-dialog button")].find(b=>b.textContent==="Export Diagnostics").click()');
    execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.resolve('scripts/drive-file-dialog.ps1'),'-DialogTitle','Export acquisition diagnostics','-SelectedPath',diagnostics,'-OwnerPid',String(pid)],{windowsHide:true,timeout:30000});
    await until(()=>evaluate('document.querySelector(".device-summary-dialog").textContent.includes("Redacted diagnostics saved.")'),'Diagnostics save not confirmed');
    const report=JSON.parse(await readFile(diagnostics,'utf8'));
    assert.equal(report.canObtain,false);assert.equal(report.probe.attempted,true);assert.equal(report.strategies.length,5);
    assert(!JSON.stringify(report).includes('192.168.100.1'));assert(!JSON.stringify(report).includes(target.id));
    await writeFile(path.join(evidence,'h01-acquisition-redacted.json'),JSON.stringify(report,null,2));
    await screenshot('production-h01-acquisition');
    await evaluate('document.querySelector(".device-summary-dialog .icon-button").click()');
    record(report.probe.httpStatus===null ? 'H01 physical HTTP BLOCKED: selected-interface route/transport check returned no HTTP response; GUI/IPC and redacted diagnostics verified' : 'H01 real native GUI approval → bounded root response → candidate family → separate routes → redacted diagnostics; no firmware bytes or writes',{probeCode:report.probe.code,httpStatus:report.probe.httpStatus,protocolCandidates:report.protocolCandidates});
  }
}

