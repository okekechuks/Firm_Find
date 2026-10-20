// Explicit isolated desktop entry point. No production data-directory override.
import {spawn,spawnSync} from "node:child_process";
import path from "node:path";
import {createTestProfile} from "./native-test-profile.mjs";
const root=path.resolve(import.meta.dirname,"..");
const profile=createTestProfile(root);
console.log(`SIMULATION MODE. Isolated run ${profile.id}. Ephemeral virtual state; no physical device changes.`);
const child=spawn("powershell.exe",["-NoProfile","-Command","npm.cmd run tauri -- dev --config src-tauri/tauri.native-test.conf.json --features native-test-profile -- --bin obtainer-native-test --locked --offline -- --simulator"],{cwd:root,env:{...process.env,OBTAINER_NATIVE_TEST_RUN_ID:profile.id,CARGO_INCREMENTAL:"0"},windowsHide:true,stdio:"inherit"});
child.on("exit",code=>{console.log(`Isolated simulator run retained: ${profile.root}`);process.exitCode=code??1;});
child.on("error",error=>{console.error(error.message);process.exitCode=1;});
for(const signal of ["SIGINT","SIGTERM"])process.on(signal,()=>{if(child.exitCode===null&&child.pid)spawnSync("taskkill",["/PID",String(child.pid),"/T","/F"],{windowsHide:true});});
