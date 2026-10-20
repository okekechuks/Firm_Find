import { invoke, isTauri } from "@tauri-apps/api/core";
import { simulationActionSchema,simulationControlSchema,simulationErrorSchema,simulationRequestSchema,simulationStatusSchema,type SimulationAction,type SimulationControl,type SimulationStatus } from "../types/simulator";
async function call(command:string,input?:unknown):Promise<SimulationStatus>{
 if(!isTauri())throw new Error("Simulation requires the isolated Windows desktop entry point.");
 let response:unknown;
 try {response=await invoke(command,input===undefined?undefined:{input});}
 catch(error){const parsed=simulationErrorSchema.safeParse(error);if(parsed.success)throw parsed.data;throw error;}
 return simulationStatusSchema.parse(response);
}
export function simulationStatus(){return call("simulation_status");}
export function simulationControl(input:SimulationControl){return call("simulation_control",simulationControlSchema.parse(input));}
export function simulationRequest(status:SimulationStatus,action:SimulationAction){
 if(!status.active||!status.device)throw new Error("Start the virtual router first.");
 return call("simulation_request",simulationRequestSchema.parse({target:status.device.id,endpoint:status.device.endpoint,epoch:status.epoch,revision:status.revision,action:simulationActionSchema.parse(action)}));
}
