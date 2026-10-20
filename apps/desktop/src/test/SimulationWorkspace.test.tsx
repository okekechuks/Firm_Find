import {act,render,screen,within} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {beforeEach,expect,it,vi} from "vitest";
import {SimulationWorkspace} from "../features/simulation/SimulationWorkspace";
import {simulationControl,simulationRequest,simulationStatus} from "../lib/simulator";
import {changedFixture,inspectedFixture,simulationFixture,virtualDns} from "./simulatorFixtures";
import type {SimulationStatus} from "../types/simulator";
vi.mock("../lib/simulator",()=>({simulationControl:vi.fn(),simulationRequest:vi.fn(),simulationStatus:vi.fn()}));
beforeEach(()=>{vi.clearAllMocks();vi.mocked(simulationStatus).mockResolvedValue(simulationFixture());});
it("requires explicit activation and clearly labels a separate virtual workspace",async()=>{
 vi.mocked(simulationStatus).mockResolvedValue(simulationFixture({active:false,device:null,epoch:0,revision:0}));vi.mocked(simulationControl).mockResolvedValue(simulationFixture());
 render(<SimulationWorkspace/>);expect(await screen.findByText(/This is a virtual router/)).toBeInTheDocument();await userEvent.click(await screen.findByRole("button",{name:"Start simulation"}));
 expect(simulationControl).toHaveBeenCalledWith({kind:"start"});expect(await screen.findByLabelText("Virtual device")).toHaveValue("sim-router-001");expect(screen.queryByRole("button",{name:"Apply DNS to Simulator"})).not.toBeInTheDocument();
});
it("requires explicit handshake approval, connection and separate read authorization",async()=>{
 let s=simulationFixture();vi.mocked(simulationRequest).mockImplementation(async(_status,action)=>{
  if(action.kind==="approve_handshake")s={...s,approvalId:"sim-approval"};
  if(action.kind==="connect")s={...s,approvalId:null,connection:"connected_read_only",authentication:"authenticated",activeConnections:1};
  if(action.kind==="approve_reads")s={...s,readAuthorized:true};
  if(action.kind==="read_features")s=inspectedFixture();return s;
 });render(<SimulationWorkspace/>);
 const connect=await screen.findByRole("button",{name:"Connect virtual router"});expect(connect).toBeDisabled();
 await userEvent.click(screen.getByLabelText("Approve virtual endpoint handshake"));await userEvent.click(screen.getByRole("button",{name:"Approve virtual handshake"}));await userEvent.type(screen.getByLabelText("Simulation password"),"lab-only");await userEvent.click(connect);
 expect(await screen.findByText("Simulated Connected")).toBeInTheDocument();expect(screen.getByRole("button",{name:"Read all features"})).toBeDisabled();
 await userEvent.click(screen.getByRole("button",{name:"Authorize feature reads"}));await userEvent.click(screen.getByRole("button",{name:"Read all features"}));
 expect(await screen.findByText(/Current APN: lab.example/)).toBeInTheDocument();expect(screen.queryByLabelText("Simulation password")).not.toBeInTheDocument();
 expect(within(screen.getByRole("region",{name:"Simulated Wi-Fi"})).getByText(/3 associated clients/)).toBeInTheDocument();
 expect(simulationRequest).toHaveBeenCalledWith(expect.anything(),{kind:"connect",approvalId:"sim-approval",credential:"lab-only"});
});
it("previews real draft values and displays only backend apply/readback outcomes",async()=>{
 vi.mocked(simulationStatus).mockResolvedValue(inspectedFixture());let s=inspectedFixture();
 vi.mocked(simulationRequest).mockImplementation(async(_status,action)=>{
  if(action.kind==="preview_dns")s={...s,preview:{id:"sim-preview-1",target:"sim-router-001",epoch:1,baseRevision:1,previous:virtualDns,proposed:action.configuration,changes:[{field:"Upstream IPv4",before:["1.1.1.1","8.8.8.8"],after:["9.9.9.9"]}],affectedServices:["virtual-dns-forwarder"],snapshotRequired:true}};
  if(action.kind==="apply_dns")s=changedFixture();return s;
 });render(<SimulationWorkspace/>);
 await userEvent.selectOptions(await screen.findByLabelText("Upstream IPv4 mode"),"manual");const field=screen.getByLabelText("Upstream IPv4 resolvers");await userEvent.clear(field);await userEvent.type(field,"9.9.9.9");await userEvent.click(screen.getByRole("button",{name:"Preview DNS changes"}));
 expect(await screen.findByRole("region",{name:"DNS change preview"})).toHaveTextContent("9.9.9.9");expect(simulationRequest).toHaveBeenCalledWith(expect.anything(),expect.objectContaining({kind:"preview_dns",configuration:expect.objectContaining({upstreamIpv4:["9.9.9.9"],upstreamSourceIpv4:"manual"})}));
 await userEvent.click(screen.getByRole("button",{name:"Apply DNS to Simulator"}));expect(await screen.findByLabelText("Simulated DNS result")).toHaveTextContent("Simulated DNS verified");expect(screen.getByLabelText("Current simulated DNS")).toHaveTextContent("9.9.9.9");expect(simulationRequest).toHaveBeenLastCalledWith(expect.anything(),{kind:"read_features"});
});
it("shows truthful restore failure and reset requirement",async()=>{
 const s=changedFixture();s.outcome={...s.outcome!,operation:"restore",applied:false,verified:false,configurationMatches:false,rollback:"failed",requiresReset:true,message:"SIMULATION: restore failed."};vi.mocked(simulationStatus).mockResolvedValue(s);render(<SimulationWorkspace/>);
 expect(await screen.findByLabelText("Simulated DNS result")).toHaveTextContent("Simulated DNS not verified");expect(screen.getByText("Manual simulator reset required.")).toBeInTheDocument();expect(screen.queryByText("Simulated DNS verified")).not.toBeInTheDocument();
});
it("shows typed timeouts and refreshes the actual disconnected status",async()=>{
 vi.mocked(simulationStatus).mockResolvedValueOnce(inspectedFixture()).mockResolvedValue(simulationFixture());vi.mocked(simulationRequest).mockRejectedValue({code:"TIMEOUT",message:"Virtual read timed out.",retryable:true});render(<SimulationWorkspace/>);await userEvent.click(await screen.findByRole("button",{name:"Read all features"}));
 expect(await screen.findByRole("alert")).toHaveTextContent("TIMEOUT: Virtual read timed out.");expect(screen.queryByRole("button",{name:"Apply DNS to Simulator"})).not.toBeInTheDocument();
});
it("ignores pending results after navigation unmount and simulator re-entry",async()=>{
 let complete!:(s:SimulationStatus)=>void;vi.mocked(simulationStatus).mockResolvedValueOnce(inspectedFixture()).mockResolvedValue(simulationFixture({active:false,device:null}));vi.mocked(simulationRequest).mockReturnValue(new Promise(resolve=>{complete=resolve;}));
 const first=render(<SimulationWorkspace/>);await userEvent.click(await screen.findByRole("button",{name:"Read all features"}));first.unmount();render(<SimulationWorkspace/>);await screen.findByText("Simulation stopped");await act(async()=>complete(changedFixture()));expect(screen.queryByText("Simulated DNS verified")).not.toBeInTheDocument();
});
