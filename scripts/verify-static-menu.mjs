// Owner-supervised B4 run. Never answers native dialogs or stores raw HTTP bodies.
import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import path from 'node:path';

export async function verifyStaticMenu(client,{label,pid,evidence}) {
  assert.equal(label,'production');
  const evaluate=source=>client.evaluate(source);
  const observed={startedAt:new Date().toISOString(),root:null,menu:null,menuDecline:null,dialogs:[]};
  async function save(){await writeFile(path.join(evidence,'b4-menu-observations.json'),JSON.stringify(observed,null,2));}
  async function until(check,message,timeout=300000){
    const end=Date.now()+timeout;
    while(Date.now()<end){if(await check())return;await new Promise(resolve=>setTimeout(resolve,100));}
    throw new Error(message);
  }
  async function button(text){await evaluate(`[...document.querySelectorAll('.device-summary-dialog button')].find(b=>b.textContent===${JSON.stringify(text)}).click()`);}
  async function dialog(stage,title){
    const native=JSON.parse(execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.resolve('scripts/read-native-dialog.ps1'),'-OwnerPid',String(pid),'-Title',title],{windowsHide:true,encoding:'utf8',timeout:15000}));
    observed.dialogs.push({stage,...native});await save();console.log(JSON.stringify({stage,dialogTitle:title,observedAt:native.observedAt}));
  }
  const inventory=await evaluate("window.__TAURI_INTERNALS__.invoke('get_discovery')");
  const target=inventory.devices.find(d=>d.connected&&d.vendorId==='1286'&&d.productId==='4E31'&&d.networks.filter(n=>n.linkUp&&!n.virtualInterface&&n.gateways.includes('192.168.100.1')).length===1);
  assert(target,'Owner MG11 USB candidate unavailable; no endpoint request made.');
  // Observe actual invoke outcomes. Do not mint receipts, substitute responses, or retain tokens.
  await evaluate(`(()=>{const original=window.__TAURI_INTERNALS__.invoke;window.__b4MenuEvents=[];window.__TAURI_INTERNALS__.invoke=async function(command,args){
    const watched=['approve_firmware_route_probe','discover_firmware_routes','approve_static_menu','read_static_menu'].includes(command);
    if(watched)window.__b4MenuEvents.push({command,phase:'called',time:Date.now()});
    try{const result=await original.call(this,command,args);if(watched){let value=null;
      if(command==='discover_firmware_routes')value={probe:result.probe,protocols:result.protocols.map(p=>p.family),observedVersions:result.identity.evidence.filter(e=>e.attribute==='firmware_version'&&e.verification==='observed').map(e=>e.value),canObtain:result.canObtain};
      if(command==='read_static_menu')value={staticMenu:result.staticMenu,canObtain:result.canObtain};
      window.__b4MenuEvents.push({command,phase:'returned',time:Date.now(),value});}return result;
    }catch(error){if(watched)window.__b4MenuEvents.push({command,phase:'rejected',time:Date.now(),code:error?.code||'unknown'});throw error;}
  };})()`);
  try {
    await evaluate(`document.querySelector('[data-device-id="${target.id}"] button').click()`);
    await until(()=>evaluate('!!document.querySelector(".device-summary-dialog[open]")'),'MG11 popup missing',20000);
    await evaluate('document.querySelector(".device-summary-dialog > details").open=true');
    assert.equal(await evaluate('document.querySelector(".device-summary-dialog input[placeholder]").value'),'http://192.168.100.1/');
    await evaluate('[...document.querySelectorAll(".device-summary-dialog label")].find(l=>l.textContent.includes("I own or administer")).querySelector("input").click()');
    await button('Check Compatibility');
    await dialog('root_approval','OBTAINER - approve read-only check');
    await until(()=>evaluate("window.__b4MenuEvents.some(e=>e.command==='discover_firmware_routes'&&e.phase==='returned')||window.__b4MenuEvents.some(e=>e.command==='approve_firmware_route_probe'&&e.phase==='rejected')"),'Root approval/read did not finish');
    observed.root=await evaluate("window.__b4MenuEvents.find(e=>e.command==='discover_firmware_routes'&&e.phase==='returned')?.value||null");await save();
    if(!observed.root || !observed.root.protocols.includes('asr_duster_xml') || observed.root.probe.code!=='identity_unknown'){return;}
    await button('Read static menu');
    await dialog('menu_yes_approval','OBTAINER - approve static menu check');
    await until(()=>evaluate("window.__b4MenuEvents.some(e=>e.command==='read_static_menu'&&['returned','rejected'].includes(e.phase))||window.__b4MenuEvents.some(e=>e.command==='approve_static_menu'&&e.phase==='rejected')"),'Static menu approval/read did not finish');
    observed.menu=await evaluate("window.__b4MenuEvents.find(e=>e.command==='read_static_menu'&&['returned','rejected'].includes(e.phase))||window.__b4MenuEvents.find(e=>e.command==='approve_static_menu'&&e.phase==='rejected')");await save();
    console.log(JSON.stringify({stage:'menu_result',value:observed.menu}));
    // A fresh approval dialog is the negative run, without a second root/menu fetch.
    // Stop after any non-observed outcome. A later negative run would need a new guideline.
    if(observed.menu?.value?.staticMenu?.code!=='observed')return;
    await button('Read static menu');
    await dialog('menu_no_approval','OBTAINER - approve static menu check');
    await until(()=>evaluate("window.__b4MenuEvents.filter(e=>e.command==='approve_static_menu'&&e.phase==='rejected').length===1"),'Owner did not decline separate static menu approval');
    const events=await evaluate('window.__b4MenuEvents');
    observed.menuDecline={approvalError:events.find(e=>e.command==='approve_static_menu'&&e.phase==='rejected').code,readCalls:events.filter(e=>e.command==='read_static_menu'&&e.phase==='called').length,readResults:events.filter(e=>e.command==='read_static_menu'&&e.phase==='returned').length,secondReadCalls:events.filter(e=>e.command==='read_static_menu'&&e.phase==='called').length-1};
    assert.equal(observed.menuDecline.approvalError,'CANCELLED');assert.equal(observed.menuDecline.secondReadCalls,0);assert.equal(observed.menuDecline.readResults,1);
    await save();
  } finally {
    observed.events=await evaluate('window.__b4MenuEvents');await save();
  }
}
