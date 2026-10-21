// Native WebView2 verification. Uses real IPC and owner-supplied observations;
// it never supplies verified identity evidence or writes to a router.
import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import path from 'node:path';

let originalObservationIds;
export async function verifyProfiles(client, {label, evidence, record}) {
  const evaluate = (s) => client.evaluate(s);
  const invoke = (command, args = {}) => evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
  const until = async (check, message) => {
    const deadline = Date.now()+20000;
    while (Date.now()<deadline) { if (await check()) return; await new Promise((r) => setTimeout(r,150)); }
    throw new Error(message);
  };
  const registry=await invoke('get_profile_registry');assert.equal(registry.profiles.filter(p=>p.origin!=='owned_openwrt_os_lab').length,2);assert.equal(registry.profiles.filter(p=>p.origin==='owned_openwrt_os_lab').length,process.argv.includes('--phase7')?1:0);assert.deepEqual(registry.diagnostics,[]);
  const observation=registry.profiles.find((p) => p.profile.profileId === 'observed.comfi-mg11');assert.equal(observation.authoritative,false);
  const reference=registry.profiles.find(p=>p.profile.profileId==='tplink.archer-c7-v5.openwrt-23-05-5');assert(reference);
  const valid=await invoke('validate_device_profile',{document:JSON.stringify(reference.profile)});assert.equal(valid.authoritative,false);assert.equal(valid.origin,'local_unverified');
  assert.equal(valid.profile.firmwareVersionConstraints.scheme,undefined,'Legacy numeric profile serialization changed');
  const opaque={...valid.profile,profileId:'fixture.opaque-preview',firmwareVersionConstraints:{scheme:'opaque_exact',exact:['MF69_AM802L_ZX_DD_N_241106_EU'],minInclusive:null,maxExclusive:null}};
  const preview=await invoke('validate_device_profile',{document:JSON.stringify(opaque)});
  assert.equal(preview.authoritative,false);assert.equal(preview.profile.schemaVersion,1);assert.equal(preview.profile.firmwareVersionConstraints.scheme,'opaque_exact');
  for (const document of ['{',JSON.stringify({...valid.profile,schemaVersion:99}),JSON.stringify({...valid.profile,script:'untrusted.exe'}),
    JSON.stringify({...opaque,firmwareVersionConstraints:{...opaque.firmwareVersionConstraints,minInclusive:'1.0.0',maxExclusive:'2.0.0'}}),
    JSON.stringify({...opaque,firmwareVersionConstraints:{...opaque.firmwareVersionConstraints,scheme:'guess_dates'}}),
    JSON.stringify({...opaque,firmwareVersionConstraints:{...opaque.firmwareVersionConstraints,exact:[]}}),
    JSON.stringify({...opaque,firmwareVersionConstraints:{...opaque.firmwareVersionConstraints,scheme:'numeric_triplet'}}),
  ]) {
    const result=await evaluate(`(async()=>{try{await window.__TAURI_INTERNALS__.invoke('validate_device_profile',{document:${JSON.stringify(document)}});return null;}catch(e){return e;}})()`);
    assert.equal(result.code,'INVALID_PROFILE');
  }
  assert.equal((await invoke('get_profile_registry')).digest,registry.digest);
  record(label+': real registry IPC, strict rejection, local preview never activated');

  const discovery=await invoke('get_discovery');
  const device=discovery.devices.find((d) => d.connected && d.vendorId === '1286' && d.productId === '4E31');
  assert(device,'Connected owner-reported MG11 USB candidate missing; physical qualification remains pending');
  await evaluate("window.location.hash='compatibility'");
  await until(() => evaluate("Boolean(document.querySelector('.compatibility-selector select'))"),'Compatibility screen missing');
  await evaluate("[...document.querySelectorAll('summary')].find(s=>s.textContent==='Validate a local profile document').parentElement.open=true");
  await evaluate(`(()=>{const i=document.querySelector('.compatibility-panel textarea');Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value').set.call(i,${JSON.stringify(JSON.stringify(opaque))});i.dispatchEvent(new Event('input',{bubbles:true}));})()`);
  await evaluate("[...document.querySelectorAll('button')].find(b=>b.textContent==='Validate profile').click()");
  await until(() => evaluate("document.body.innerText.includes('Valid schema: fixture.opaque-preview')"),'React/Rust opaque profile validation failed');
  assert.equal((await invoke('get_profile_registry')).digest,registry.digest);
  record(label+': schema-1 opaque preview passes real React/Rust IPC; invalid constraints rejected; registry unchanged');
  await evaluate(`(()=>{const s=document.querySelector('.compatibility-selector select');s.value=${JSON.stringify(device.id)};s.dispatchEvent(new Event('change',{bubbles:true}));})()`);
  await until(() => evaluate("Boolean(document.querySelector('.support-table tbody tr'))"),'Real compatibility matrix did not render');
  const before=await invoke('get_device_compatibility',{deviceId:device.id});assert.equal(before.selectedProfile,null);

  // Record only the exact user-reported text. LT800/ has no verified meaning.
  await evaluate("[...document.querySelectorAll('summary')].find(s=>s.textContent==='Add user-reported identity evidence').parentElement.open=true");
  for (const [attribute,value] of [['reported_identifier','LT800/'],['firmware_version','MF69_AM802L_ZX_DD_N_241106_EU']]) {
    await until(() => evaluate("!document.querySelector('.reported-evidence button').disabled"),'Evidence form stayed disabled');
    await evaluate(`(()=>{const f=document.querySelector('.reported-evidence');const s=f.querySelector('select');s.value=${JSON.stringify(attribute)};s.dispatchEvent(new Event('change',{bubbles:true}));const i=f.querySelector('input');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(i,${JSON.stringify(value)});i.dispatchEvent(new Event('input',{bubbles:true}));})()`);
    // Separate event turns ensure React commits controlled field state before submit.
    await evaluate("document.querySelector('.reported-evidence').requestSubmit()");
    await until(() => evaluate("document.querySelector('.reported-evidence input')?.value==='' && !document.querySelector('.reported-evidence button').disabled"),'Reported evidence save/evaluation did not finish');
  }
  const report=await invoke('get_device_compatibility',{deviceId:device.id});
  assert.equal(report.engineVersion,'1.1.0','Native build must use hardened engine');
  assert.equal(report.confidence,'CANDIDATE');assert.equal(report.selectedProfile,null);assert.equal(report.features.wifiUserLimit.verifiedRange,null);
  for (const f of Object.values(report.features)) {assert.equal(f.assessment,'UNKNOWN');assert.equal(f.canApply,false);assert.equal(f.adapterImplemented,false);assert(f.readiness.includes('NOT_IMPLEMENTED'));}
  const reported=report.evidence.filter((e) => ['LT800/','MF69_AM802L_ZX_DD_N_241106_EU'].includes(e.value));assert.equal(reported.length,2);
  for (const e of reported) {assert.equal(e.source,'user_supplied');assert.equal(e.verification,'user_reported');assert.equal(e.reliability,'hint');}
  assert.equal(reported.find((e) => e.value === 'LT800/').attribute,'reported_identifier');
  assert(!report.evidence.some((e) => e.attribute==='hardware_revision' && e.value==='LT800/'));
  const ids=reported.map((e) => e.id).sort();originalObservationIds ??= ids;assert.deepEqual(ids,originalObservationIds);
  assert.equal(await evaluate("document.querySelectorAll('.support-table tbody tr').length"),4);
  assert.equal(await evaluate("[...document.querySelectorAll('.compatibility-panel button')].some(b=>/apply/i.test(b.textContent))"),false);
  record(label+': MG11 real React/Rust matrix; four UNKNOWN features; owner text remains unverified', {deviceId:device.id,confidence:report.confidence,reported:reported.map(({attribute,value,verification}) => ({attribute,value,verification}))});
  record(label+': reported evidence survives same-value saves and restarts', {observationIds:ids});
  await evaluate("document.querySelector('.support-table').scrollIntoView({block:'center'})");
  const image=await client.send('Page.captureScreenshot',{format:'png'});await writeFile(path.join(evidence,label+'-compatibility.png'),Buffer.from(image.data,'base64'));
  await writeFile(path.join(evidence,label+'-profiles.json'),JSON.stringify({registry,report},null,2));
  await evaluate("document.querySelector('a[href=\"#devices\"]').click()");
  await until(() => evaluate("Boolean(document.querySelector('.device-card'))"),'Device cards missing');
  await evaluate("[...document.querySelectorAll('.device-card')].find(b=>b.textContent.includes('VID 1286')&&b.textContent.includes('PID 4E31')).click()");
  await until(() => evaluate("Boolean(document.querySelector('.support-table tbody tr'))"),'Device details did not load compatibility');
  record(label+': connected-device details use the real compatibility engine');
}
