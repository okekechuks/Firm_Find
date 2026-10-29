// Synthetic identity + genuine public firmware. This is never physical success.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile,writeFile} from 'node:fs/promises';
import path from 'node:path';
import {retainManifest} from './verify-extraction.mjs';
import {verifyFirmwareStructure} from './verify-firmware-structure.mjs';
import {assertOwned} from './native-test-profile.mjs';
const id='1'.repeat(64);
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const until=async(check,message)=>{const end=Date.now()+220000;while(Date.now()<end){if(await check())return;await new Promise(r=>setTimeout(r,150));}throw new Error(message);};
export async function verifyUniversalWorkflow(client,{profile,evidence,record,screenshot,reopened=false}) {
  const evaluate=expression=>client.evaluate(expression);
  const invoke=(command,args={})=>evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
  const click=label=>evaluate(`Array.from(document.querySelectorAll('dialog button')).find(b=>b.textContent.trim()===${JSON.stringify(label)}).click()`);
  const inventory=await invoke('get_discovery');assert.equal(inventory.devices.length,1);assert.equal(inventory.devices[0].id,id);assert(inventory.warnings[0].includes('Synthetic'));
  await evaluate("window.location.hash='devices'");
  await until(()=>evaluate("document.querySelector('.device-card button')!==null"),'Synthetic workflow target not rendered');
  await evaluate("document.querySelector('.device-card button').click()");
  await until(()=>evaluate("document.querySelector('.workflow-source')!==null"),'Qualified workflow source not rendered');
  let workflow=await invoke('get_device_workflow',{deviceId:id});const source=workflow.officialSources[0];
  assert(source);assert.equal(source.revision,'v5');assert.equal(source.sha256,'0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d');
  assert(workflow.configuration.every(s=>!s.executable));assert(workflow.stages.filter(s=>['customization','rebuild','installation','recovery'].includes(s.id)).every(s=>!s.executable));
  if(!reopened) {
    await assert.rejects(()=>invoke('start_device_firmware_acquisition',{binding:workflow.binding,sourceId:source.id,confirmed:false}),/Approve this exact|SOURCE_APPROVAL_REQUIRED/);
    assert.equal((await invoke('list_library_jobs')).length,0);
    assert.equal(await evaluate("Array.from(document.querySelectorAll('dialog button')).find(b=>b.textContent.trim()==='Obtain Firmware').disabled"),true);
    await evaluate("document.querySelector('.workflow-source input').click()");await click('Obtain Firmware');
    await until(async()=>{workflow=await invoke('get_device_workflow',{deviceId:id});return workflow.artifacts.length===1;},'Real official bytes not admitted through device workflow');
    const acquisition=workflow.jobs.find(j=>j.kind==='official_image_download'&&j.status==='completed');assert(acquisition);assert.equal(acquisition.deviceId,id);assert(acquisition.recoveryInfo.includes(workflow.binding.identityDigest));
    await until(async()=>{workflow=await invoke('get_device_workflow',{deviceId:id});return Boolean(workflow.artifacts[0]?.analysis);},'Automatic bounded analysis did not complete');
    assert.equal(workflow.artifacts[0].source.sha256,source.sha256);assert.equal(workflow.artifacts[0].artifact.artifactClass,'official_image_download');assert.equal(workflow.artifacts[0].artifact.associatedDeviceId,id);
    await until(()=>evaluate("Array.from(document.querySelectorAll('dialog button')).some(b=>b.textContent.trim()==='Extract Contents'&&!b.disabled)"),'Direct extraction action not available');
    await evaluate("document.querySelector('[aria-label=\"Acquired firmware\"]').scrollIntoView({block:'start'})");
    await screenshot('workflow-acquired-analyzed');
    await click('Extract Contents');
    await until(()=>evaluate("Array.from(document.querySelectorAll('dialog button')).some(b=>b.textContent.trim()==='Cancel extraction'&&!b.disabled)"),'Extraction Cancel not rendered');
    await click('Cancel extraction');
    let cancelled;
    await until(async()=>{cancelled=(await invoke('list_library_jobs')).find(j=>j.kind==='firmware_extraction');return cancelled&&['completed','failed','cancelled'].includes(cancelled.status);},'Cancellation did not settle');
    assert.equal(cancelled.status,'cancelled',JSON.stringify(cancelled));assert.equal(await invoke('get_firmware_extraction',{sha256:source.sha256}),null);
    await until(()=>evaluate("Array.from(document.querySelectorAll('dialog button')).some(b=>b.textContent.trim()==='Extract Contents'&&!b.disabled)"),'Extraction retry unavailable');
    await click('Extract Contents');
    await until(async()=>{const jobs=await invoke('list_library_jobs');const j=jobs.find(j=>j.kind==='firmware_extraction'&&j.id!==cancelled.id);if(j?.status==='failed')throw new Error(JSON.stringify(j));return j?.status==='completed';},'Real filesystem extraction did not complete');
    record('device workflow: synthetic trusted exact target, real official HTTPS download, associated immutable Library original, automatic analysis, native cancel/retry extraction',{sourceSha256:source.sha256,sourceBytes:source.sizeBytes,physicalDeviceSuccess:false});
  }
  let report;await until(async()=>{report=await invoke('get_firmware_extraction',{sha256:source.sha256});return Boolean(report);},'Persisted extraction missing');
  const manifestSha256=await retainManifest(profile,evidence,report);
  await verifyFirmwareStructure(client,{report,evidence,reopened});
  assert.equal(report.fileCount,869);assert.equal(report.totalBytes,13411262);assert.equal(report.isolation.rootReadOnly,true);
  const vault=path.join(profile.root,'data/vault/sha256');const sourceFile=path.join(vault,source.sha256.slice(0,2),source.sha256+'.bin');await assertOwned(profile,sourceFile);
  const original=await readFile(sourceFile);assert.equal(hash(original),source.sha256);assert.equal(original.length,source.sizeBytes);
  const bundle=path.join(vault,report.bundleSha256.slice(0,2),report.bundleSha256+'.bin');await assertOwned(profile,bundle);const bytes=await readFile(bundle);assert.equal(hash(bytes),report.bundleSha256);
  for(const e of report.entries.filter(e=>e.kind==='file'))assert.equal(hash(bytes.subarray(e.offset,e.offset+e.size)),e.sha256);
  await until(()=>evaluate("document.querySelector('.extraction-section')?.innerText.includes('Browse Extracted Files')"),'Workflow tree not rendered');await click('etc/');await click('openwrt_release');
  await until(()=>evaluate("document.querySelector('.extraction-preview')?.innerText.includes('25.12.6')"),'Extracted firmware text not rendered');
  // A real integrity job emits background Library events. Those must refresh
  // state without unmounting the user's existing file browser/preview.
  const integrity=await invoke('verify_firmware_integrity',{sha256:source.sha256});
  await until(async()=>{const job=await invoke('get_library_job',{id:integrity.id});assert.notEqual(job.status,'failed');return job.status==='completed';},'Integrity background job did not finish');
  await until(()=>evaluate("Array.from(document.querySelectorAll('.notification-toast')).some(t=>t.innerText.includes('Integrity verification complete'))"),'Background event was not consumed by React');
  await until(()=>evaluate("document.querySelector('[aria-label=\"Device firmware workflow\"]').getAttribute('aria-busy')==='false'"),'Background workflow reload did not finish');
  assert(await evaluate("document.querySelector('.extraction-preview')?.innerText.includes('25.12.6')"),'Background refresh destroyed the selected file preview');
  const viewport=await evaluate("new Promise(resolve=>{const p=document.querySelector('.extraction-preview');p.scrollIntoView({block:'center',behavior:'instant'});requestAnimationFrame(()=>requestAnimationFrame(()=>{const r=p.getBoundingClientRect();resolve({connected:p.isConnected,top:r.top,bottom:r.bottom,width:r.width,height:r.height,viewportHeight:innerHeight,dialogScrollTop:document.querySelector('dialog').scrollTop,text:p.innerText});}));})");
  assert(viewport.connected&&viewport.width>0&&viewport.height>0&&viewport.top>=0&&viewport.bottom<=viewport.viewportHeight,JSON.stringify(viewport));
  await writeFile(path.join(evidence,reopened?'workflow-reopened-viewport.json':'workflow-viewport.json'),JSON.stringify(viewport,null,2));
  await screenshot(reopened?'workflow-reopened-tree':'workflow-real-tree');
  workflow=await invoke('get_device_workflow',{deviceId:id});assert(workflow.artifacts[0].extracted);assert(workflow.artifacts[0].analysis);assert.equal(workflow.stages.find(s=>s.id==='rebuild').executable,false);
  for(const stage of ['acquisition','analysis','extraction'])assert.equal(workflow.stages.find(s=>s.id===stage).job.status,'completed',stage);
  await writeFile(path.join(evidence,reopened?'workflow-restart.json':'workflow-completed.json'),JSON.stringify({workflow,sourceSha256:hash(original),manifestSha256,bundleSha256:hash(bytes),physicalDeviceSuccess:false},null,2));
  if(reopened) {
    await evaluate("document.querySelector('.device-firmware-workflow a').click()");
    await until(()=>evaluate("document.querySelector('h1')?.textContent==='Library'&&document.querySelector('.firmware-actions')!==null"),'Direct associated Library link did not select the artifact');
    record('device workflow restart: acquired provenance, analyzed bytes, extraction index, all file hashes, rendered real text, direct selected Library link',{sourceSha256:source.sha256,manifestSha256,bundleSha256:report.bundleSha256,files:report.fileCount,physicalDeviceSuccess:false});
  }
}
