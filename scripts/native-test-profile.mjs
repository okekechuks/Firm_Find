import assert from 'node:assert/strict';
import { randomBytes } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFile, writeFile, unlink, mkdir, rmdir, stat } from 'node:fs/promises';
import path from 'node:path';

export function createTestProfile(workspace) {
  const id=randomBytes(16).toString('hex');
  const local=execFileSync('powershell.exe',['-NoProfile','-Command',"[Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)"],{windowsHide:true,encoding:'utf8'}).trim();
  return {id,root:path.join(local,'OBTAINER-NativeTests',id),binary:path.join(workspace,'target/release/obtainer-native-test.exe')};
}
export function probeNormal(workspace,evidence,phase) {
  const output=path.join(evidence,'normal-data-'+phase+'.json');
  execFileSync('py',[path.join(workspace,'scripts/probe-native-isolation.py'),output,...(phase==='after'?[path.join(evidence,'normal-data-before.json')]:[])],{windowsHide:true});
}
export async function assertOwned(profile,file=profile.root) {
  assert(/^[a-f0-9]{32}$/.test(profile.id));const relative=path.relative(profile.root,path.resolve(file));assert(!relative.startsWith('..')&&!path.isAbsolute(relative));
  const marker=JSON.parse(await readFile(path.join(profile.root,'.obtainer-native-test.json'),'utf8'));assert.equal(marker.kind,'OBTAINER_NATIVE_TEST');assert.equal(marker.version,1);assert.equal(marker.run_id,profile.id);assert.equal(marker.state,'active');
}
export function cleanupProcess(profile,env) {return spawnSync(profile.binary,['--cleanup-native-test-run'],{env,windowsHide:true,encoding:'utf8'});}
export function verifyActivationGuards(workspace,profile,env,record){
  const run=(binary,variables)=>{const result=spawnSync(binary,[],{env:variables,windowsHide:true,encoding:'utf8',timeout:15000});assert(!result.error,`Activation probe could not launch ${binary}: ${result.error}`);return result;};
  assert.equal(run(path.join(workspace,'target/release/obtainer.exe'),env).status,1);
  record('ordinary production executable rejects test activation before opening data');
  const missing={...env};delete missing.OBTAINER_NATIVE_TEST_RUN_ID;assert.equal(run(profile.binary,missing).status,1);
  assert.equal(run(profile.binary,{...env,OBTAINER_NATIVE_TEST_RUN_ID:'../OBTAINER'}).status,1);
  assert.equal(run(profile.binary,{...env,OBTAINER_DATA_DIR:profile.root}).status,1);
  record('native-test executable rejects missing/unsafe run IDs and arbitrary data paths');
}
export async function verifyCleanupGuards(profile,env,evidence,record) {
  await assertOwned(profile);
  const markerPath=path.join(profile.root,'.obtainer-native-test.json');const original=await readFile(markerPath,'utf8');const wrong={...JSON.parse(original),run_id:'f'.repeat(32)};
  await writeFile(markerPath,JSON.stringify(wrong));try{assert.notEqual(cleanupProcess(profile,env).status,0);assert(await stat(profile.root));}finally{await writeFile(markerPath,original);}
  record('cleanup rejects mismatched marker/run ID and retains test data');
  const staging=path.join(profile.root,'data/vault/staging/unknown-review-item');await writeFile(staging,'unknown staging sentinel');
  try{assert.notEqual(cleanupProcess(profile,env).status,0);assert.equal(await readFile(staging,'utf8'),'unknown staging sentinel');}finally{await unlink(staging);}
  record('cleanup retains unknown staging without deleting files');
  const target=path.join(evidence,'unrelated-cleanup-sentinel');await mkdir(target);await writeFile(path.join(target,'keep.txt'),'outside permitted test root');const link=path.join(profile.root,'fixtures/unsafe-junction');
  execFileSync('powershell.exe',['-NoProfile','-Command',"$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:OBTAINER_TEST_LINK -Target $env:OBTAINER_TEST_TARGET | Out-Null"],{windowsHide:true,env:{...env,OBTAINER_TEST_LINK:link,OBTAINER_TEST_TARGET:target}});
  try{assert.notEqual(cleanupProcess(profile,env).status,0);assert.equal(await readFile(path.join(target,'keep.txt'),'utf8'),'outside permitted test root');}finally{await rmdir(link);}
  record('cleanup refuses reparse traversal; unrelated sentinel remains unchanged');
}
