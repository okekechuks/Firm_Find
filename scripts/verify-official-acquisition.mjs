// Explicitly requested live public-source test. Never accesses a router endpoint.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir,readFile,writeFile,copyFile,constants } from 'node:fs/promises';
import path from 'node:path';
import { assertOwned } from './native-test-profile.mjs';

export async function verifyOfficialAcquisition(client,{root,profile,evidence,record,screenshot}) {
  const evaluate=expression=>client.evaluate(expression);
  const invoke=(command,args={})=>evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
  const until=async(check,message,budget=30000)=>{const deadline=Date.now()+budget;while(Date.now()<deadline){if(await check())return;await new Promise(resolve=>setTimeout(resolve,150));}throw new Error(message);};
  const clickText=text=>evaluate(`Array.from(document.querySelectorAll('button')).find(b=>b.textContent.trim()===${JSON.stringify(text)}).click()`);
  const sources=await invoke('list_official_sources');assert.equal(sources.length,3);
  const source=sources.find(item=>item.profile==='tplink_archer-c7-v5');assert(source);assert.equal(source.installability,'unknown');
  await evaluate("document.querySelector('a[href=\"#firmware\"]').click()");
  await until(()=>evaluate("document.querySelector('h1')?.textContent==='Library'"),'Library did not open');
  await clickText('Official Firmware');
  await until(()=>evaluate("document.querySelector('dialog[aria-label=\"Official firmware\"] select')?.options.length===3"),'Catalogue did not render');
  assert.equal(await evaluate("Array.from(document.querySelectorAll('dialog button')).find(b=>b.textContent==='Download to Library').disabled"),true);
  await screenshot('official-before-approval');
  await evaluate("document.querySelector('dialog[aria-label=\"Official firmware\"] input[type=checkbox]').click()");
  await clickText('Download to Library');
  let job;
  await until(async()=>{job=(await invoke('list_library_jobs')).find(item=>item.kind==='official_image_download'&&item.originalFilename===source.filename);return Boolean(job);},'Official download did not create a persistent job');
  await until(async()=>{job=await invoke('get_library_job',{id:job.id});return ['completed','failed','cancelled'].includes(job.status);},'Official download did not finish within bounded budget',220000);
  assert.equal(job.status,'completed',JSON.stringify(job));assert.equal(job.relatedArtifact,source.sha256);assert.equal(job.progressBytes,source.sizeBytes);
  const details=await invoke('get_firmware_artifact',{sha256:source.sha256});
  assert.equal(details.artifact.artifactClass,'official_image_download');assert.equal(details.artifact.sizeBytes,source.sizeBytes);
  const provenance=await invoke('get_official_provenance',{sha256:source.sha256});assert.equal(provenance[0].id,source.id);assert.equal(provenance[0].sha256,source.sha256);
  const blob=path.join(profile.root,'data/vault/sha256',source.sha256.slice(0,2),source.sha256+'.bin');await assertOwned(profile,blob);
  const bytes=await readFile(blob);const actual=createHash('sha256').update(bytes).digest('hex');assert.equal(actual,source.sha256);assert.equal(bytes.length,source.sizeBytes);
  const retained=path.join(root,'.tools/sprint/acquired');await mkdir(retained,{recursive:true});
  const copy=path.join(retained,source.filename);
  try{await copyFile(blob,copy,constants.COPYFILE_EXCL);}catch(error){if(error.code!=='EEXIST')throw error;assert.equal(createHash('sha256').update(await readFile(copy)).digest('hex'),source.sha256);}
  await writeFile(path.join(evidence,'official-image-acquisition.json'),JSON.stringify({source,job,details,provenance,actualSha256:actual,actualBytes:bytes.length,retainedFile:copy},null,2));
  await until(()=>evaluate("Array.from(document.querySelectorAll('dialog button')).some(b=>b.textContent==='Open in Library')"),'Completed download did not offer Library navigation');
  await clickText('Open in Library');
  await until(()=>evaluate("document.querySelector('.firmware-actions button')?.disabled===false"),'Acquired artifact details did not render');
  await screenshot('official-in-library');
  await clickText('Analyze Firmware');
  await until(()=>evaluate("document.querySelector('dialog[aria-label=\"Firmware analysis\"]')?.open===true"),'Analysis popup missing');
  await clickText('Analyze firmware');
  let report;
  await until(async()=>{report=await invoke('get_firmware_analysis',{sha256:source.sha256});return Boolean(report);},'Persisted format analysis did not appear',60000);
  assert.equal(report.inputSha256,source.sha256);assert.equal(report.inputBytes,source.sizeBytes);assert.equal(report.recursiveExtraction,false);assert.equal(report.executableContentsRun,false);assert.equal(report.compatibility,'unknown');assert(report.regions.length>0);
  await until(()=>evaluate(`document.querySelector('dialog[aria-label="Firmware analysis"]')?.innerText.includes(${JSON.stringify(report.format)})`),'Actual analysis results did not render');
  await screenshot('official-analysis');
  await writeFile(path.join(evidence,'official-image-analysis.json'),JSON.stringify(report,null,2));
  await evaluate("document.querySelector('[aria-label=\"Close firmware analysis\"]').click()");
  record('actual public official-image download, independent byte/hash check, classified Library admission, provenance, rendered bounded analysis',{filename:source.filename,bytes:bytes.length,sha256:actual,format:report.format,regions:report.regions.length,retainedFile:copy});
  return source.sha256;
}
