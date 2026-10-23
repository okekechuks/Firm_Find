// Owner-supervised B4 run. No automatic dialog answers, raw HTTP bodies or retained receipts.
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
  async function dialog(stage,title){
    const native=JSON.parse(execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.resolve('scripts/read-native-dialog.ps1'),'-OwnerPid',String(pid),'-Title',title],{windowsHide:true,encoding:'utf8',timeout:15000}));
    observed.dialogs.push({stage,...native});await save();console.log(JSON.stringify({stage,dialogTitle:title,observedAt:native.observedAt}));
  }
  const inventory=await evaluate("window.__TAURI_INTERNALS__.invoke('get_discovery')");
  const target=inventory.devices.find(d=>d.connected&&d.vendorId==='1286'&&d.productId==='4E31'&&d.networks.filter(n=>n.linkUp&&!n.virtualInterface&&n.gateways.includes('192.168.100.1')).length===1);
  assert(target,'Owner MG11 USB candidate unavailable; no endpoint request made.');
  const network=target.networks.find(n=>n.linkUp&&!n.virtualInterface&&n.gateways.includes('192.168.100.1'));
  const input={deviceId:target.id,endpoint:'http://192.168.100.1/',interfaceId:network.id,approvalToken:null,advancedEndpoint:false};
  // Tauri invoke is immutable. Call real IPC directly; never replace it or mint a receipt.
  async function step(approval,read,projection){
    await evaluate(`(()=>{window.__b4Step={phase:'waiting_approval',readCalls:0};void(async()=>{
      let calls=0;try{const input=${JSON.stringify(input)};
        const receipt=await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(approval)},{input});
        calls++;window.__b4Step={phase:'reading',readCalls:calls};
        const result=await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(read)},{input:{...input,approvalToken:receipt}});
        window.__b4Step={phase:'returned',readCalls:calls,finishedAt:Date.now(),value:${projection}};
      }catch(error){window.__b4Step={phase:'rejected',readCalls:calls,finishedAt:Date.now(),code:error?.code||'unknown'};}
    })();})()`);
  }
  async function result(){await until(()=>evaluate("['returned','rejected'].includes(window.__b4Step.phase)"),'Native approval/read did not finish');return evaluate('window.__b4Step');}
  const rootProjection="{probe:result.probe,protocols:result.protocols.map(p=>p.family),observedVersions:result.identity.evidence.filter(e=>e.attribute==='firmware_version'&&e.verification==='observed').map(e=>e.value),canObtain:result.canObtain}";
  const menuProjection='{staticMenu:result.staticMenu,canObtain:result.canObtain}';
  await evaluate(`document.querySelector('[data-device-id="${target.id}"] button').click()`);
  await until(()=>evaluate('!!document.querySelector(".device-summary-dialog[open]")'),'MG11 React popup missing',20000);
  await step('approve_firmware_route_probe','discover_firmware_routes',rootProjection);
  await dialog('root_approval','OBTAINER - approve read-only check');
  observed.root=await result();await save();
  if(observed.root.phase!=='returned'||!observed.root.value.protocols.includes('asr_duster_xml')||observed.root.value.probe.code!=='identity_unknown')return;
  await step('approve_static_menu','read_static_menu',menuProjection);
  await dialog('menu_yes_approval','OBTAINER - approve static menu check');
  observed.menu=await result();await save();console.log(JSON.stringify({stage:'menu_result',value:observed.menu}));
  // Failure stops the positive run. A separate declined approval sends no further request.
  if(observed.menu.phase!=='returned')return;
  await step('approve_static_menu','read_static_menu',menuProjection);
  await dialog('menu_no_approval','OBTAINER - approve static menu check');
  observed.menuDecline=await result();await save();
  assert.equal(observed.menuDecline.phase,'rejected');assert.equal(observed.menuDecline.code,'CANCELLED');assert.equal(observed.menuDecline.readCalls,0);
}
