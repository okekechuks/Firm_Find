import {beforeEach,expect,it,vi} from "vitest";
import {invoke,isTauri} from "@tauri-apps/api/core";
import {simulationControl,simulationRequest,simulationStatus} from "../lib/simulator";
import {simulationStatusSchema} from "../types/simulator";
import {changedFixture,simulationFixture} from "./simulatorFixtures";
vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn(),isTauri:vi.fn()}));
beforeEach(()=>{vi.clearAllMocks();vi.mocked(isTauri).mockReturnValue(true);vi.mocked(invoke).mockResolvedValue(simulationFixture());});
it("validates status and sends strict control requests",async()=>{expect((await simulationStatus()).device?.id).toBe("sim-router-001");await simulationControl({kind:"start"});expect(invoke).toHaveBeenLastCalledWith("simulation_control",{input:{kind:"start"}});});
it("binds requests to backend-reported endpoint, device, revision and generation",async()=>{const s=simulationFixture();await simulationRequest(s,{kind:"approve_handshake"});expect(invoke).toHaveBeenLastCalledWith("simulation_request",{input:{target:s.device!.id,endpoint:s.device!.endpoint,epoch:1,revision:1,action:{kind:"approve_handshake"}}});});
it("rejects physical, false simulation and unknown response fields",async()=>{
 for(const patch of [{device:{...simulationFixture().device,id:"a".repeat(64)}},{device:{...simulationFixture().device,simulated:false}},{device:{...simulationFixture().device,endpoint:"http://192.168.100.1/"}},{authorized:true}]){
  vi.mocked(invoke).mockResolvedValueOnce({...simulationFixture(),...patch});await expect(simulationStatus()).rejects.toThrow();
 }
});
it("rejects contradictory successful verification and feature payloads",()=>{
 const s=changedFixture();s.outcome!.serviceHealthy=false;expect(simulationStatusSchema.safeParse(s).success).toBe(false);
 const f=changedFixture();f.features!.dns.error={code:"TIMEOUT",message:"Virtual timeout",retryable:true};expect(simulationStatusSchema.safeParse(f).success).toBe(false);
});
it("propagates structured authentication failures without creating success",async()=>{vi.mocked(invoke).mockRejectedValueOnce({code:"AUTHENTICATION_FAILED",message:"Virtual authentication failed; credentials omitted.",retryable:true});await expect(simulationRequest(simulationFixture(),{kind:"connect",approvalId:"sim-approval",credential:"lab-only"})).rejects.toMatchObject({code:"AUTHENTICATION_FAILED"});});
it("requires actual Tauri and active virtual runtime",async()=>{vi.mocked(isTauri).mockReturnValue(false);await expect(simulationStatus()).rejects.toThrow(/isolated Windows/);expect(invoke).not.toHaveBeenCalled();expect(()=>simulationRequest(simulationFixture({active:false,device:null}),{kind:"read_features"})).toThrow(/Start the virtual router/);});
it("shows the explicit wrong-identity fixture without trusting it or accepting physical models",async()=>{
 const fixture=simulationFixture();const wrong={...fixture,scenario:"wrong_identity",device:{...fixture.device!,model:"Wrong Virtual Device"}};
 vi.mocked(invoke).mockResolvedValueOnce(wrong);expect((await simulationStatus()).device?.model).toBe("Wrong Virtual Device");
 for(const invalid of [{...wrong,readAuthorized:true,activeConnections:1,connection:"connected_read_only"},{...wrong,device:fixture.device},{...wrong,device:{...wrong.device,model:"comFi MG11"}}]){
  vi.mocked(invoke).mockResolvedValueOnce(invalid);await expect(simulationStatus()).rejects.toThrow();
 }
});
