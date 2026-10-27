// Native correction qualification: genuine signed TRX inputs and labelled synthetic failures.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile,writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import path from 'node:path';
import {assertOwned} from './native-test-profile.mjs';
import {retainManifest} from './verify-extraction.mjs';
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
const until=async(check,message)=>{const deadline=Date.now()+190000;while(Date.now()<deadline){if(await check())return;await new Promise(r=>setTimeout(r,150));}throw new Error(message);};

export async function verifyExtractionReview(client,{root,profile,evidence,pid,record,screenshot}) {
  const evaluate=expression=>client.evaluate(expression);
  const invoke=(name,args={})=>evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(name)},${JSON.stringify(args)})`);
  const click=text=>evaluate(`Array.from(document.querySelectorAll('button')).find(b=>b.textContent.trim()===${JSON.stringify(text)}).click()`);
  const fixtures=JSON.parse(await readFile(path.join(root,'.tools/trx/verified-fixtures.json'),'utf8'));
  const cases=[
    {filename:'openwrt-25.12.6-bcm47xx-mips74k-asus_rt-n16-squashfs.trx',folder:'trx',sha256:'0f29bd64f7a8f931cf1121dd54b514f1ecb58486ffa1713a282c97928c703d5d',container:'trx'},
    {filename:'openwrt-25.12.6-bcm53xx-generic-asus_rt-ac68u-squashfs.trx',folder:'trx',sha256:'1827a286bc03df7a83ccd93e84cb1375cc1e10c1857ded3eb9c0ad413f761d36',error:'UNSUPPORTED_NESTED_FORMAT'},
    {filename:'synthetic-resource-limit.gz',folder:'review-fixtures',error:'EXTRACTION_RESOURCES'},
    {filename:'synthetic-export-names.sqfs',folder:'review-fixtures',container:'raw'},
  ];
  let retainedTrx;
  for(const item of cases) {
    const source=path.join(root,'.tools',item.folder,item.filename);const original=await readFile(source);const hash=digest(original);
    if(item.sha256){assert.equal(hash,item.sha256);assert(fixtures.some(f=>f.sha256===hash));}
    await evaluate("document.querySelector('a[href=\"#firmware\"]').click()");
    await click('Import Firmware');
    execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(root,'scripts/drive-file-dialog.ps1'),'-DialogTitle','Import firmware','-SelectedPath',source,'-OwnerPid',String(pid)],{windowsHide:true,timeout:30000});
    await until(()=>evaluate("document.querySelector('[aria-label=\"Confirm firmware import\"]')!==null"),'Import confirmation missing');await click('Confirm import');
    await until(()=>evaluate(`Array.from(document.querySelectorAll('.artifact-card')).some(e=>e.querySelector('h3')?.textContent===${JSON.stringify(item.filename)})`),'Imported card missing');
    await evaluate(`Array.from(document.querySelectorAll('.artifact-card')).find(e=>e.querySelector('h3')?.textContent===${JSON.stringify(item.filename)}).click()`);
    await until(()=>evaluate("document.querySelector('.firmware-actions button')?.disabled===false"),'Actions unavailable');await click('Analyze Firmware');
    await until(()=>evaluate("document.querySelector('dialog[aria-label=\"Firmware analysis\"]')?.open"),'Analysis dialog missing');await click('Extract Contents');
    let job;
    await until(async()=>{job=(await invoke('list_library_jobs')).find(j=>j.kind==='firmware_extraction'&&j.relatedArtifact===hash);return job&&['completed','failed','cancelled'].includes(job.status);},'Review extraction unfinished');
    const report=await invoke('get_firmware_extraction',{sha256:hash});
    if(item.error) {
      assert.equal(job.status,'failed',JSON.stringify(job));assert.equal(job.errorCode,item.error);assert.equal(report,null);
      await until(()=>evaluate("document.querySelector('.extraction-section [role=\"alert\"]')!==null"),'Persistent failure did not render');
      if(retainedTrx)assert.equal((await invoke('get_firmware_extraction',{sha256:retainedTrx.hash})).bundleSha256,retainedTrx.bundleSha256);
      record('native failure: '+item.error+'; no successful output; prior extraction retained',{filename:item.filename,sha256:hash,errorCode:job.errorCode});
    } else {
      assert.equal(job.status,'completed',JSON.stringify(job));assert.equal(report.container,item.container);
      const bundle=path.join(profile.root,'data/vault/sha256',report.bundleSha256.slice(0,2),report.bundleSha256+'.bin');await assertOwned(profile,bundle);
      const bytes=await readFile(bundle);assert.equal(digest(bytes),report.bundleSha256);
      for(const entry of report.entries.filter(e=>e.kind==='file'))assert.equal(digest(bytes.subarray(entry.offset,entry.offset+entry.size)),entry.sha256);
      const manifestSha256=await retainManifest(profile,evidence,report);
      if(item.container==='trx') {
        assert.equal(report.fileCount,851);assert.equal(report.directoryCount,124);assert.equal(report.totalBytes,12421418);
        assert.equal(report.containerInfo.parser,'OpenWrt otrx');assert.equal(report.region.offset,2634752);assert.equal(report.status,'partially_extracted');
        const text=await invoke('read_extracted_text',{sha256:hash,path:'etc/openwrt_release'});assert(text.includes('25.12.6'));
        retainedTrx={hash,bundleSha256:report.bundleSha256};
        record('native genuine TRX v1: pinned parser, direct SquashFS, all file hashes independently verified',{sha256:hash,files:report.fileCount,directories:report.directoryCount,totalBytes:report.totalBytes,manifestSha256,bundleSha256:report.bundleSha256});
      } else {
        assert(report.entries.some(e=>e.kind==='symlink_metadata'));assert(report.entries.some(e=>e.kind==='special_omitted'));
        for(const name of ['a'.repeat(101),'café','resume\u0301']) {
          const entry=report.entries.find(e=>e.path===name);assert(entry.exportError);
          await until(()=>evaluate(`document.querySelector(${JSON.stringify('[aria-label="Select '+name+' for export"]')})?.disabled===true`),'Unexportable checkbox not disabled');
          const result=await evaluate(`Promise.race([window.__TAURI_INTERNALS__.invoke('export_extracted_files',{sha256:${JSON.stringify(hash)},paths:[${JSON.stringify(name)}]}).then(()=>({unexpected:true}),e=>({code:e.code})),new Promise(r=>setTimeout(()=>r({timeout:true}),2000))])`);
          assert.equal(result.code,'UNEXPORTABLE_FILENAME',JSON.stringify(result));
        }
        record('native export preflight: long and Unicode names rejected before Save; links/special entries inert',{synthetic:true,sha256:hash,manifestSha256});
      }
    }
    assert.equal(digest(await readFile(source)),hash);
    await writeFile(path.join(evidence,item.filename+'-result.json'),JSON.stringify({synthetic:item.folder==='review-fixtures',sha256:hash,job,report},null,2));
    await evaluate("document.querySelector('.extraction-section').scrollIntoView({block:'start'})");await screenshot('review-'+item.filename);
    await evaluate("document.querySelector('[aria-label=\"Close firmware analysis\"]').click()");
  }
}
