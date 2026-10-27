import { expect, it } from "vitest";
import { snapshotSchema } from "../types/contracts";
it("accepts the explicit isolated workflow provenance and rejects arbitrary provenance",()=>{
  const snapshot={version:"test",databaseVersion:6,dataDirectory:"isolated",settings:{theme:"system"},counts:{devices:1,artifacts:0,runningJobs:0},recentEvents:[],testProfile:{runId:"a".repeat(32),discoverySource:"synthetic_workflow_fixture",simulatedAdapters:false}};
  expect(snapshotSchema.parse(snapshot).testProfile?.discoverySource).toBe("synthetic_workflow_fixture");
  expect(snapshotSchema.safeParse({...snapshot,testProfile:{...snapshot.testProfile,discoverySource:"verified_physical_router"}}).success).toBe(false);
});
