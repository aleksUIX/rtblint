"""Rebuild independent OpenWrap fixtures from pinned vendor wire declarations."""
from pathlib import Path
import copy
import json

ROOT = Path(__file__).resolve().parents[2]
PIN = '24ff50ca1f00ca1e80ab80bcfa1c812c35f87797'
SOURCE = f'https://github.com/PubMatic-OpenWrap/prebid-server/blob/{PIN}/'
cases = []
inventory = []

def obj(**fields): return ('object', fields)
def arr(item): return ('array', item)
def mapping(item): return ('map', item)
S, I, B, N, I8, I32, A = 'string', 'integer', 'boolean', 'number', 'int8', 'int32', 'opaque'
adpod = obj(minads=I, maxads=I, adminduration=I, admaxduration=I, excladv=I, excliabcat=I)
request = obj(
    wrapper=obj(profileid=I, versionid=I, ssauction=I, sumry_disable=I,
        clientconfig=I, supportdeals=B, includebrandcategory=I, abtest=I,
        wiid=S, ssai=S, kv=mapping(A), video=obj(adrule=B),
        sdksubintegration=I, edsstatus=I),
    bidder=mapping(mapping(A)),
    adpod=obj(**adpod[1], crosspodexcladv=I, crosspodexcliabcat=I,
        excliabcatwindow=I, excladvwindow=I),
    prebid=obj(transparency=obj(content=mapping(obj(include=B, keys=arr(S)))),
        keyval=mapping(A), tracker_disabled=B, googlessufeature=B, debug_override=B))
imp = obj(wrapper=obj(adserverurl=S, div=S), reward=I8,
    bidder=mapping(obj(keywords=arr(obj(key=S, value=arr(S))),
        dealtier=obj(prefix=S, minDealTier=I))),
    data=obj(), gpid=S, prebid=obj(), owsdk=mapping(A),
    billing_id=arr(S), publisher_setting_list_id=arr(S), allowed_vendor_type=arr(I),
    excluded_creatives=arr(obj(buyer_creative_id=S)), is_app_open_ad=I8,
    allowed_restricted_category=arr(I), creative_enforcement_settings=obj(
        policy_enforcement=I, scan_enforcement=I, publisher_blocks_enforcement=I),
    dfp_ad_unit_code=S)
video = obj(offset=I, adpod=adpod)
banner = obj(flexslot=obj(wmin=I32, wmax=I32, hmin=I32, hmax=I32))
regs = obj(gdpr=I, us_privacy=S)
source = obj(omidpv=S, omidpn=S)
user = obj(consent=S)
bid = obj(errorCode=I, errorMessage=S, refreshInterval=I, crtype=S,
    summary=arr(obj(vastTagID=S, bidder=S, bid=N, errorCode=I,
        errorMessage=S, width=I, height=I, regex=S)),
    video=obj(minduration=I, maxduration=I, skip=I8, skipmin=I, skipafter=I,
        battr=arr(I), playbackmethod=arr(I)), banner=obj(), dspid=I, winner=I,
    netecpm=N, origbidcpm=N, origbidcur=S, origbidcpmusd=N, fsc=I,
    adpod=obj(isAdpodBid=B, targeting=mapping(S),
        debug=obj(Targeting=mapping(S), targeting=mapping(S)), aprc=I, refbids=arr(S)),
    ibv=B, clicktrackers=arr(S), owsdk=mapping(A), act=I, bidexp_enf=I)

REQ = {'id':'req', 'imp':[{'id':'1','banner':{'w':320,'h':50}}]}
CTV = {'id':'req', 'imp':[{'id':'1','video':{'mimes':['video/mp4']}}]}
RESP = {'id':'req','seatbid':[{'bid':[{'id':'bid','impid':'1','price':1.2,
    'mtype':1,'adm':'<div>Advertisement</div>'}]}]}

def display(path):
    text = ''
    for key in path:
        text += f'[{key}]' if isinstance(key,int) else ('.' if text else '') + key
    return text
def setv(data,path,value):
    cur = data
    for key in path[:-1]:
        cur = cur[key] if isinstance(key,int) else cur.setdefault(key,{})
    cur[path[-1]] = copy.deepcopy(value)
def example(node):
    if isinstance(node,str):
        return {'string':'example','integer':1,'boolean':True,'number':1.2,
                'int8':1,'int32':1,'opaque':{'unknown':[True,7,'value']}}[node]
    kind, child = node
    if kind == 'object': return {key:example(value) for key,value in child.items()}
    if kind == 'array': return [example(child)]
    return {'vendor':example(child)}
def walk(node,path):
    yield node,path
    if isinstance(node,str): return
    kind,child=node
    if kind=='object':
        for key,value in child.items(): yield from walk(value,path+[key])
    else: yield from walk(child,path+([0] if kind=='array' else ['vendor']))
def bad(node):
    if isinstance(node,str): return 7 if node=='string' else 'wrong' if node=='boolean' else True
    return [] if node[0] in ('map','object') else {}
def expected(id,path,warning=False):
    return [{'id':id,'path':display(path),'severity':'warning' if warning else 'error'}]
def add(id,profile,direction,payload,valid=True,errors=None,request=None):
    case={'id':id,'profile':profile,'direction':direction,'input':copy.deepcopy(payload),
          'valid':valid,'expected':errors or []}
    if request is not None:case['request']=copy.deepcopy(request)
    cases.append(case)

# Each declaration has an independent positive value, wrong type and Go-null control.
groups = [
    ('request',request,REQ,['ext'],'modules/pubmatic/openwrap/models/request.go'),
    ('request',imp,REQ,['imp',0,'ext'],'modules/pubmatic/openwrap/models/request.go'),
    ('request',video,CTV,['imp',0,'video','ext'],'modules/pubmatic/openwrap/models/request.go'),
    ('request',banner,REQ,['imp',0,'banner','ext'],'openrtb_ext/pubmatic_ow.go'),
    ('request',regs,REQ,['regs','ext'],'modules/pubmatic/openwrap/models/request.go'),
    ('request',source,REQ,['source','ext'],'modules/pubmatic/openwrap/models/source.go'),
    ('request',user,REQ,['user','ext'],'openrtb_ext/user.go'),
    ('response',bid,RESP,['seatbid',0,'bid',0,'ext'],'modules/pubmatic/openwrap/models/response.go'),
]
for direction,schema,baseline,prefix,source_path in groups:
    full=copy.deepcopy(baseline)
    setv(full,prefix,example(schema))
    for node,path in walk(schema,prefix):
        kind=node if isinstance(node,str) else node[0]
        field_source=source_path
        if path[:2]==['ext','prebid']:
            field_source='openrtb_ext/request.go' if path[:4]==['ext','prebid','transparency','content'] else 'openrtb_ext/openwrap.go'
        if path[:3]==['imp',0,'ext'] and len(path)>3:
            if path[3] in ['billing_id','publisher_setting_list_id','allowed_vendor_type',
                'excluded_creatives','is_app_open_ad','allowed_restricted_category',
                'creative_enforcement_settings','dfp_ad_unit_code']:
                field_source='openrtb_ext/pubmatic_ow.go'
            if 'dealtier' in path:field_source='openrtb_ext/deal_tier.go'
        if path[:6]==['seatbid',0,'bid',0,'ext','adpod'] and len(path)>6 and path[6] in ['aprc','refbids']:
            field_source='openrtb_ext/openwrap.go'
        inventory.append({'direction':direction,'path':display(path),'wire_type':kind,
            'source':SOURCE+field_source,'unknown_keys_open':kind=='object'})
        if kind=='opaque': continue
        for suffix,value,valid in [('published-shape',example(node),True),
                                   ('wrong-shape',bad(node),False),('go-null',None,True)]:
            payload=copy.deepcopy(full);setv(payload,path,value)
            add(direction+'-'+display(path)+'-'+suffix,'pubmatic-openwrap',direction,
                payload,valid,[] if valid else expected('openrtb.profile.field_type',path))
        if kind in (I,I8,I32):
            low,high = (-128,127) if kind==I8 else (-2147483648,2147483647) if kind==I32 else (-9223372036854775808,9223372036854775807)
            # skip has its separately documented binary flag restriction.
            if path[-1]=='skip':continue
            for value in [low,high,low-1,high+1]:
                payload=copy.deepcopy(full);setv(payload,path,value)
                valid=low<=value<=high
                add(direction+'-'+display(path)+'-width-'+str(value),'pubmatic-openwrap',direction,
                    payload,valid,[] if valid else expected('openrtb.profile.field_type',path))

add('raw-wire-no-normalized-pbs-bidders','pubmatic-openwrap','request',REQ)
add('ctv-no-visible-account-adpod-config','pubmatic-openwrap-ctv','request',CTV)
add('ctv-banner-only','pubmatic-openwrap-ctv','request',REQ,False,
    expected('openrtb.profile.openwrap.ctv_video_required',['imp']))
mixed=copy.deepcopy(CTV);mixed['imp'].append(copy.deepcopy(REQ['imp'][0]));mixed['imp'][1]['id']='2'
add('ctv-mixed-video-and-filtered-banner','pubmatic-openwrap-ctv','request',mixed)
add('ctv-response-published','pubmatic-openwrap-ctv','response',RESP)
add('pair-generic-openrtb-oracle','pubmatic-openwrap','pair',RESP,request=REQ)
add('spec-custom-types-unconstrained','spec','request',dict(REQ,ext={'wrapper':{'supportdeals':1}}))
for field,values in [('minads',[-1,0,1,3,4]),('maxads',[-1,0,1,3]),
                     ('adminduration',[-1,0,1,30,31]),('admaxduration',[-1,0,1,30]),
                     ('excladv',[-1,0,100,101,None]),('excliabcat',[-1,0,100,101,None])]:
    for value in values:
        config={'adminduration':1,'admaxduration':30};config[field]=value
        payload=copy.deepcopy(CTV);setv(payload,['imp',0,'video','ext','adpod'],config)
        id=None
        if field in ('minads','maxads') and value<0:id='adpod_count'
        elif field=='minads' and value==4:id='adpod_count_order'
        elif field in ('adminduration','admaxduration') and value<=0:id='adpod_duration'
        elif field=='adminduration' and value==31:id='adpod_duration_order'
        elif field in ('excladv','excliabcat') and value is not None and not 0<=value<=100:id='adpod_exclusion'
        add('ctv-'+field+'-'+str(value),'pubmatic-openwrap-ctv','request',payload,id is None,
            [] if id is None else expected('openrtb.profile.openwrap.'+id,['imp',0,'video','ext','adpod',field]))
for name,config,valid in [
    ('explicit-empty',{},False),('null',None,True),
    ('executed-maxads-default3',{'minads':3,'adminduration':1,'admaxduration':30},True),
    ('equal-counts-and-durations',{'minads':2,'maxads':2,'adminduration':30,'admaxduration':30},True),
    ('malformed-count-does-not-infer-order',{'minads':'wrong','maxads':1,'adminduration':1,'admaxduration':30},False),
]:
    payload=copy.deepcopy(CTV);setv(payload,['imp',0,'video','ext','adpod'],config)
    errors=expected('openrtb.profile.openwrap.adpod_duration',['imp',0,'video','ext','adpod','adminduration']) if config=={} else expected('openrtb.profile.field_type',['imp',0,'video','ext','adpod','minads']) if name.startswith('malformed') else []
    add('ctv-adpod-'+name,'pubmatic-openwrap-ctv','request',payload,valid,errors)
# The general wire profile does not invent CTV requiredness or run the unused old validator.
payload=copy.deepcopy(CTV);setv(payload,['imp',0,'video','ext','adpod'],{})
add('general-wire-explicit-adpod-empty-no-ctv-context','pubmatic-openwrap','request',payload)
payload=copy.deepcopy(CTV);setv(payload,['ext','adpod'],{'adminduration':0,'admaxduration':0})
add('ctv-global-adpod-not-selected-by-current-resolver','pubmatic-openwrap-ctv','request',payload)
for value in [0,1,2,-1,128]:
    payload=copy.deepcopy(RESP);setv(payload,['seatbid',0,'bid',0,'ext','video','skip'],value)
    add('bid-video-skip-'+str(value),'pubmatic-openwrap','response',payload,value in (0,1),
        [] if value in (0,1) else expected('openrtb.profile.value_invalid',['seatbid',0,'bid',0,'ext','video','skip']))
for key,value in [('prebid',{'bidder':{}}),('unknown_future',{'anything':[True,2]}),
                  ('skadn',{'version':'2.0','network':'network.skadnetwork','campaign':'1',
                      'itunesitem':'456','sourceapp':'123','nonce':'nonce','timestamp':'1',
                      'signature':'opaque','newversion':True}),
                  ('adpod',{'debug':{'Targeting':None}})]:
    payload=copy.deepcopy(RESP);setv(payload,['seatbid',0,'bid',0,'ext',key],value)
    add('response-open-imported-or-opaque-'+key,'pubmatic-openwrap','response',payload)
schain={'ver':'1.0','complete':1,'nodes':[{'asi':'exchange.example','sid':'seller','hp':1}]}
for name,changes,warning in [('accepted',{},False),('nonpayment-node',{'hp':0},True),
    ('missing-hp',{'hp':None},True),('sid-64-codepoints',{'sid':'é'*64},False),
    ('sid-65-codepoints',{'sid':'é'*65},True)]:
    chain=copy.deepcopy(schain);chain['nodes'][0].update(changes)
    payload=copy.deepcopy(CTV);payload['source']={'schain':chain}
    add('ctv-schain-'+name,'pubmatic-openwrap-ctv','request',payload,name!='missing-hp',
        expected('openrtb.profile.openwrap.schain_removed',['source','schain'],True) if warning else [])

out=ROOT/'fixtures/exchange-depth/pubmatic-openwrap/cases.json'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(json.dumps({'source_commit':PIN,'cases':cases},indent=2,ensure_ascii=False)+'\n')
(ROOT/'docs/exchange-profiles/pubmatic-openwrap-field-inventory.json').write_text(
    json.dumps({'source_commit':PIN,'custom_wire_paths':inventory},indent=2)+'\n')
print(json.dumps({'cases':len(cases),'positive':sum(c['valid'] for c in cases),'typed_paths':len(inventory)}))
