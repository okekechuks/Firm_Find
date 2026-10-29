// Independent assertions against the real native structural projection.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {writeFile} from 'node:fs/promises';
import path from 'node:path';

export async function verifyFirmwareStructure(client,{report,evidence,reopened=false}) {
  const invoke=()=>client.evaluate(`window.__TAURI_INTERNALS__.invoke('get_firmware_structure',{sha256:${JSON.stringify(report.inputSha256)}})`);
  const structure=await invoke();
  assert(structure);assert.equal(structure.inputSha256,report.inputSha256);
  assert.equal(structure.inputBytes,report.inputBytes);assert.equal(structure.bundleSha256,report.bundleSha256);
  assert.equal(structure.fileCount,report.fileCount);assert.equal(structure.totalExtractedBytes,report.totalBytes);
  for(const key of ['modifiable','repackable','signatureCompatible','installable']) {
    assert.equal(structure.readiness[key].executable,false);
    assert.equal(structure.readiness[key].state,'unknown');
  }
  const index=new Map(structure.regions.map(region=>[region.id,region]));
  assert.equal(index.size,structure.regions.length);
  for(const region of structure.regions) {
    assert(region.length>0);let ancestor=region,decoded=false;
    while(ancestor) {
      if(ancestor.coordinateSpace!=='source')decoded=true;
      ancestor=ancestor.parentId===null?null:index.get(ancestor.parentId);
    }
    if(decoded)assert.equal(region.sourceAbsoluteOffset,null,'Decoded coordinates were presented as source offsets');
    if(region.sourceAbsoluteOffset!==null)assert(region.sourceAbsoluteOffset+region.length<=report.inputBytes);
    if(region.parentId!==null)assert(index.get(region.parentId).children.includes(region.id));
    if(region.id.startsWith('unknown-'))assert.equal(region.sha256,null,'Unknown byte range received an invented SHA');
    for(const hint of region.heuristics)assert.equal(hint.verified,false);
  }
  if(report.version===2) {
    for(const layer of report.layers) {
      const region=index.get(layer.id);assert(region);
      assert.equal(region.relativeOffset,layer.offset);assert.equal(region.length,layer.length);
      assert.equal(region.sha256,layer.inputSha256);
    }
    const unknown=structure.regions.filter(region=>region.id.startsWith('unknown-'));
    assert.equal(unknown.length,report.unexplainedRegions.length);
    report.unexplainedRegions.forEach((gap,ordinal)=>{
      assert.equal(unknown[ordinal].parentId,gap.layerId);
      assert.equal(unknown[ordinal].relativeOffset,gap.offset);assert.equal(unknown[ordinal].length,gap.length);
    });
  }
  assert.deepEqual(await invoke(),structure,'Same immutable input produced a different graph');
  const readyDeadline=Date.now()+30000;
  while(!await client.evaluate("Array.from(document.querySelectorAll('.extraction-section summary')).some(node=>node.textContent.trim()==='Artifact Structure')")) {
    if(Date.now()>=readyDeadline)throw new Error('React did not publish the extraction report/structure action');
    await new Promise(resolve=>setTimeout(resolve,150));
  }
  await client.evaluate("Array.from(document.querySelectorAll('.extraction-section summary')).find(node=>node.textContent.trim()==='Artifact Structure').click()");
  const deadline=Date.now()+30000;
  while(!await client.evaluate("document.querySelector('[aria-label=\"Firmware artifact structure\"] [aria-label=\"Firmware regions\"]')!==null")) {
    if(Date.now()>=deadline)throw new Error('Actual Artifact Structure tree did not render');
    await new Promise(resolve=>setTimeout(resolve,150));
  }
  assert(await client.evaluate("document.querySelector('[aria-label=\"Rebuild readiness\"]').innerText.includes('REPACKABLE: unknown')"));
  const viewport=await client.evaluate("new Promise(resolve=>{const node=document.querySelector('[aria-label=\"Firmware artifact structure\"]');node.scrollIntoView({block:'start',behavior:'instant'});requestAnimationFrame(()=>requestAnimationFrame(()=>{const rect=node.getBoundingClientRect();resolve({top:rect.top,width:rect.width,height:rect.height,viewport:innerHeight});}));})");
  assert(viewport.width>0&&viewport.height>0&&viewport.top>=0&&viewport.top<viewport.viewport,JSON.stringify(viewport));
  const capture=await client.send('Page.captureScreenshot',{format:'png'});
  await writeFile(path.join(evidence,`structure-${report.inputSha256}${reopened?'-reopened':''}.png`),Buffer.from(capture.data,'base64'));
  const bytes=JSON.stringify(structure,null,2)+'\n';
  await writeFile(path.join(evidence,`structure-${report.inputSha256}${reopened?'-reopened':''}.json`),bytes);
  return {regions:structure.regions.length,structureSha256:createHash('sha256').update(bytes).digest('hex')};
}
