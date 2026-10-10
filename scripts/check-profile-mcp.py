"""Replay the authored exchange corpus through the native MCP stdio transport."""
import collections
import json
import pathlib
import subprocess
import sys

inputs_file, reference_file, executable, receipt_file = sys.argv[1:]
package = pathlib.Path(__file__).resolve().parents[1] / "npm/package.json"
expected_version = json.loads(package.read_text())["version"]
cases = json.loads(pathlib.Path(inputs_file).read_text())["cases"]
references = {
    f"{case['group']}/{case['id']}": case["reports"]["new"]
    for case in json.loads(pathlib.Path(reference_file).read_text())["cases"]
}
messages = [
    {"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {"protocolVersion": "2024-11-05"}},
    {"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}},
]
for index, case in enumerate(cases, 2):
    arguments = {"payload": case["input"], "profile": case["profile"], "version": case["version"], "dialect": "spec-json"}
    if case["direction"] == "pair":
        arguments["bid_request"] = case["request"]
    messages.append({"jsonrpc": "2.0", "id": index, "method": "tools/call", "params": {
        "name": "validate_bid_request" if case["direction"] == "request" else "validate_bid_response",
        "arguments": arguments,
    }})
result = subprocess.run([executable], input="\n".join(json.dumps(m) for m in messages) + "\n", capture_output=True, text=True, check=True)
responses = [json.loads(line) for line in result.stdout.splitlines()]
assert len(responses) == len(messages), "Missing MCP responses"
assert responses[0]["result"]["serverInfo"]["version"] == expected_version
definitions = {tool["name"]: tool for tool in responses[1]["result"]["tools"]}
request_ids = definitions["validate_bid_request"]["inputSchema"]["properties"]["profile"]["enum"]
response_ids = definitions["validate_bid_response"]["inputSchema"]["properties"]["profile"]["enum"]
assert request_ids == response_ids and len(set(request_ids)) == 27
assert {c["profile"] for c in cases}.issubset(request_ids)

def normalize(report):
    return report["valid"], collections.Counter((i["severity"], i["id"], i.get("path")) for i in report["issues"])

failures = []
for index, (case, response) in enumerate(zip(cases, responses[2:]), 2):
    identity = f"{case['group']}/{case['id']}"
    assert response["id"] == index, "MCP response order changed"
    report = json.loads(response["result"]["content"][0]["text"])
    if normalize(report) != normalize(references[identity]):
        failures.append(identity)
receipt = {"core_version": expected_version, "profile_ids": request_ids, "cases": len(cases), "comparisons": len(cases), "failures": failures}
pathlib.Path(receipt_file).write_text(json.dumps(receipt, indent=2) + "\n")
print(json.dumps({"cases": len(cases), "profiles": len(request_ids), "failures": len(failures)}))
assert not failures, f"MCP mismatches: {len(failures)}"
