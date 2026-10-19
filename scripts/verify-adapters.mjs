// Phase 5 checks use real native IPC and Windows observations. No management network requests.
import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import path from "node:path";
async function until(check,message){const end=Date.now()+30000;while(Date.now()<end){if(await check())return;await new Promise(r=>setTimeout(r,100));}throw new Error(message);}
export async function verifyAdapters(client,{label,evidence,record}){
  const invoke=async(command,args={})=>{
    const result=await client.evaluate(`(async()=>{try{return {ok:true,value:await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})};}catch(error){return {ok:false,error};}})()`);
    if(!result.ok)throw result.error;
    return result.value;
  };
  const query=(deviceId,targetKind="discovered")=>invoke("get_device_adapter_matches",{input:{deviceId,targetKind}});
  const descriptors=await invoke("list_available_adapters");
  assert.equal(descriptors.length,1);assert.equal(descriptors[0].implementationStatus,"declared");
  assert(!descriptors.some(d=>d.implementationStatus==="test_only"));
  for(const args of [{input:{deviceId:"../unsafe"}},{input:{deviceId:"A".repeat(64)}},{input:{deviceId:"0".repeat(64),authorized:true}}]){
    await assert.rejects(()=>invoke("get_device_adapter_matches",args));
  }
  await assert.rejects(()=>query("0".repeat(64)),e=>e.code==="DEVICE_NOT_FOUND");
  for(const command of ["apply_adapter_configuration","execute_command","extract_device_firmware"]){
    await assert.rejects(()=>invoke(command,{authorized:true,operation:"modify_configuration"}));
  }
  record(label+": real adapter registry IPC, strict request validation and absent write/execution endpoints");
  let snapshot=await invoke("get_discovery");const results=[];
  for(const device of snapshot.devices.filter(d=>d.connected).slice(0,12)){
    let result;
    try{result=await query(device.id);}catch(e){if(e.code==="OPERATION_CANCELLED"){snapshot=await invoke("get_discovery");result=await query(device.id);}else throw e;}
    assert.equal(result.deviceId,device.id);assert.notEqual(result.connectionState,"connected_read_only");
    assert.equal(result.selectedAdapter,null);
    for(const match of result.matches){
      for(const capability of match.capabilities){assert.equal(capability.canApply,false);assert.equal(capability.canRead,false);}
      for(const feature of Object.values(match.features)){assert.equal(feature.assessment,"UNKNOWN");assert.equal(feature.canWrite,false);}
    }
    if(device.networks.length&&!device.transports.includes("usb")){
      assert.equal(device.category,"generic_network_interface");assert.equal(result.protectedDevice,true);
    }
    results.push(result);
  }
  const mg11=snapshot.devices.find(d=>d.connected&&d.vendorId==="1286"&&d.productId==="4E31");
  if(mg11){const result=await query(mg11.id);assert.equal(result.selectedAdapter,null);results.push(result);}
  await writeFile(path.join(evidence,label+"-adapters.json"),JSON.stringify({descriptors,results,mg11Connected:Boolean(mg11)},null,2));
  record(label+": passive Windows-to-profile-to-adapter evaluation; host NICs protected, four features unknown, no fabricated connection",{devices:results.length,mg11Connected:Boolean(mg11)});
  await client.evaluate('document.querySelector(\'nav a[href="#devices"]\').click()');
  await until(()=>client.evaluate('Boolean(document.querySelector(".device-toolbar"))'),"Devices page missing");
  await client.evaluate('(()=>{const label=[...document.querySelectorAll("label")].find(e=>e.textContent.includes("Show advanced Windows"));const input=label?.querySelector("input");if(input&&!input.checked)input.click();})()');
  await until(()=>client.evaluate('Boolean(document.querySelector(".device-card button[aria-label^=\\"Check compatibility\\"]"))'),"Visible real Windows device cards missing");
  await client.evaluate('document.querySelector(".device-card button[aria-label^=\\"Check compatibility\\"]").click()');
  await until(()=>client.evaluate('Boolean(document.querySelector(".adapter-summary button:not([disabled])"))'),"Adapter status did not load through React and Rust");
  assert.equal(await client.evaluate('Boolean(document.querySelector(".adapter-summary [role=alert]"))'),false);
  assert(await client.evaluate('document.querySelector(".adapter-summary").textContent.includes("unavailable")'));
  await client.evaluate('document.querySelector(".device-card button[aria-label^=\\"More info\\"]").click()');
  await until(()=>client.evaluate('Boolean(document.querySelector(".device-info-dialog[open]"))'),"More info popup missing");
  assert(await client.evaluate('[...document.querySelectorAll(".device-info-dialog button")].find(b=>b.textContent.includes("Extract Firmware")).disabled'));
  await client.evaluate('document.querySelector("[aria-label=\\"Close device information\\"]").click()');
  await client.evaluate('document.querySelector(".adapter-summary").scrollIntoView({block:"center"})');
  await client.evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
  const screenshot=await client.send("Page.captureScreenshot",{format:"png"});
  await writeFile(path.join(evidence,label+"-phase5-adapter-status.png"),Buffer.from(screenshot.data,"base64"));
  record(label+": actual native React adapter status, protected/unsupported display and disabled extraction");
  const address="http://192.168.254.253:54321/";
  const before=await invoke("get_manual_routers");assert(!before.some(r=>r.address===address),"Fixture address already exists; refuse overwrite");
  await client.evaluate('[...document.querySelectorAll("button")].find(b=>b.textContent==="Add Router").click()');
  await until(()=>client.evaluate('Boolean(document.querySelector(".router-form form"))'),"Add Router form missing");
  await client.evaluate(`(()=>{const inputs=document.querySelectorAll('.router-form input');const set=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set;set.call(inputs[0],${JSON.stringify(address)});inputs[0].dispatchEvent(new Event('input',{bubbles:true}));set.call(inputs[1],'Phase 5 isolated endpoint');inputs[1].dispatchEvent(new Event('input',{bubbles:true}));})()`);
  await client.evaluate('document.querySelector(".router-form form").requestSubmit()');
  await until(()=>client.evaluate('Boolean([...document.querySelectorAll(".saved-router")].find(e=>e.textContent.includes("Phase 5 isolated endpoint")))'),"Saved endpoint UI missing");
  const routers=await invoke("get_manual_routers");
  const saved=routers.find(r=>r.address===address);assert(saved);
  try{
    const result=await query(saved.id,"saved_endpoint");assert.equal(result.connectionState,"identity_verification_required");assert.equal(result.selectedAdapter,null);
    assert.equal((await invoke("get_manual_routers")).find(r=>r.id===saved.id).lastResult,null);
    await until(()=>client.evaluate('Boolean([...document.querySelectorAll(".saved-router")].find(e=>e.textContent.includes("Phase 5 isolated endpoint")))'),"Saved endpoint UI missing");
    await client.evaluate('(()=>{const row=[...document.querySelectorAll(".saved-router")].find(e=>e.textContent.includes("Phase 5 isolated endpoint"));[...row.querySelectorAll("button")].find(b=>b.textContent==="Adapter status").click();})()');
    await until(()=>client.evaluate('Boolean(document.querySelector(".saved-connections-dialog .adapter-summary button:not([disabled])"))'),"Saved endpoint adapter IPC missing");
    assert(await client.evaluate('document.querySelector(".saved-connections-dialog .adapter-summary").textContent.includes("No compatible adapter")'));
    await client.evaluate('document.querySelector("[aria-label=\\"Close saved connections\\"]").click()');
  }finally{await invoke("remove_manual_router",{id:saved.id});}
  record(label+": saved endpoint adapter query and GUI remain passive; no HTTP observation or device record invented");
}
