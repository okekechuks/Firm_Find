// Generate the review schema from the same frontend contract used for native IPC.
// Rust additionally enforces provenance, references, bounds and semantic constraints.
import { readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import ts from 'typescript';
import assert from 'node:assert/strict';

const require = createRequire(import.meta.url);
const source = await readFile(new URL('../apps/desktop/src/types/profiles.ts', import.meta.url), 'utf8');
const js = ts.transpileModule(source, {compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText
  .replace('from "zod"', `from ${JSON.stringify(pathToFileURL(require.resolve('zod')).href)}`);
const { profileSchema } = await import('data:text/javascript;base64,' + Buffer.from(js).toString('base64'));
const { z } = require('zod');
// Only unwrap this known refinement. Rust remains the semantic validation authority.
assert.equal(profileSchema._def.typeName, 'ZodEffects');
assert.equal(profileSchema._def.effect.type, 'refinement');
const structural = profileSchema._def.schema;
assert.equal(structural._def.typeName, 'ZodObject');
// The locked application uses Zod 3. Convert only its contract primitives;
// throw for any new kind so a contract change cannot silently lose a field.
function convert(type) {
  const d = type._def;
  switch (d.typeName) {
    case 'ZodString': return {type:'string',maxLength:1024};
    case 'ZodBoolean': return {type:'boolean'};
    case 'ZodNumber': return {type:d.checks.some((c) => c.kind === 'int') ? 'integer' : 'number',...Object.fromEntries(d.checks.filter((c) => c.kind === 'min').map((c) => ['minimum',c.value]))};
    case 'ZodLiteral': return {const:d.value};
    case 'ZodEnum': return {type:'string',enum:d.values};
    case 'ZodArray': return {type:'array',maxItems:64,items:convert(d.type)};
    case 'ZodNullable': return {anyOf:[convert(d.innerType),{type:'null'}]};
    case 'ZodOptional': return convert(d.innerType);
    case 'ZodRecord': return {type:'object',maxProperties:64,additionalProperties:convert(d.valueType)};
    case 'ZodObject': {
      const fields=d.shape();
      return {type:'object',properties:Object.fromEntries(Object.entries(fields).map(([key,value]) => [key,convert(value)])),required:Object.keys(fields).filter((key) => !fields[key].isOptional()),additionalProperties:false};
    }
    default: throw new Error(`Unsupported contract kind: ${d.typeName}`);
  }
}
const schema = {$schema:'https://json-schema.org/draft/2020-12/schema',...convert(structural.omit({inspectionOnly:true}).extend({schemaVersion:z.literal(1)}))};
schema.$id = 'urn:obtainer:device-profile:1';
schema.title = 'OBTAINER device profile schema 1';
schema.description = 'Structural schema. Rust validation also enforces 64 KiB, depth 16, collection 64, strings 1024 bytes, unique keys, exact sub-capability keys, referenced evidence, calendar dates, explicit firmware schemes and application-owned adapter state. Firmware scheme defaults to legacy numeric_triplet; opaque_exact permits only literal exact versions without ranges. Validation never grants trust.';
await writeFile(new URL('../device-profiles/schema-v1.json', import.meta.url), JSON.stringify(schema, null, 2) + '\n');
console.log('Generated device-profiles/schema-v1.json');
const inspection = {$schema:schema.$schema,...convert(structural.extend({schemaVersion:z.literal(2),inspectionOnly:z.literal(true)}))};
inspection.$id = 'urn:obtainer:device-profile:2';
inspection.title = 'OBTAINER inspection-only OS profile schema 2';
inspection.description = schema.description + ' Schema 2 requires inspectionOnly=true and UNKNOWN hardware feature assessments. Rust additionally enforces an exact OpenWrt release, compound board/target/build identity, absent manufacturer/revision claims and no verified feature ranges. This schema does not authorize a connection or activate a profile.';
for (const feature of Object.values(inspection.properties.capabilities.properties)) {
  feature.properties.assessment = {const:'UNKNOWN'};
  feature.properties.subCapabilities.additionalProperties.properties.assessment = {const:'UNKNOWN'};
  feature.properties.verifiedRange = {type:'null'};
}
inspection.properties.manufacturer = {const:''};
inspection.properties.hardwareRevisions.maxItems = 0;
inspection.properties.firmwareFamilies = {type:'array',items:{const:'OpenWrt'},minItems:1,maxItems:1};
const constraints=inspection.properties.firmwareVersionConstraints.properties;
constraints.exact.minItems=1; constraints.exact.maxItems=1;
constraints.minInclusive={type:'null'}; constraints.maxExclusive={type:'null'};
await writeFile(new URL('../device-profiles/schema-v2.json', import.meta.url), JSON.stringify(inspection,null,2)+'\n');
console.log('Generated device-profiles/schema-v2.json');
