// Actual native Windows GUI + isolated real-image extraction. No router requests.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile,writeFile,copyFile,readdir,stat} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import path from 'node:path';
import {gzipSync} from 'node:zlib';
import {assertOwned} from './native-test-profile.mjs';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const until = async(check,message) => {const deadline=Date.now()+190000;while(Date.now()<deadline){if(await check())return;await new Promise(r=>setTimeout(r,150));}throw new Error(message);};
async function retainManifest(profile,evidence,report) {
  const vault=path.join(profile.root,'data/vault/sha256');await assertOwned(profile,vault);
  for(const prefix of await readdir(vault,{withFileTypes:true})) {
    if(!prefix.isDirectory()||!/^[a-f0-9]{2}$/.test(prefix.name))continue;
    for(const name of await readdir(path.join(vault,prefix.name))) {
      if(!/^[a-f0-9]{64}\.bin$/.test(name))continue;
      const file=path.join(vault,prefix.name,name);await assertOwned(profile,file);
      if((await stat(file)).size>8*1024*1024)continue;
      const raw=await readFile(file);let parsed;
      try{parsed=JSON.parse(raw.toString('utf8'));}catch{continue;}
      if(parsed.bundleSha256!==report.bundleSha256||parsed.inputSha256!==report.inputSha256)continue;
      const hash=digest(raw);assert.equal(name,hash+'.bin');
      await writeFile(path.join(evidence,'extraction-manifest-'+report.container+'.json'),raw);
      return hash;
    }
  }
  throw new Error('Published extraction manifest not found');
}

export async function verifyExtraction(client,{root,profile,evidence,record,screenshot,pid,hash,reopened=false}) {
  const evaluate=expression=>client.evaluate(expression);
  const invoke=(command,args={})=>evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
  const click=text=>evaluate(`Array.from(document.querySelectorAll('button')).find(b=>b.textContent.trim()===${JSON.stringify(text)}).click()`);
  await evaluate("document.querySelector('a[href=\"#firmware\"]').click()");
  await until(()=>evaluate("document.querySelector('h1')?.textContent==='Library'"),'Library missing');
  await until(()=>evaluate("document.querySelector('.artifact-card')!==null || document.querySelector('.firmware-card')!==null"),'Firmware card missing');
  await evaluate("(document.querySelector('.firmware-card')||document.querySelector('.artifact-card')).click()");
  await until(()=>evaluate("document.querySelector('.firmware-actions button')?.disabled===false"),'Actions not ready');
  await click('Analyze Firmware');
  await until(()=>evaluate("document.querySelector('dialog[aria-label=\"Firmware analysis\"]')?.open"),'Analysis popup missing');
  let report;
  if(!reopened) {
    await click('Extract Contents');
    await until(()=>evaluate("Array.from(document.querySelectorAll('button')).some(b=>b.textContent.trim()==='Cancel extraction'&&!b.disabled)"),'Cancel action missing');
    await click('Cancel extraction');
    let cancelledJob;
    await until(async()=>{cancelledJob=(await invoke('list_library_jobs')).find(j=>j.kind==='firmware_extraction'&&j.relatedArtifact===hash);return cancelledJob&&['completed','failed','cancelled'].includes(cancelledJob.status);},'Cancelled extraction did not settle');
    assert.equal(cancelledJob.status,'cancelled',JSON.stringify(cancelledJob));
    assert.equal(await invoke('get_firmware_extraction',{sha256:hash}),null);
    await until(()=>evaluate("Array.from(document.querySelectorAll('button')).some(b=>b.textContent.trim()==='Extract Contents'&&!b.disabled)"),'Extraction did not recover after cancellation');
    record('native GUI cancellation: no extraction published; retry available',{jobId:cancelledJob.id});
    await click('Extract Contents');
    let job;
    await until(async()=>{job=(await invoke('list_library_jobs')).find(j=>j.kind==='firmware_extraction'&&j.relatedArtifact===hash&&j.id!==cancelledJob.id);return Boolean(job);},'Extraction job missing');
    await until(async()=>{job=await invoke('get_library_job',{id:job.id});return ['completed','failed','cancelled'].includes(job.status);},'Extraction timed out');
    assert.equal(job.status,'completed',JSON.stringify(job));
    await writeFile(path.join(evidence,'extraction-job.json'),JSON.stringify(job,null,2));
  }
  report=await invoke('get_firmware_extraction',{sha256:hash});
  assert(report);assert.equal(report.inputSha256,hash);assert.equal(report.workerExit,0);
  const manifestSha256=await retainManifest(profile,evidence,report);
  const source=path.join(profile.root,'data/vault/sha256',hash.slice(0,2),hash+'.bin');await assertOwned(profile,source);
  assert.equal(digest(await readFile(source)),hash);
  const bundle=path.join(profile.root,'data/vault/sha256',report.bundleSha256.slice(0,2),report.bundleSha256+'.bin');await assertOwned(profile,bundle);
  const bytes=await readFile(bundle);assert.equal(digest(bytes),report.bundleSha256);
  let actualBytes=0,files=0;
  for(const entry of report.entries.filter(e=>e.kind==='file')) {
    const content=bytes.subarray(entry.offset,entry.offset+entry.size);
    assert.equal(content.length,entry.size);assert.equal(digest(content),entry.sha256);actualBytes+=content.length;files++;
  }
  assert.equal(actualBytes,report.totalBytes);assert(files>500);
  for(const name of ['etc/openwrt_release','bin/busybox','sbin/procd','usr/lib/os-release'])assert(report.entries.some(e=>e.path===name&&e.kind==='file'),name);
  const release=await invoke('read_extracted_text',{sha256:hash,path:'etc/openwrt_release'});assert(release.includes("DISTRIB_RELEASE='25.12.6'"));
  await until(()=>evaluate("document.querySelector('.extraction-section')?.innerText.includes('Browse Extracted Files')"),'Persisted extracted tree did not render');
  await click('etc/');await click('openwrt_release');
  await until(()=>evaluate("document.querySelector('.extraction-preview')?.innerText.includes('25.12.6')"),'Actual text preview missing');
  await screenshot(reopened?'extraction-reopened':'extraction-real-tree');
  await writeFile(path.join(evidence,reopened?'extraction-reopened.json':'extraction-report.json'),JSON.stringify(report,null,2));
  if(!reopened) {
    await copyFile(bundle,path.join(evidence,'extracted-contents.bundle'));
  } else {
    await evaluate("document.querySelector('[aria-label=\"Select etc/openwrt_release for export\"]').click()");
    await evaluate("Array.from(document.querySelectorAll('.extraction-section label input')).find(i=>i.parentElement.innerText.includes('I reviewed')).click()");
    const archive=path.join(profile.root,'exports','selected-real-files.tar');await assertOwned(profile,archive);
    await click('Export Extracted Files');
    execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(root,'scripts/drive-file-dialog.ps1'),'-DialogTitle','Export selected extracted files','-SelectedPath',archive,'-OwnerPid',String(pid)],{windowsHide:true,timeout:30000});
    await until(()=>evaluate("document.querySelector('.extraction-section')?.innerText.includes('Selected files exported')"),'Export did not complete');
    const exported=await readFile(archive);assert.equal(exported.subarray(0,18).toString(),'etc/openwrt_release'.slice(0,18));
    const exportedSize=parseInt(exported.subarray(124,135).toString(),8);assert.equal(exported.subarray(512,512+exportedSize).toString(),release);
    assert.equal(exported[156],48);assert.equal(exported.subarray(100,107).toString(),'0000600');
    await copyFile(archive,path.join(evidence,'selected-real-files.tar'));
    await writeFile(path.join(evidence,'extraction-export.json'),JSON.stringify({sha256:digest(exported),bytes:exported.length,selectedFiles:['etc/openwrt_release'],contentSha256:digest(Buffer.from(release))},null,2));
  }
  await evaluate("document.querySelector('[aria-label=\"Close firmware analysis\"]').click()");
  record(reopened?'native restart: extraction browsing, real text preview and selected-file TAR export':'native extraction: actual SquashFS files, independent hashes, rendered tree and text preview',{files,directories:report.entries.filter(e=>e.kind==='directory').length,bytes:actualBytes,inputSha256:hash,bundleSha256:report.bundleSha256,manifestSha256});
  if(reopened) {
    // A real firmware payload in a locally generated gzip wrapper, not an OEM
    // package or a second official firmware download. Source remains unchanged.
    const wrapped=gzipSync(await readFile(source),{level:6});const wrappedHash=digest(wrapped);
    const filename='owner-created-openwrt-wrapper.gz';const input=path.join(profile.root,'fixtures',filename);await assertOwned(profile,input);await writeFile(input,wrapped);
    await click('Import Firmware');
    execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(root,'scripts/drive-file-dialog.ps1'),'-DialogTitle','Import firmware','-SelectedPath',input,'-OwnerPid',String(pid)],{windowsHide:true,timeout:30000});
    await until(()=>evaluate("document.querySelector('[aria-label=\"Confirm firmware import\"]')!==null"),'Gzip import confirmation missing');
    await click('Confirm import');
    await until(async()=>{const page=await invoke('list_firmware_artifacts',{query:{}});return page.artifacts.some(a=>a.sha256===wrappedHash);},'Gzip wrapper not imported');
    await until(()=>evaluate(`Array.from(document.querySelectorAll('.artifact-card')).some(e=>e.querySelector('h3')?.textContent===${JSON.stringify(filename)})`),'Gzip card missing');
    await evaluate(`Array.from(document.querySelectorAll('.artifact-card')).find(e=>e.querySelector('h3')?.textContent===${JSON.stringify(filename)}).click()`);
    await until(()=>evaluate("document.querySelector('.firmware-actions button')?.disabled===false"),'Gzip actions missing');await click('Analyze Firmware');
    await until(()=>evaluate("document.querySelector('dialog[aria-label=\"Firmware analysis\"]')?.open"),'Gzip dialog missing');await click('Extract Contents');
    let gzipJob;
    await until(async()=>{gzipJob=(await invoke('list_library_jobs')).find(j=>j.kind==='firmware_extraction'&&j.relatedArtifact===wrappedHash);return gzipJob&&['completed','failed','cancelled'].includes(gzipJob.status);},'Gzip extraction unfinished');
    assert.equal(gzipJob.status,'completed',JSON.stringify(gzipJob));
    const gzipReport=await invoke('get_firmware_extraction',{sha256:wrappedHash});assert.equal(gzipReport.container,'gzip');assert.equal(gzipReport.expandedBytes,7405862);
    const gzipManifestSha256=await retainManifest(profile,evidence,gzipReport);
    assert.deepEqual(gzipReport.entries.map(({path,sha256,size,kind})=>({path,sha256,size,kind})),report.entries.map(({path,sha256,size,kind})=>({path,sha256,size,kind})));
    assert.equal(digest(await readFile(source)),hash);
    await writeFile(path.join(evidence,'gzip-extraction-report.json'),JSON.stringify({wrapperSha256:wrappedHash,wrapperBytes:wrapped.length,report:gzipReport},null,2));
    await until(()=>evaluate("document.querySelector('.extraction-section')?.innerText.includes('Browse Extracted Files')"),'Gzip tree not rendered');
    await screenshot('gzip-real-firmware-tree');
    await evaluate("document.querySelector('[aria-label=\"Close firmware analysis\"]').click()");
    record('additional gzip-to-SquashFS route: real OpenWrt payload in explicitly local wrapper; every extracted file matches original route',{wrapperSha256:wrappedHash,wrapperBytes:wrapped.length,files:gzipReport.fileCount,totalBytes:gzipReport.totalBytes,manifestSha256:gzipManifestSha256});
  }
}
