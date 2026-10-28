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
  assert.equal(descriptors.length,1);assert.equal(descriptors[0].id,"openwrt-readonly-contract");assert.equal(descriptors[0].implementationStatus,"built_in");
  assert(!descriptors[0].operations.some(op=>['modify_configuration','flash_firmware','send_ussd'].includes(op)));
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
  // The former card-level adapter panel was removed from the product UI.
  // Open a protected Windows observation only; never approve or run a probe.
  const protectedResult=results.find(result=>result.protectedDevice&&snapshot.devices.some(d=>d.id===result.deviceId&&d.connected));
  if(protectedResult){
    await client.evaluate('(()=>{const details=document.querySelector(".other-interfaces");if(details&&!details.open)details.open=true;const label=[...document.querySelectorAll("label")].find(e=>e.textContent.includes("Show advanced Windows"));const input=label?.querySelector("input");if(input&&!input.checked)input.click();})()');
    const selector=`.device-card[data-device-id="${protectedResult.deviceId}"] button[aria-label^="More info"]`;
    await until(()=>client.evaluate(`Boolean(document.querySelector(${JSON.stringify(selector)}))`),"Protected Windows device card missing");
    await client.evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`);
    await until(()=>client.evaluate('Boolean(document.querySelector(".device-info-dialog[open] .device-network-summary"))'),"More info network summary missing");
    await until(()=>client.evaluate('document.querySelector(".device-firmware-workflow")?.getAttribute("aria-busy")==="false"'),"Local device workflow did not load");
    assert(await client.evaluate('Boolean([...document.querySelectorAll(".device-info-dialog button")].find(b=>b.textContent.trim()==="Obtain Firmware"&&b.disabled))'));
    assert.equal(await client.evaluate('Boolean(document.querySelector(".device-info-dialog .acquisition-results"))'),false);
    const screenshot=await client.send("Page.captureScreenshot",{format:"png"});
    await writeFile(path.join(evidence,label+"-phase5-device-info.png"),Buffer.from(screenshot.data,"base64"));
    await client.evaluate('document.querySelector("[aria-label=\\"Close device information\\"]").click()');
    record(label+": protected device information and disabled acquisition shown; no compatibility check or device request",{deviceId:protectedResult.deviceId});
  }else{
    record(label+": protected device information UI not tested; no connected protected Windows record available");
  }
  const address="http://192.168.254.253:54321/";
  const before=await invoke("get_manual_routers");assert(!before.some(r=>r.address===address),"Fixture address already exists; refuse overwrite");
  // Saved-connections controls are no longer mounted. Exercise their local IPC
  // against the isolated profile and do not call probe_management_endpoint.
  const routers=await invoke("add_manual_router",{input:{address,label:"Phase 5 isolated endpoint",interfaceId:null}});
  const saved=routers.find(r=>r.address===address);assert(saved);
  try{
    const result=await query(saved.id,"saved_endpoint");assert.equal(result.connectionState,"identity_verification_required");assert.equal(result.selectedAdapter,null);
    assert.equal((await invoke("get_manual_routers")).find(r=>r.id===saved.id).lastResult,null);
  }finally{await invoke("remove_manual_router",{id:saved.id});}
  record(label+": saved endpoint adapter IPC remains passive; saved-connections GUI no longer mounted and was not tested");
}
