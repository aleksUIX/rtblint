import json,re,argparse
from pathlib import Path
parser=argparse.ArgumentParser(description='Generate typed Index descriptors from archived primary table extracts.')
parser.add_argument('source_directory',type=Path,help='Directory containing index-seller-tables.json, index-dsp-tables.json and index-response-tables.json')
parser.add_argument('--repo',type=Path,default=Path(__file__).resolve().parents[2])
args=parser.parse_args()
src=args.source_directory
out=args.repo/'crates/rtblint-core/src/profile/index_schema.rs'
objects={'BidRequest':'BidRequest','Source':'Source','Regs':'Regs','Imp':'Imp','Banner':'Banner','Video':'Video','Native':'Native','PMP':'Pmp','Deals':'Deal','Site':'Site','App':'App','Content':'Content','Device':'Device','User':'User','EID':'EID','UID':'UID','Data':'Data','Bid':'Bid','Response':'BidResponse'}
nested={'DSA object':'DSA','Transparency object':'TRANSPARENCY','SKAdNetwork extension object':'SKAD','SKAdNetwork ext object':'SKAD','Skadnetlist object':'SKADLIST','Placement object':'PLACEMENT','Fidelities object':'FIDELITY'}
notes=[];inventory=[];lines=['// Field descriptors transcribed from Index Exchange public tables reviewed 2026-10-09.','// Requiredness and documented contradictions are handled in index_exchange.rs.']
for label in ['index-seller','index-dsp','index-response']:
 tables=json.loads((src/(label+'-tables.json')).read_text()); schemas={};roots={}
 for t in tables:
  h=t['heading'];name=None
  for stem,obj in objects.items():
   if h.lower() in [(stem+' extension object').lower(),(stem+' ext object').lower()]:name=obj;roots[obj]=obj
  if h in nested:name=nested[h]
  if name:
   schemas[name]=t['rows'][1:]
 def kind(row,container):
  field,typ,*_=row;typ=typ.lower()
  if container=='Regs' and field=='us_privacy':return 'Kind::String'
  if field in ['dsa']:return 'Kind::Object('+prefix+'_DSA)'
  if field=='transparency':return 'Kind::Array(&Kind::Object('+prefix+'_TRANSPARENCY))'
  if field=='skadn':return 'Kind::Object('+prefix+'_SKAD)'
  if field=='skadnetlist' and 'SKADLIST' in schemas:return 'Kind::Object('+prefix+'_SKADLIST)'
  if field=='placement':return 'Kind::Object('+prefix+'_PLACEMENT)'
  if field=='fidelities':return 'Kind::Array(&Kind::Object('+prefix+'_FIDELITY))'
  if 'array' in typ:
   item='String' if 'string' in typ else 'Integer' if 'integer' in typ else 'Object(&[])'
   return 'Kind::Array(&Kind::'+item+')'
  if 'object' in typ:return 'Kind::Object(&[])'
  if 'boolean' in typ:return 'Kind::CompatibleFlag'
  if 'integer' in typ:return 'Kind::Integer'
  if any(x in typ for x in ['float','double']):return 'Kind::Number'
  if 'string' in typ:return 'Kind::String'
  return None
 prefix=label.replace('-','_').upper()
 for name,rows in schemas.items():
  lines.append(f'const {prefix}_{name.upper()}: &[Field] = &[')
  for row in rows:
   if len(row)<2:continue
   k=kind(row,name)
   if k:
    lines.append('    Field { name: '+json.dumps(row[0])+', kind: '+k+' },')
    inventory.append({'surface':label,'object':name,'field':row[0],'type':k})
   else:notes.append({'surface':label,'object':name,'field':row[0],'reason':'No unambiguous JSON type in source row'})
  lines.append('];')
 lines.append('fn '+label.replace('-','_')+'_schema(object: &str) -> &\'static [Field] { match object {')
 for name in roots:lines.append(f'    "{name}" => {prefix}_{name.upper()},')
 lines.append('    _ => &[],\n}}')
out.write_text('\n'.join(lines)+'\n')
(args.repo/'docs/exchange-profiles/index-field-inventory.json').write_text(json.dumps({'fields':inventory,'deferred':notes},indent=2)+'\n')
print('Index descriptors:',len(inventory),'deferred',len(notes))
