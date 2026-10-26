import {expect,it} from "vitest";
import {artifactSchema} from "../types/library";
import {artifact} from "./libraryFixtures";
it("retains explicit content class without inferring from filename or source",()=>{
  expect(artifactSchema.parse({...artifact,artifactClass:"user_imported_unverified"}).artifactClass).toBe("user_imported_unverified");
  expect(artifactSchema.parse({...artifact,originalFilename:"official-device-firmware.bin"}).artifactClass).toBeUndefined();
  expect(artifactSchema.safeParse({...artifact,artifactClass:"verified_firmware"}).success).toBe(false);
});
