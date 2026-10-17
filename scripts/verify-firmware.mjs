// Real native dialogs + React + IPC. All sources generated here; never real router firmware.
import assert from 'node:assert/strict';
import { open,readFile,stat,writeFile,unlink } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import {createReadStream} from 'node:fs';
import {assertOwned} from './native-test-profile.mjs';
import path from 'node:path';
const root=path.resolve(import.meta.dirname,'..');
let fixtures,hash,bytes,interruptedId,profile,prepared=false,largeHash,shutdownVerify,recoveredInterrupted;
export function configureFirmwareTestRun(run){profile=run;fixtures=path.join(run.root,'fixtures');}
async function prepare(){
 if(prepared)return;
 assert(profile,"Native fixtures require an isolated test profile");await assertOwned(profile,fixtures);
 bytes=Buffer.alloc(1024*1024,0x5a);bytes.write('OBTAINER Phase 4 HARMLESS GENERATED TEST BYTES - NOT DEVICE FIRMWARE\n');hash=createHash('sha256').update(bytes).digest('hex');
 await writeFile(path.join(fixtures,'obtainer-generated-phase4.bin'),bytes);await writeFile(path.join(fixtures,'obtainer-identical-alias.bin'),bytes);
 const file=await open(path.join(fixtures,'obtainer-generated-large.bin'),'wx');await file.truncate(256*1024*1024);await file.close();
 const digest=createHash('sha256');for await(const chunk of createReadStream(path.join(fixtures,'obtainer-generated-large.bin')))digest.update(chunk);largeHash=digest.digest('hex');
 await writeFile(path.join(fixtures,'obtainer-fail-transaction.bin'),Buffer.from(bytes).fill(0x51,256));prepared=true;
}
async function until(check,message,timeout=30000){const end=Date.now()+timeout;while(Date.now()<end){if(await check())return;await new Promise(r=>setTimeout(r,60));}throw new Error(message);}
function api(client){return (command,args={})=>client.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);}
function dialog(title,file,pid){return execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(root,'scripts/drive-file-dialog.ps1'),'-DialogTitle',title,'-SelectedPath',file,'-OwnerPid',String(pid)],{windowsHide:true,encoding:'utf8',timeout:30000});}
async function screen(client){await client.evaluate("document.querySelector('a[href=\"#firmware\"]').click()");await until(()=>client.evaluate("Boolean([...document.querySelectorAll('button')].find(b=>b.textContent==='Import Firmware'))"),'Library screen did not render');}
async function choose(client,pid,filename){
 await screen(client);await client.evaluate("[...document.querySelectorAll('button')].find(b=>b.textContent==='Import Firmware').click()");
 dialog('Import firmware',path.join(fixtures,filename),pid);
 await until(()=>client.evaluate("Boolean(document.querySelector('[aria-label=\"Confirm firmware import\"]'))"),'Native file selection did not produce React confirmation');
}
async function importFile(client,pid,filename){
 const invoke=api(client);const old=new Set((await invoke('list_library_jobs')).map(j=>j.id));await choose(client,pid,filename);
 await client.evaluate("[...document.querySelectorAll('button')].find(b=>b.textContent==='Confirm import').click()");let job;
 await until(async()=>{job=(await invoke('list_library_jobs')).find(j=>!old.has(j.id)&&j.kind==='firmware_import');return Boolean(job);},'Import job was not persisted');return job;
}
async function terminal(client,id,timeout=60000){let job;await until(async()=>{job=await api(client)('get_library_job',{id});return ['completed','failed','cancelled'].includes(job.status);},'Library job did not finish',timeout);return job;}
export async function interruptFirmwareImport(client,{pid,record,evidence}){
 await prepare();await checkShutdown(client,record);const job=await importFile(client,pid,'obtainer-generated-large.bin');
 await until(async()=>{const j=await api(client)('get_library_job',{id:job.id});return j.status==='running'&&j.stage==='copying'&&j.progressBytes>0&&j.progressBytes<j.totalBytes;},'Could not observe an active import before interruption');
 interruptedId=job.id;await writeFile(path.join(evidence,'interrupted-import.json'),JSON.stringify({jobId:job.id,source:'harmless generated 256 MiB bytes'},null,2));record('Phase 4: actual copying job recorded before forced termination',{jobId:job.id});
}
export async function verifyFirmware(client,{label,pid,record,evidence}){
 await prepare();const invoke=api(client);const snapshot=await invoke('get_snapshot');assert.equal(snapshot.databaseVersion,4);assert.equal(snapshot.testProfile.runId,profile.id);assert.equal(path.normalize(snapshot.dataDirectory).toLowerCase(),path.join(profile.root,'data').toLowerCase());await checkShutdown(client,record);
 if(recoveredInterrupted){const j=await invoke('get_library_job',{id:recoveredInterrupted.id});assert.equal(j.status,'failed');assert.equal(j.errorCode,'INTERRUPTED');assert.equal(j.finishedAt,recoveredInterrupted.finishedAt);record(label+': repeated startup keeps interrupted job terminal without rerunning it');}
 if(label!=='dev-reopen'){
  const saved=await invoke('get_firmware_artifact',{sha256:hash});assert.equal(saved.artifact.sizeBytes,bytes.length);assert(saved.imports.length>=2);assert.equal(saved.artifact.integrityStatus,'not_checked');
  record(label+': existing artifact/import history available before any new import; stale verified label withdrawn');
 }
 if(interruptedId){const j=await invoke('get_library_job',{id:interruptedId});assert.equal(j.status,'failed');assert.equal(j.errorCode,'INTERRUPTED');assert.equal(await stat(path.join(snapshot.dataDirectory,'vault/staging',interruptedId+'.part')).then(()=>true,()=>false),false);record(label+': interrupted copy recovered as failed, known staging removed, no false success',{jobId:interruptedId});recoveredInterrupted={id:j.id,finishedAt:j.finishedAt};interruptedId=undefined;}
 const registry=await invoke('get_profile_registry');assert.equal(registry.profiles.length,2);assert.deepEqual(registry.diagnostics,[]);
 const discovery=await invoke('get_discovery');if(discovery.devices.length){const r=await invoke('get_device_compatibility',{deviceId:discovery.devices[0].id});assert(Object.values(r.features).every(f=>f.assessment==='UNKNOWN'&&!f.canApply));}
 record(label+': schema 4 and existing Phase 3.1 profile/compatibility IPC preserved');
 const first=await importFile(client,pid,'obtainer-generated-phase4.bin');const a=await terminal(client,first.id);assert.equal(a.status,'completed');assert.equal(a.relatedArtifact,hash);assert.equal(a.progressBytes,bytes.length);
 await until(()=>client.evaluate(`Boolean([...document.querySelectorAll('.artifact-card')].find(b=>b.textContent.includes(${JSON.stringify(hash)})))`),'Committed artifact did not render in React');
 const blob=path.join(snapshot.dataDirectory,'vault/sha256',hash.slice(0,2),hash+'.bin');assert.deepEqual(await readFile(blob),bytes);assert.deepEqual(await readFile(path.join(fixtures,'obtainer-generated-phase4.bin')),bytes);
 record(label+': real native file picker, confirmation, persisted job, SHA-256 vault bytes, original source preserved',{jobId:a.id,sha256:hash,size:bytes.length});
 const duplicate=await importFile(client,pid,'obtainer-identical-alias.bin');const b=await terminal(client,duplicate.id);assert.equal(b.status,'completed');assert.equal(b.duplicate,true);assert.equal(b.relatedArtifact,hash);
 let details=await invoke('get_firmware_artifact',{sha256:hash});assert(details.imports.some(i=>i.originalFilename==='obtainer-generated-phase4.bin'));assert(details.imports.some(i=>i.originalFilename==='obtainer-identical-alias.bin'));assert(details.imports.every(i=>i.sourceCategory==='user_import'&&i.provenanceStatus==='unverified'));
 record(label+': same bytes under another name deduplicate; import occurrences and unverified provenance persist',{sourceCount:details.artifact.sourceCount});
 await until(()=>client.evaluate(`Boolean([...document.querySelectorAll('.artifact-card')].find(b=>b.textContent.includes(${JSON.stringify(hash)})))`),'Artifact card missing');
 await client.evaluate(`[...document.querySelectorAll('.artifact-card')].find(b=>b.textContent.includes(${JSON.stringify(hash)})).click()`);
 await until(()=>client.evaluate("Boolean([...document.querySelectorAll('button')].find(b=>b.textContent==='Verify Integrity'))"),'Artifact details did not render');
 const old=new Set((await invoke('list_library_jobs')).map(j=>j.id));await client.evaluate("[...document.querySelectorAll('button')].find(b=>b.textContent==='Verify Integrity').click()");let verify;
 await until(async()=>{verify=(await invoke('list_library_jobs')).find(j=>!old.has(j.id)&&j.kind==='integrity_verify');return Boolean(verify);},'Integrity job not created');assert.equal((await terminal(client,verify.id)).status,'completed');details=await invoke('get_firmware_artifact',{sha256:hash});assert.equal(details.artifact.integrityStatus,'verified');
 record(label+': actual streamed integrity verification and history',{checks:details.checks.length});
 const manifest=path.join(profile.root,'exports',label+'-manifest-'+Date.now()+'.json');await client.evaluate("[...document.querySelectorAll('button')].find(b=>b.textContent==='Export Manifest').click()");dialog('Export artifact manifest',manifest,pid);
 await until(()=>client.evaluate("document.body.innerText.includes('Manifest saved.')"),'Manifest UI did not confirm export');const exported=JSON.parse(await readFile(manifest,'utf8'));assert.equal(exported.sha256,hash);assert.equal(exported.manifestSchemaVersion,1);assert(!JSON.stringify(exported).includes(snapshot.dataDirectory));assert(!JSON.stringify(exported).includes('associatedDeviceId'));
 record(label+': actual native save dialog and unsigned metadata-only manifest',{manifestSchemaVersion:exported.manifestSchemaVersion});
 if(label==='dev-reopen'){
   const beforeCount=(await invoke('list_firmware_artifacts',{query:{}})).total;const started=Date.now();const big=await importFile(client,pid,'obtainer-generated-large.bin');let progress;
   await until(async()=>{progress=await invoke('get_library_job',{id:big.id});return progress.status==='running'&&progress.stage==='copying'&&progress.progressBytes>0&&progress.progressBytes<progress.totalBytes;},'Actual copy progress was not observed');
   await until(()=>client.evaluate(`Boolean(document.querySelector('[data-job-id="${big.id}"] progress')?.value>0 && [...document.querySelectorAll('[data-job-id="${big.id}"] button')].some(b=>b.textContent==='Cancel job'))`),'Real copy progress did not reach the React view');
   const memory=JSON.parse(execFileSync('powershell.exe',['-NoProfile','-Command',`Get-Process -Id ${pid} | Select-Object WorkingSet64,PeakWorkingSet64,PrivateMemorySize64 | ConvertTo-Json`],{windowsHide:true,encoding:'utf8'}));
   await client.evaluate(`[...document.querySelectorAll('[data-job-id="${big.id}"] button')].find(b=>b.textContent==='Cancel job').click()`);
   const cancelled=await terminal(client,big.id);assert.equal(cancelled.status,'cancelled');assert.equal((await invoke('cancel_library_job',{id:big.id})).status,'cancelled');assert.equal((await invoke('list_firmware_artifacts',{query:{}})).total,beforeCount);
   record(label+': real copy progress, UI cancellation, idempotent retry and no incomplete library artifact',{bytesObserved:progress.progressBytes,totalBytes:progress.totalBytes,elapsedMs:Date.now()-started,copyThroughputMiBPerSecond:progress.progressBytes/1048576/Math.max(.001,(progress.updatedAt-progress.startedAt)/1000),memory});
 }
 await client.evaluate("document.querySelector('a[href=\"#jobs\"]').click()");await until(()=>client.evaluate("document.body.innerText.includes('Persistent library jobs') && document.body.innerText.includes('Firmware file imported')"),'Persisted jobs/activity did not render');
 record(label+': persisted job and activity screens render after restart');
 const counts=(await invoke('get_snapshot')).counts;const listing=await invoke('list_firmware_artifacts',{query:{}});assert.equal(counts.artifacts,listing.total);assert.equal(counts.runningJobs,0);
 await client.evaluate("document.querySelector('a[href=\"#dashboard\"]').click()");
 await until(()=>client.evaluate(`Boolean([...document.querySelectorAll('.stats-grid .stat')].find(c=>c.textContent.includes('Firmware artifacts')&&c.querySelector('strong')?.textContent===${JSON.stringify(String(counts.artifacts))}))`),'Dashboard count did not reflect committed SQLite artifacts');
 record(label+': dashboard displays actual committed artifact count and no running library work',{artifacts:counts.artifacts});
 await writeFile(path.join(evidence,label+'-library.json'),JSON.stringify({artifact:await invoke('get_firmware_artifact',{sha256:hash}),jobs:await invoke('list_library_jobs'),activity:await invoke('get_library_activity')},null,2));
 await screen(client);await until(()=>client.evaluate(`Boolean([...document.querySelectorAll('.artifact-card')].find(b=>b.textContent.includes(${JSON.stringify(hash)})))`),'Persisted library did not render for screenshot');
 await client.evaluate('window.scrollTo(0,0)');await client.evaluate('new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
 const image=await client.send('Page.captureScreenshot',{format:'png'});await writeFile(path.join(evidence,label+'-library.png'),Buffer.from(image.data,'base64'));
}

async function sql(script){await assertOwned(profile);execFileSync('py',['-c','import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.executescript(sys.argv[2]); c.close()',path.join(profile.root,'data/obtainer.db'),script],{windowsHide:true});}
async function nativeSelection(client,pid,filename){
 await client.evaluate("(()=>{window.__phase41Selection=window.__TAURI_INTERNALS__.invoke('select_firmware_file');})()");dialog('Import firmware',path.join(fixtures,filename),pid);
 const result=await client.evaluate('window.__phase41Selection');await client.evaluate('delete window.__phase41Selection');return result;
}
async function selectDetails(client){await screen(client);await until(()=>client.evaluate(`Boolean([...document.querySelectorAll('.artifact-card')].find(b=>b.textContent.includes(${JSON.stringify(hash)})))`),'Test artifact missing');await client.evaluate(`[...document.querySelectorAll('.artifact-card')].find(b=>b.textContent.includes(${JSON.stringify(hash)})).click()`);await until(()=>client.evaluate("Boolean([...document.querySelectorAll('button')].find(b=>b.textContent==='Export Manifest'))"),'Details missing');}
export async function verifyFirmwareHardening(client,{pid,evidence,record}){
 await assertOwned(profile);const invoke=api(client);
 const picks=[await nativeSelection(client,pid,'obtainer-generated-large.bin'),await nativeSelection(client,pid,'obtainer-generated-large.bin')];
 const jobs=await client.evaluate(`Promise.all(${JSON.stringify(picks)}.map(s=>window.__TAURI_INTERNALS__.invoke('start_firmware_import',{input:{ticket:s.ticket,description:'Harmless isolated concurrent test',associatedDeviceId:null}})))`);
 assert.notEqual(jobs[0].id,jobs[1].id);const overlap=await Promise.all(jobs.map(j=>invoke('get_library_job',{id:j.id})));assert(overlap.some(j=>j.status==='pending'));assert(overlap.some(j=>j.status==='running'));
 // The serial worker hashes each 256 MiB copy and its published blob. Include queue time in the bounded debug-build deadline.
 const ends=await Promise.all(jobs.map(j=>terminal(client,j.id,180000)));assert(ends.every(j=>j.status==='completed'&&j.relatedArtifact===largeHash));assert.equal(ends.filter(j=>j.duplicate).length,1);assert.equal((await invoke('get_firmware_artifact',{sha256:largeHash})).artifact.sourceCount,2);
 record('isolated native concurrent identical 256 MiB imports: one blob, two durable occurrences');
 const blob=path.join(profile.root,'data/vault/sha256',hash.slice(0,2),hash+'.bin');await assertOwned(profile,blob);const corrupt=Buffer.from(bytes);corrupt[0]^=0xff;
 await writeFile(blob,corrupt);let job=await invoke('verify_firmware_integrity',{sha256:hash});assert.equal((await terminal(client,job.id)).errorCode,'HASH_MISMATCH');assert.equal((await invoke('get_firmware_artifact',{sha256:hash})).artifact.integrityStatus,'hash_mismatch');
 await unlink(blob);job=await invoke('verify_firmware_integrity',{sha256:hash});assert.equal((await terminal(client,job.id)).errorCode,'FILE_MISSING');assert.equal((await invoke('get_firmware_artifact',{sha256:hash})).artifact.integrityStatus,'missing');
 const restored=await terminal(client,(await importFile(client,pid,'obtainer-generated-phase4.bin')).id);assert.equal(restored.status,'completed');assert.deepEqual(await readFile(blob),bytes);
 record('isolated native corruption and missing-blob detection; source reimport restores generated bytes without touching normal vault');
 await sql("CREATE TRIGGER phase41_artifact_failure BEFORE INSERT ON artifact_imports BEGIN SELECT RAISE(FAIL,'isolated artifact commit failure'); END;");let failure;
 try{failure=await terminal(client,(await importFile(client,pid,'obtainer-fail-transaction.bin')).id);assert.equal(failure.errorCode,'DATABASE_UNAVAILABLE');assert.equal(failure.status,'failed');assert(failure.relatedArtifact);await stat(path.join(profile.root,'data/vault/sha256',failure.relatedArtifact.slice(0,2),failure.relatedArtifact+'.bin'));const absent=await client.evaluate(`window.__TAURI_INTERNALS__.invoke('get_firmware_artifact',{sha256:${JSON.stringify(failure.relatedArtifact)}}).then(()=>false,()=>true)`);assert(absent);}finally{await sql('DROP TRIGGER phase41_artifact_failure;');}
 const retry=await terminal(client,(await importFile(client,pid,'obtainer-fail-transaction.bin')).id);assert.equal(retry.status,'completed');assert.equal(retry.duplicate,true);
 record('isolated native database failure after artifact publication: failed job, retained orphan, verified reuse on retry');
 await selectDetails(client);await sql("CREATE TRIGGER phase41_manifest_failure BEFORE INSERT ON activity_links WHEN (SELECT kind FROM audit_events WHERE id=NEW.event_id)='manifest_exported' BEGIN SELECT RAISE(FAIL,'isolated manifest activity failure'); END;");
 const manifest=path.join(profile.root,'exports','manifest-saved-activity-failed.json');
 try{await client.evaluate("[...document.querySelectorAll('button')].find(b=>b.textContent==='Export Manifest').click()");dialog('Export artifact manifest',manifest,pid);await until(()=>client.evaluate("document.body.innerText.includes('Manifest saved, but workspace activity could not be recorded.')"),'Saved export was falsely reported failed');const data=JSON.parse(await readFile(manifest,'utf8'));assert.equal(data.sha256,hash);assert(!JSON.stringify(data).includes(profile.root));}finally{await sql('DROP TRIGGER phase41_manifest_failure;');}
 record('isolated native manifest remains saved when activity transaction fails; React displays separate warning');
 await writeFile(path.join(evidence,'phase41-hardening.json'),JSON.stringify({concurrentJobs:ends,failedCommit:failure,retry,manifestSaved:true,testDataRoot:profile.root},null,2));
}
export async function startShutdownVerification(client,{record,evidence}){
 const j=await api(client)('verify_firmware_integrity',{sha256:largeHash});await until(async()=>{const p=await api(client)('get_library_job',{id:j.id});return p.status==='running'&&p.stage==='verifying'&&p.progressBytes>0&&p.progressBytes<p.totalBytes;},'Verification did not reach real progress before shutdown');shutdownVerify=j.id;await writeFile(path.join(evidence,'verification-before-shutdown.json'),JSON.stringify({jobId:j.id},null,2));record('isolated native verification observed running before graceful application shutdown',{jobId:j.id});
}
async function checkShutdown(client,record){if(!shutdownVerify)return;const job=await api(client)('get_library_job',{id:shutdownVerify});assert.equal(job.status,'cancelled');assert.notEqual((await api(client)('get_firmware_artifact',{sha256:largeHash})).artifact.integrityStatus,'checking');record('graceful shutdown cancelled active verification; restart preserved terminal job and usable artifact');shutdownVerify=undefined;}
