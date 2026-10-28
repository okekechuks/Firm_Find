// Genuine public package, isolated Library only. Never approves a router request.
// An unqualified vendor package is not an installed-device dump or a compatible image.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile,writeFile,copyFile,constants} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import path from 'node:path';
import {assertOwned} from './native-test-profile.mjs';

const filename='1.02SEA--OTA--1.03SEA-diff.zip';
const hash='a592a30abac46942dc8c8b9fdc4254882f4be1b3fa1261e140d6ca5bddfa5743';
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
async function until(check,message) {
  const deadline=Date.now()+190000;
  while(Date.now()<deadline){if(await check())return;await new Promise(resolve=>setTimeout(resolve,150));}
  throw new Error(message);
}

export async function verifyManufacturerPackage(client,{root,profile,evidence,pid,record,screenshot,reopened=false}) {
  const evaluate=source=>client.evaluate(source);
  const invoke=(name,args={})=>evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(name)},${JSON.stringify(args)})`);
  const click=text=>evaluate(`Array.from(document.querySelectorAll('button')).find(b=>b.textContent.trim()===${JSON.stringify(text)}).click()`);
  const snapshot=await invoke('get_snapshot');
  assert.equal(snapshot.testProfile.discoverySource,'windows_observed');
  assert.equal(snapshot.testProfile.simulatedAdapters,false);
  const inventory=await invoke('get_discovery');
  const candidates=inventory.devices.filter(device=>device.connected&&device.vendorId==='1286'&&device.productId==='4E31');
  const observations=[];
  for(const candidate of candidates) {
    const report=await invoke('discover_firmware_routes',{input:{deviceId:candidate.id,endpoint:null,interfaceId:null,approvalToken:null,advancedEndpoint:false}});
    assert.equal(report.probe.attempted,false);assert.equal(report.probe.code,'not_approved');
    assert.equal(report.probe.httpStatus,null);assert.deepEqual(report.protocols,[]);assert.equal(report.canObtain,false);
    assert.equal(report.identity.confidence,'unverified');
    for(const field of ['verifiedManufacturer','verifiedModel','verifiedHardwareRevision','verifiedFirmware'])assert.equal(report.identity[field],null);
    observations.push({vendorId:'1286',productId:'4E31',connected:candidate.connected,networkCount:candidate.networks.length,probe:{attempted:false,code:'not_approved',httpStatus:null},canObtain:false,identity:'unverified',unitVariant:'not_independently_verified'});
  }
  await writeFile(path.join(evidence,reopened?'hardware-local-reopened.json':'hardware-local.json'),JSON.stringify({discoverySource:'windows_observed',candidateCount:candidates.length,observations,routerApprovalCalls:0},null,2));
  const source=path.join(root,'.tools/physical-sprint-artifacts',filename);
  const bytes=await readFile(source);
  assert.equal(bytes.length,10735104);assert.equal(digest(bytes),hash);
  assert.equal(bytes.subarray(0,6).toString('ascii'),'070702');
  await evaluate("document.querySelector('a[href=\"#firmware\"]').click()");
  await until(()=>evaluate("document.querySelector('h1')?.textContent==='Library'"),'Library did not render');
  if(!reopened) {
    const input=path.join(profile.root,'fixtures',filename);await assertOwned(profile,input);
    await copyFile(source,input,constants.COPYFILE_EXCL);
    await click('Import Firmware');
    execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(root,'scripts/drive-file-dialog.ps1'),'-DialogTitle','Import firmware','-SelectedPath',input,'-OwnerPid',String(pid)],{windowsHide:true,timeout:30000});
    await until(()=>evaluate("document.querySelector('[aria-label=\"Confirm firmware import\"]')!==null"),'Import confirmation missing');
    await click('Confirm import');
  }
  await until(()=>evaluate(`Array.from(document.querySelectorAll('.artifact-card')).some(e=>e.querySelector('h3')?.textContent===${JSON.stringify(filename)})`),'Genuine package card missing');
  await evaluate(`Array.from(document.querySelectorAll('.artifact-card')).find(e=>e.querySelector('h3')?.textContent===${JSON.stringify(filename)}).click()`);
  await until(()=>evaluate("document.querySelector('.firmware-actions button')?.disabled===false"),'Package actions unavailable');
  await click('Analyze Firmware');
  await until(()=>evaluate("document.querySelector('dialog[aria-label=\"Firmware analysis\"]')?.open"),'Analysis dialog missing');
  if(!reopened)await click('Analyze firmware');
  let analysis;
  await until(async()=>{analysis=await invoke('get_firmware_analysis',{sha256:hash});return !!analysis;},'Package analysis did not finish');
  assert.equal(analysis.inputSha256,hash);assert.equal(analysis.inputBytes,bytes.length);
  assert.equal(analysis.format,'CPIO CRC archive');assert.equal(analysis.compatibility,'unknown');
  assert.equal(analysis.recursiveExtraction,false);assert.equal(analysis.executableContentsRun,false);
  assert(analysis.limitations.some(text=>text.includes('full firmware image or an incremental update')));
  assert.deepEqual(await invoke('get_official_provenance',{sha256:hash}),[]);
  await until(()=>evaluate("document.querySelector('dialog[aria-label=\"Firmware analysis\"]')?.innerText.includes('CPIO CRC archive')"),'Actual CPIO result did not render');
  if(!reopened) {
    await click('Extract Contents');
    let job;
    await until(async()=>{job=(await invoke('list_library_jobs')).find(j=>j.kind==='firmware_extraction'&&j.relatedArtifact===hash);return job&&['completed','failed','cancelled'].includes(job.status);},'Unsupported package job did not settle');
    assert.equal(job.status,'failed',JSON.stringify(job));assert.equal(job.errorCode,'UNSUPPORTED_FORMAT');
    assert.equal(await invoke('get_firmware_extraction',{sha256:hash}),null);
    await until(()=>evaluate("document.querySelector('.extraction-section [role=\"alert\"]')!==null"),'Unsupported package error did not render');
    await writeFile(path.join(evidence,'manufacturer-package-job.json'),JSON.stringify(job,null,2));
  } else {
    assert.equal(await invoke('get_firmware_extraction',{sha256:hash}),null);
    assert((await invoke('list_library_jobs')).some(j=>j.kind==='firmware_extraction'&&j.relatedArtifact===hash&&j.status==='failed'&&j.errorCode==='UNSUPPORTED_FORMAT'));
  }
  const vault=path.join(profile.root,'data/vault/sha256',hash.slice(0,2),hash+'.bin');await assertOwned(profile,vault);
  assert.equal(digest(await readFile(vault)),hash);assert.equal(digest(await readFile(source)),hash);
  await writeFile(path.join(evidence,reopened?'manufacturer-package-reopened.json':'manufacturer-package-analysis.json'),JSON.stringify({sourceSha256:hash,sourceBytes:bytes.length,analysis,officialProvenance:[],deviceCompatibility:'unverified',physicalDump:false,extraction:null},null,2));
  await screenshot(reopened?'manufacturer-package-reopened':'manufacturer-package-analysis');
  await evaluate("document.querySelector('[aria-label=\"Close firmware analysis\"]').click()");
  record(reopened?'native restart: real delta package and analysis retained; no extraction or compatibility promotion':'native real vendor package: CPIO observed, unsupported extraction rejected, immutable original preserved',{sha256:hash,bytes:bytes.length,physicalDump:false,compatiblePhysicalTarget:false});
}
