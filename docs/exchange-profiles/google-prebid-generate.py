#!/usr/bin/env python3
"""Regenerate Google extension wire shapes from the reviewed proto downloads.

Requires protoc and the Python protobuf descriptor reader. These are generator
tools, not Rust build dependencies. Supply a directory containing the two
downloads named in google-prebid-sources.json. Refuse changed source bytes.
"""
import hashlib
import json
import pathlib
import subprocess
import sys
from google.protobuf import descriptor_pb2

repo = pathlib.Path(__file__).resolve().parents[2]
source_dir = pathlib.Path(sys.argv[1])
manifest = json.loads((pathlib.Path(__file__).with_name("google-prebid-sources.json")).read_text())
for source in manifest["sources"]:
    if source["file"] not in ("openrtb-proto.txt", "openrtb-adx-proto.txt"):
        continue
    raw = (source_dir / source["file"]).read_bytes()
    if hashlib.sha256(raw).hexdigest() != source["sha256"]:
        raise SystemExit("The downloaded Google source has changed: " + source["file"])
    (source_dir / source["file"].replace("-proto.txt", ".proto")).write_bytes(raw)
descriptor_path = source_dir / "google.pb"
subprocess.run(["protoc", "-I", str(source_dir), "--include_imports",
                "--descriptor_set_out=" + str(descriptor_path),
                str(source_dir / "openrtb-adx.proto")], check=True)
descriptors = descriptor_pb2.FileDescriptorSet()
descriptors.ParseFromString(descriptor_path.read_bytes())
messages, enums, roots = {}, {}, {}

def record_message(message, prefix):
    name = prefix + "." + message.name
    messages[name] = message
    for enum in message.enum_type:
        enums[name + "." + enum.name] = [v.number for v in enum.value]
    for child in message.nested_type:
        record_message(child, name)

for file in descriptors.file:
    package = "." + file.package
    for enum in file.enum_type:
        enums[package + "." + enum.name] = [v.number for v in enum.value]
    for message in file.message_type:
        record_message(message, package)
    for extension in file.extension:
        object_name = extension.extendee.rsplit(".", 1)[1]
        if object_name == "SeatBid":
            object_name = "Seatbid"
        roots[object_name] = extension.type_name

needed = set(roots.values())
queue = list(needed)
while queue:
    for field in messages[queue.pop()].field:
        if field.type == field.TYPE_MESSAGE and field.type_name not in needed:
            needed.add(field.type_name)
            queue.append(field.type_name)

primitive = {1: "Number", 2: "Number", 3: "Int64String", 4: "Uint64String",
             5: "Int32", 6: "Uint64String", 7: "Uint32", 8: "Flag", 9: "String",
             12: "Bytes", 13: "Uint32", 15: "Int32", 16: "Int64String",
             17: "Int32", 18: "Int64String"}
lines = ["// Generated from reviewed Google protocol v210 downloads.",
         "// Regenerate with docs/exchange-profiles/google-prebid-generate.py.",
         "// Source URLs and SHA-256 digests: google-prebid-sources.json.",
         "fn extension_schema(object_name: &str) -> Option<&'static [WireField]> {",
         "    match object_name {"]
for object_name, message in sorted(roots.items()):
    lines.append(f'        "{object_name}" => message_schema("{message}"),')
lines += ["        _ => None,", "    }", "}", "", "fn message_schema(name: &str) -> Option<&'static [WireField]> {", "    match name {"]
inventory = []
for name in sorted(needed):
    lines.append(f'        "{name}" => Some(&[')
    for field in messages[name].field:
        if field.type == field.TYPE_MESSAGE:
            kind = f'WireType::Message("{field.type_name}")'
        elif field.type == field.TYPE_ENUM:
            kind = "WireType::Enum(&[" + ", ".join(str(v) for v in enums[field.type_name]) + "])"
        else:
            kind = "WireType::" + primitive[field.type]
        repeated = str(field.label == field.LABEL_REPEATED).lower()
        group = f"Some({field.oneof_index})" if field.HasField("oneof_index") else "None"
        lines.append(f'            WireField {{ name: "{field.name}", kind: {kind}, repeated: {repeated}, oneof: {group} }},')
        inventory.append({"message": name, "field": field.name, "protobuf_type": field.type,
                          "type_name": field.type_name, "repeated": field.label == field.LABEL_REPEATED,
                          "oneof": field.oneof_index if field.HasField("oneof_index") else None})
    lines.append("        ]),")
lines += ["        _ => None,", "    }", "}", ""]
generated_path = repo / "crates/rtblint-core/src/profile/google_schema.rs"
generated_path.write_text("\n".join(lines))
subprocess.run(["rustfmt", "--edition", "2021", str(generated_path)], check=True)
(pathlib.Path(__file__).with_name("google-prebid-google-wire-inventory.json")).write_text(
    json.dumps({"protocol_version": "v210", "extension_roots": roots,
                "message_count": len(needed), "field_count": len(inventory),
                "fields": inventory}, indent=2) + "\n")
print(f"Generated {len(roots)} extension roots, {len(needed)} messages, {len(inventory)} field shapes.")
