#!/usr/bin/env python3
"""Build profile reference pages from source rules and independently asserted fixtures."""
from pathlib import Path
import argparse
import hashlib
import json
import re

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--infra-dir', type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
infra = args.infra_dir.resolve()
base_file = infra / 'apps/rtblint-web/lib/findings.ts'
base = base_file.read_text()
existing = set(re.findall(r'id: "([^"]+)"', base))
sources = list((root / 'crates/rtblint-core/src').glob('*.rs')) + list((root / 'crates/rtblint-core/src/profile').glob('*.rs'))
messages = {}
severity = {}


def record_message(identifier, content, end, warning=False):
    tail = content[end:end + 900]
    words = re.search(r'"((?:\\.|[^"\\])*)"', tail)
    if not words:
        raise RuntimeError('Missing source message: ' + identifier)
    message = words.group(1).replace('\\\n', ' ').replace('\\n', ' ')
    message = re.sub(r'\{[^}]+\}', 'the field', message)
    message = ' '.join(message.split()).replace('\u2014', '.').replace('\u2013', 'through')
    if not message.endswith('.'):
        message += '.'
    messages.setdefault(identifier, message)
    prefix = content[max(0, end - 160):end]
    if warning or re.search(r'\w*warning\s*\(\s*"[^"\\]*"\s*$', prefix) or 'severity: Severity::Warning' in tail[:160]:
        severity[identifier] = 'warning'


for source in sources:
    content = source.read_text()
    for match in re.finditer(r'"(openrtb\.profile(?:\.[a-z0-9_]+)+)"', content):
        record_message(match.group(1), content, match.end())
    namespaces = set(re.findall(r'format!\(\s*"(openrtb\.profile(?:\.[a-z0-9_]+)+)\.\{suffix\}"', content))
    if len(namespaces) > 1:
        raise RuntimeError('Ambiguous suffix helper namespaces: ' + str(source))
    if namespaces:
        namespace = next(iter(namespaces))
        for match in re.finditer(r'(?<![\w.:])(error|warning)\s*\(\s*"([a-z0-9_]+)"', content):
            record_message(namespace + '.' + match.group(2), content, match.end(), match.group(1) == 'warning')

metadata = (root / 'crates/rtblint-core/src/profile.rs').read_text()
variant_ids = dict(re.findall(r'Self::(\w+) => "([^"]+)"', metadata.split('pub fn as_str')[1].split('pub fn display_name')[0]))
source_urls = {variant_ids[variant]: url for variant, url in re.findall(r'Self::(\w+) => Some\("([^"]+)"\)', metadata.split('pub fn source_url')[1].split('pub fn from_id')[0])}
examples = {}
raw_response_cases = []
for directory in [root / 'fixtures/exchange-depth', root / 'crates/rtblint-core/tests/fixtures/exchange-depth']:
    for file in directory.rglob('*'):
        if file.suffix not in ['.json', '.jsonl']:
            continue
        data = [json.loads(line) for line in file.read_text().splitlines()] if file.suffix == '.jsonl' else json.loads(file.read_text())
        cases = data if isinstance(data, list) else data.get('cases', []) if isinstance(data, dict) else []
        for case in cases:
            if not isinstance(case, dict):
                continue
            profile = case.get('profile')
            direction = case.get('direction', case.get('mode'))
            payload = case.get('input', case.get('payload'))
            raw_payload = None
            if payload is None and case.get('file'):
                raw_payload = (file.parent / case['file']).read_bytes().decode('utf-8')
                payload = json.loads(raw_payload)
            request = case.get('request')
            raw_request = None
            if request is None and case.get('request_file'):
                raw_request = (file.parent / case['request_file']).read_bytes().decode('utf-8')
                request = json.loads(raw_request)
            if case.get('id') in ['alx-response-bytes-4096', 'alx-response-bytes-4097', 'alx-response-utf8-bytes']:
                assert raw_payload is not None
                raw_response_cases.append({'id':case['id'], 'profile':profile, 'version':case.get('version','2.6-202606'), 'input':raw_payload, 'expectedValid':case['expected_valid'], 'expectedSizeError':any(finding['id']=='openrtb.profile.applovin.response_size' for finding in case['expected_profile_findings'])})
            for key in ['expected', 'expected_profile_findings', 'expected_profile_errors', 'expected_profile_warnings', 'required_issues']:
                for finding in case.get(key, []):
                    if not isinstance(finding, dict):
                        continue
                    id_ = finding.get('id')
                    if id_ not in messages or id_ in existing or payload is None:
                        continue
                    printed_payload = raw_payload if id_ == 'openrtb.profile.applovin.response_size' and raw_payload is not None else json.dumps(payload, indent=2, ensure_ascii=False)
                    sample = ('Request:\n' + json.dumps(request, indent=2, ensure_ascii=False) + '\n\nResponse:\n' if direction == 'pair' else '') + printed_payload
                    candidate = {'id':id_, 'profile':profile, 'direction':direction, 'version':case.get('version','2.6-202606'), 'payload':payload, 'request':request, 'rawPayload':raw_payload, 'rawRequest':raw_request, 'text':sample, 'fixture':str(file.relative_to(root)), 'case':case.get('id',case.get('name')), 'path':finding.get('path'), 'matchPath':'path' in finding, 'source':case.get('source'), 'severity':finding.get('severity', 'warning' if key=='expected_profile_warnings' else severity.get(id_,'error'))}
                    if id_ not in examples or len(sample) < len(examples[id_]['text']):
                        examples[id_] = candidate
missing = set(messages) - existing - set(examples)
if missing:
    raise RuntimeError('Diagnostics need independently asserted source fixtures: ' + ', '.join(sorted(missing)))
assert len(raw_response_cases) == 3, 'Retain all raw response byte-boundary controls'

entries = []
index = []
for id_ in sorted(set(messages) - existing):
    example = examples[id_]
    message = messages[id_]
    source = source_urls.get(example['profile'])
    response_sources = {
        'xandr': 'https://learn.microsoft.com/en-us/xandr/bidders/incoming-bid-response-from-bidders',
        'bidswitch': 'https://protocol.bidswitch.com/standards-v57/response-bid-obj.html',
        'bidswitch-supplier': 'https://protocol.bidswitch.com/ssp-protocol-v11/ssp-response-bid-object.html',
        'equativ': 'https://help.equativ.com/open-rtb-api-integration-bid-response-specification',
        'equativ-supplier': 'https://help.equativ.com/open-rtb-api-integration-bid-response-specification',
        'index-exchange': 'https://kb.indexexchange.com/dsps/open-rtb/list_of_supported_openrtb_bid_response_fields_dsp.htm',
        'inmobi': 'https://support.inmobi.com/advertise/integration/ortb-specs/bid-responses-dsp',
        'mobilefuse': 'https://docs.mobilefuse.com/docs/bid-responses',
        'mobilefuse-sdk': 'https://docs.mobilefuse.com/docs/bid-responses',
    }
    if example['direction'] in ['response', 'pair']:
        source = response_sources.get(example['profile'], source)
    if isinstance(example['source'], str) and example['source'].startswith('https://'):
        source = example['source']
    if id_ == 'openrtb.profile.bidswitch.click_macro':
        source = 'https://protocol.bidswitch.com/ssp-protocol-v11/ssp-macros.html' if example['profile'] == 'bidswitch-supplier' else 'https://protocol.bidswitch.com/standards/macros.html'
    scope = {'request':'Bid request','response':'Bid response','pair':'Request/response pair'}[example['direction']]
    fix = 'Use the documented field type, value and representation at the reported path for this integration direction.'
    if any(term in id_ for term in ['not_offered','unoffered','mismatch','unoffered','slot_not_offered','banner_size']):
        fix = 'Use an option offered by the matching request. Check the request and response together at the reported path.'
    elif any(term in id_ for term in ['length','limit','bid_count','count','size','duration']):
        fix = 'Keep the value within the published boundary described above. Check Unicode character counts where the contract counts characters.'
    elif any(term in id_ for term in ['required','inventory','placement']):
        fix = 'Supply the missing information through the documented field or permitted alternative for this direction.'
    elif any(term in id_ for term in ['macro']):
        fix = 'Use the documented macro spelling and placement for this endpoint. Remove extra occurrences or unsupported contexts.'
    elif example['severity'] == 'warning':
        fix = 'Review the documented processing behavior or source ambiguity before changing the payload. Confirm account-dependent exceptions with the destination.'
    if id_ == 'openrtb.profile.scope.unsupported':
        fix = 'Choose a profile with a verified contract for this payload direction and OpenRTB family. Specification validation remains available.'
    path_note = ' at ' + str(example['path']) if example['path'] not in [None,''] else ''
    note = f"Profile {example['profile']}, OpenRTB {example['version']}. The fixture asserts {id_}{path_note}."
    if id_ == 'openrtb.profile.applovin.response_size' and example['rawPayload'] is not None:
        note += f" The original response is {len(example['rawPayload'].encode('utf-8'))} UTF-8 bytes. Its exact bytes are preserved for replay."
    entry = {'id':id_, 'severity':example['severity'], 'scope':scope, 'headline':message, 'meaning':message, 'what':message + ' This check applies to the selected vendor contract and integration direction.', 'why':'The declared profile checks the published partner contract alongside canonical OpenRTB. It does not verify runtime approval or account configuration.', 'fix':fix, 'example':example['text'], 'exampleNote':note, 'related':[{'href':'https://github.com/aleksUIX/rtblint/blob/main/' + example['fixture'],'label':'Replayable source fixture'}]}
    if source: entry['related'].insert(0, {'href':source,'label':'Primary protocol documentation'})
    entries.append(entry)
    index.append({key:example[key] for key in ['id','profile','direction','version','fixture','case','path','matchPath','severity']})
    index[-1]['fixture_sha256'] = hashlib.sha256((root / example['fixture']).read_bytes()).hexdigest()
    if example['rawPayload'] is not None:
        index[-1]['raw_payload_sha256'] = hashlib.sha256(example['rawPayload'].encode('utf-8')).hexdigest()
        index[-1]['raw_payload_bytes'] = len(example['rawPayload'].encode('utf-8'))

output = infra / 'apps/rtblint-web/lib/profile-findings.ts'
output.write_text('// Generated by rtblint scripts/sync-profile-findings.py.\n// Every example has an independently asserted fixture in the source corpus.\nimport type { Finding } from "./findings";\n\nexport const PROFILE_FINDINGS: Finding[] = ' + json.dumps(entries,indent=2,ensure_ascii=False) + ';\n\n' +
    'export interface ProfileReferenceCase {\n  id: string;\n  profile: string;\n  direction: "request" | "response" | "pair";\n  version: string;\n  payload: unknown;\n  request?: unknown;\n  rawPayload?: string | null;\n  rawRequest?: string | null;\n  path?: string | null;\n  matchPath: boolean;\n  severity: "error" | "warning";\n}\n\n' +
    'export const PROFILE_REFERENCE_CASES: ProfileReferenceCase[] = ' + json.dumps([{key:examples[id_][key] for key in ['id','profile','direction','version','payload','request','rawPayload','rawRequest','path','matchPath','severity']} for id_ in sorted(set(messages)-existing)],indent=2,ensure_ascii=False) + ';\n\n' +
    'export interface RawResponseReferenceCase {\n  id: string;\n  profile: string;\n  version: string;\n  input: string;\n  expectedValid: boolean;\n  expectedSizeError: boolean;\n}\n\n' +
    'export const RAW_RESPONSE_REFERENCE_CASES: RawResponseReferenceCase[] = ' + json.dumps(raw_response_cases,indent=2,ensure_ascii=False) + ';\n')
if 'import { PROFILE_FINDINGS }' not in base:
    base = 'import { PROFILE_FINDINGS } from "./profile-findings";\n\n' + base
if '  ...PROFILE_FINDINGS,' not in base:
    position = base.rindex('\n];')
    base = base[:position] + '\n  ...PROFILE_FINDINGS,' + base[position:]
base_file.write_text(base)
(root / 'docs/exchange-profiles/rule-reference-fixture-index.json').write_text(json.dumps({'approach':'Diagnostic descriptions come from Rust source. Example captures, expected IDs and paths come from independently asserted source fixtures. No oracle expectations are learned from validator output.','count':len(index),'examples':index},indent=2) + '\n')
print(f'{len(entries)} profile references synchronized with asserted fixture examples.')
