// Compare every supplied synthetic case with the native oracle in each distribution.
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import cjs from "../npm/index.cjs";
import * as esm from "../npm/index.mjs";

const [inputsFile, referenceFile, receiptFile, webFile, workerBgFile] = process.argv.slice(2);
if (!receiptFile) throw new Error("Usage: node scripts/check-profile-distributions.mjs inputs.json native-results.json receipt.json [web-module.js] [worker-bg.js]");
const cases = JSON.parse(readFileSync(inputsFile, "utf8")).cases;
const expectedVersion = JSON.parse(readFileSync(new URL("../npm/package.json", import.meta.url), "utf8")).version;
const reference = new Map(JSON.parse(readFileSync(referenceFile, "utf8")).cases.map(c => [`${c.group}/${c.id}`, c.reports.new]));
const normalize = result => ({ valid: result.valid, issues: result.issues.map(i => JSON.stringify([i.severity, i.id, i.path ?? null])).sort() });
const versions = {};
const distributions = [
  ["npm-cjs", c => c.direction === "request" ? cjs.validateProfile(c.input, c.profile, c.version) : c.direction === "response" ? cjs.validateResponseProfile(c.input, c.profile, c.version) : cjs.validateResponseAgainstRequestProfile(c.input, c.request, c.profile, c.version)],
  ["npm-esm", c => c.direction === "request" ? esm.validateProfile(c.input, c.profile, c.version) : c.direction === "response" ? esm.validateResponseProfile(c.input, c.profile, c.version) : esm.validateResponseAgainstRequestProfile(c.input, c.request, c.profile, c.version)],
];
versions["npm-cjs"] = cjs.coreVersion();
versions["npm-esm"] = esm.coreVersion();
function addWasm(name, api) {
  versions[name] = api.core_version();
  distributions.push([name, c => c.direction === "request" ? api.validate_profile(c.version, "spec-json", c.profile, c.input) : c.direction === "response" ? api.validate_response_profile(c.version, "spec-json", c.profile, c.input) : api.validate_response_against_request_profile(c.version, "spec-json", c.profile, c.request, c.input)]);
}
if (webFile) {
  const web = await import(pathToFileURL(resolve(webFile)).href);
  await web.default({ module_or_path: readFileSync(resolve(webFile, "../rtblint_wasm_bg.wasm")) });
  addWasm("browser-wasm", web);
}
if (workerBgFile) {
  const bg = await import(pathToFileURL(resolve(workerBgFile)).href);
  const module = new WebAssembly.Module(readFileSync(resolve(workerBgFile, "../rtblint_wasm_bg.wasm")));
  const instance = new WebAssembly.Instance(module, { "./rtblint_wasm_bg.js": { ...bg } });
  bg.__wbg_set_wasm(instance.exports);
  instance.exports.__wbindgen_start();
  addWasm("worker-wasm", bg);
}
const failures = [];
for (const [name, version] of Object.entries(versions)) {
  if (version !== expectedVersion) failures.push({ distribution: name, case: "candidate-core-version" });
}
for (const c of cases) {
  const id = `${c.group}/${c.id}`;
  if (!reference.has(id)) throw new Error(`Missing native reference ${id}`);
  const expected = JSON.stringify(normalize(reference.get(id)));
  for (const [name, run] of distributions) {
    const actual = JSON.stringify(normalize(run(c)));
    if (actual !== expected) failures.push({ distribution: name, case: id });
  }
}
// The dialect argument and paired profile entry point must reach the bindings.
for (const api of [cjs, esm]) {
  const request = JSON.stringify({ id: "r", imp: [{ id: "i", secure: true, banner: {} }] });
  if (!api.validateProfile(request, "spec", "2.6-202606", "proto-json").valid) failures.push({ distribution: "npm", case: "profile-proto-json-argument" });
  if (api.validateProfile(request, "spec", "2.6-202606").valid) failures.push({ distribution: "npm", case: "profile-default-spec-json-argument" });
  let rejected = false;
  try { api.validateProfile(request, "unknown-destination"); } catch { rejected = true; }
  if (!rejected) failures.push({ distribution: "npm", case: "unknown-profile-rejection" });
}
const receipt = { cases: cases.length, distributions: distributions.map(([name]) => name), versions, comparisons: cases.length * distributions.length, failures };
writeFileSync(receiptFile, JSON.stringify(receipt, null, 2) + "\n");
if (failures.length) throw new Error(`${failures.length} distribution mismatches; see ${receiptFile}`);
console.log(JSON.stringify(receipt));
