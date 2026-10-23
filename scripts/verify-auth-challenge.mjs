// Released owner-supervised challenge run. Writes computed observations only.
// No credentials, automatic Yes, menu fetch, retry, raw headers or retained receipts.
import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
export async function verifyAuthChallenge(client,{label,pid,evidence}) {
  assert.equal(label,'production');
  const evaluate=source=>client.evaluate(source);
  const observations={startedAt:new Date().toISOString(),root:null,challenge:null,decline:null,dialogs:[]};
  const save=()=>writeFile(path.join(evidence,'challenge-observations.json'),JSON.stringify(observations,null,2));
  async function until(check,message,timeout=300000){const deadline=Date.now()+timeout;while(Date.now()<deadline){if(await check())return;await new Promise(resolve=>setTimeout(resolve,100));}throw new Error(message);}
  async function dialog(stage,title){
    const native=JSON.parse(execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.resolve('scripts/read-native-dialog.ps1'),'-OwnerPid',String(pid),'-Title',title],{windowsHide:true,encoding:'utf8',timeout:15000}));
    observations.dialogs.push({stage,...native});await save();console.log(JSON.stringify({stage,title,observedAt:native.observedAt}));
  }
  const inventory=await evaluate("window.__TAURI_INTERNALS__.invoke('get_discovery')");
  const target=inventory.devices.find(d=>d.connected&&d.vendorId==='1286'&&d.productId==='4E31'&&d.networks.filter(n=>n.linkUp&&!n.virtualInterface&&n.gateways.includes('192.168.100.1')).length===1);
  assert(target,'Selected owner USB candidate unavailable; no request sent.');
  const network=target.networks.find(n=>n.linkUp&&!n.virtualInterface&&n.gateways.includes('192.168.100.1'));
  const input={deviceId:target.id,endpoint:'http://192.168.100.1/',interfaceId:network.id,approvalToken:null,advancedEndpoint:false};
  async function step(approval,read,projection){
    await evaluate(`(()=>{window.__challengeStep={phase:'waiting_approval',readCalls:0};void(async()=>{
      let calls=0;try{const input=${JSON.stringify(input)};
        const receipt=await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(approval)},{input});
        calls++;window.__challengeStep={phase:'reading',readCalls:calls};
        const result=await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(read)},{input:{...input,approvalToken:receipt}});
        window.__challengeStep={phase:'returned',readCalls:calls,value:${projection}};
      }catch(error){window.__challengeStep={phase:'rejected',readCalls:calls,code:error?.code||'unknown'};}
    })();})()`);
  }
  async function result(){await until(()=>evaluate("['returned','rejected'].includes(window.__challengeStep.phase)"),'Native approval/read did not finish');return evaluate('window.__challengeStep');}
  await evaluate(`document.querySelector('[data-device-id="${target.id}"] button').click()`);
  await until(()=>evaluate('!!document.querySelector(".device-summary-dialog[open]")'),'Native React popup missing',20000);
  await step('approve_firmware_route_probe','discover_firmware_routes','{probe:result.probe,protocols:result.protocols.map(p=>p.family),canObtain:result.canObtain}');
  await dialog('root_approval','OBTAINER - approve read-only check');
  observations.root=await result();await save();
  if(observations.root.phase!=='returned'||!observations.root.value.protocols.includes('asr_duster_xml')||observations.root.value.probe.code!=='identity_unknown')return;
  await step('approve_auth_challenge','read_auth_challenge','{authChallenge:result.authChallenge,canObtain:result.canObtain}');
  await dialog('challenge_yes_approval','OBTAINER - approve authentication challenge');
  observations.challenge=await result();await save();console.log(JSON.stringify({stage:'challenge_result',value:observations.challenge}));
  const observed=observations.challenge.value?.authChallenge;
  // Any positive-run stop condition ends here. No automatic extra fetch or workaround.
  if(observations.challenge.phase!=='returned'||observed?.code!=='observed'||observed.httpStatus!==401||observed.setCookiePresent||observed.otherSchemesPresent)return;
  assert.equal(observations.challenge.readCalls,1);assert.equal(observations.challenge.value.canObtain,false);
  await step('approve_auth_challenge','read_auth_challenge','{authChallenge:result.authChallenge,canObtain:result.canObtain}');
  await dialog('challenge_no_approval','OBTAINER - approve authentication challenge');
  observations.decline=await result();await save();
  assert.equal(observations.decline.phase,'rejected');assert.equal(observations.decline.code,'CANCELLED');assert.equal(observations.decline.readCalls,0);
}
