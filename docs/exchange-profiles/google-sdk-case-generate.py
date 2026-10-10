"""Independent SDK context, conditional minima, mapping and token boundaries."""
from pathlib import Path
import copy
import json
import base64

ROOT=Path(__file__).resolve().parents[2]
FILE=ROOT/'fixtures/exchange-depth/google-prebid/cases.json'
SDK_SOURCE='https://developers.google.com/authorized-buyers/rtb/buyer-sdk-ads'
PROTO_SOURCE='https://developers.google.com/static/authorized-buyers/rtb/downloads/openrtb-adx-proto.txt'
NATIVE_SOURCE='https://developers.google.com/static/authorized-buyers/rtb/downloads/openrtb-proto.txt'
fixture=json.loads(FILE.read_text())
fixture['cases']=[case for case in fixture['cases'] if not case['id'].startswith('google-sdk-depth-')]

# Descriptor cases keep the tested field intact while receiving the independent
# surrounding SDK contract required by the September 2026 Buyer SDK guide.
for case in fixture['cases']:
    if case['profile']!='google-ab' or case['direction'] not in ['response','pair']:continue
    for seat in case['input'].get('seatbid',[]):
        for bid in seat.get('bid',[]):
            ext=bid.get('ext',{})
            sdk=ext.get('sdk_rendered_ad') if isinstance(ext,dict) else None
            if not isinstance(sdk,dict):continue
            bid.setdefault('adomain',['advertiser.example'])
            bid.setdefault('crid','sdk-creative')
            bid.setdefault('w',300);bid.setdefault('h',250)
            ext.setdefault('billing_id','123')
            sdk.setdefault('id','com.example.SDK');sdk.setdefault('rendering_data','opaque')
            sdk.setdefault('declared_ad',{'html_snippet':'<div>SDK representative ad</div>'})
            declared=sdk['declared_ad']
            if isinstance(declared,dict):
                if not any(key in declared for key in ['html_snippet','video_url','video_vast_xml','native_response']):
                    declared['html_snippet']='<div>SDK representative ad</div>'
                native=declared.get('native_response')
                if isinstance(native,dict) and isinstance(native.get('link'),dict):
                    native['link'].setdefault('url','https://advertiser.example')
                if isinstance(native,dict) and isinstance(native.get('assets'),list):
                    for asset in native['assets']:
                        if isinstance(asset,dict) and isinstance(asset.get('img'),dict):
                            asset['img'].setdefault('type',3)
                        if isinstance(asset,dict):
                            for container,field,value in [('title','text','SDK title'),('img','url','https://ads.example/image'),
                                ('video','vasttag','<VAST version="3.0"></VAST>'),('data','value','SDK data'),
                                ('link','url','https://advertiser.example')]:
                                if isinstance(asset.get(container),dict):asset[container].setdefault(field,value)
            case['surrounding_sdk_contract_source']=SDK_SOURCE

R={'id':'req-sdk','imp':[{'id':'1','banner':{'w':300,'h':250}}],
   'app':{'ext':{'installed_sdk':[{'id':'com.example.SDK'}]}}}
B={'id':'req-sdk','seatbid':[{'bid':[{'id':'bid','impid':'1','price':1.2,
    'adomain':['advertiser.example'],'crid':'sdk-creative','w':300,'h':250,
    'ext':{'billing_id':'123','sdk_rendered_ad':{'id':'com.example.SDK',
        'rendering_data':'opaque','declared_ad':{'html_snippet':'<div>SDK ad</div>'}}}}]}]}
BASE_PATH='seatbid[0].bid[0]'
def get_bid(payload):return payload['seatbid'][0]['bid'][0]
def sdk(payload):return get_bid(payload)['ext']['sdk_rendered_ad']
def exp(id,path,warning=False):return [{'id':id,'path':BASE_PATH+'.'+path,'severity':'warning' if warning else 'error'}]
def add(name,payload,valid=True,errors=None,request=None,source=SDK_SOURCE):
    case={'id':'google-sdk-depth-'+name,'profile':'google-ab',
        'direction':'pair' if request is not None else 'response','input':copy.deepcopy(payload),
        'valid':valid,'expected':errors or [],'source':source}
    if request is not None:case['request']=copy.deepcopy(request)
    fixture['cases'].append(case)

add('selected-minimum',B)
for field,value in [('adm','<div>Separate creative</div>'),('ext.amp_ad_url','https://ads.example/amp')]:
    payload=copy.deepcopy(B);bid=get_bid(payload)
    if field=='adm':bid[field]=value
    else:bid['ext']['amp_ad_url']=value
    add('conflicting-renderer-'+field,payload,False,
        exp('openrtb.profile.google.sdk_renderer_conflict','ext.sdk_rendered_ad'))
payload=copy.deepcopy(B);get_bid(payload)['adm']='';add('empty-adm-not-another-creative',payload)
for name,app in [('malformed-app','unknown'),('malformed-app-ext',{'ext':'unknown'}),
                 ('malformed-installed-list',{'ext':{'installed_sdk':{}}}),
                 ('partial-installed-item',{'ext':{'installed_sdk':[{'id':'another.SDK'},{}]}}),
                 ('empty-installed-id',{'ext':{'installed_sdk':[{'id':''}]}})]:
    req=copy.deepcopy(R);req['app']=app
    add('installed-sdk-'+name+'-defers',B,request=req)
for field in ['adomain','ext.billing_id','crid','w','h']:
    payload=copy.deepcopy(B);obj=get_bid(payload)
    parts=field.split('.')
    if len(parts)==2:obj=obj[parts[0]]
    obj.pop(parts[-1])
    add('required-bid-'+field,payload,False,exp('openrtb.profile.google.sdk_field_required',field))
for field in ['id','rendering_data','declared_ad']:
    for mode in ['missing','null']:
        payload=copy.deepcopy(B)
        if mode=='missing':sdk(payload).pop(field)
        else:sdk(payload)[field]=None
        add('required-sdk-'+field+'-'+mode,payload,False,
            exp('openrtb.profile.google.sdk_field_required','ext.sdk_rendered_ad.'+field))
payload=copy.deepcopy(B);sdk(payload)['id']='';add('empty-sdk-id',payload,False,
    exp('openrtb.profile.google.sdk_field_required','ext.sdk_rendered_ad.id'))
payload=copy.deepcopy(B);sdk(payload)['rendering_data']='';add('opaque-empty-rendering-data',payload)
payload=copy.deepcopy(B);sdk(payload)['declared_ad']={};add('declared-creative-empty',payload,False,
    exp('openrtb.profile.google.sdk_declared_creative_required','ext.sdk_rendered_ad.declared_ad'))
for content,value in [('html_snippet','<div>Ad</div>'),('video_url','https://ads.example/vast'),
                      ('video_vast_xml','<VAST version="3.0"></VAST>'),
                      ('native_response',{'assets':[{'img':{'type':3,'url':'https://ads.example/image'}}]})]:
    payload=copy.deepcopy(B);sdk(payload)['declared_ad']={content:value}
    add('declared-content-'+content,payload)
native={'assets':[{'img':{'type':3,'url':'https://ads.example/image'}}],
        'link':{'url':'https://advertiser.example'}}
for value,label,valid in [(3,'main',True),(1,'icon',True),(None,'null',False),('missing','missing',False)]:
    payload=copy.deepcopy(B);sdk(payload)['declared_ad']={'native_response':copy.deepcopy(native)}
    image=sdk(payload)['declared_ad']['native_response']['assets'][0]['img']
    if value=='missing':image.pop('type')
    else:image['type']=value
    add('native-image-type-'+label,payload,valid,[] if valid else exp(
        'openrtb.profile.google.sdk_native_image_type_required',
        'ext.sdk_rendered_ad.declared_ad.native_response.assets[0].img.type'),source=PROTO_SOURCE)
payload=copy.deepcopy(B);sdk(payload)['declared_ad']={'native_response':copy.deepcopy(native)}
r=copy.deepcopy(R);r['imp'][0].pop('banner');r['imp'][0]['native']={'request':json.dumps(
    {'ver':'1.2','assets':[{'id':999,'required':1,'title':{'len':50}}]})}
add('sdk-native-asset-id-independent',payload,request=r,source=PROTO_SOURCE)
for container,field,value in [('title','text','SDK title'),('img','url','https://ads.example/image'),
    ('video','vasttag','<VAST version="3.0"></VAST>'),('data','value','SDK data'),
    ('link','url','https://advertiser.example')]:
    response=copy.deepcopy(B)
    asset={container:{field:value}}
    if container=='img':asset['img']['type']=3
    sdk(response)['declared_ad']={'native_response':{'assets':[asset]}}
    add('native-'+container+'-'+field+'-present',response,source=NATIVE_SOURCE)
    sdk(response)['declared_ad']['native_response']['assets'][0][container].pop(field)
    add('native-'+container+'-'+field+'-required',response,False,
        exp('openrtb.profile.google.sdk_native_field_required',
            'ext.sdk_rendered_ad.declared_ad.native_response.assets[0].'+container+'.'+field),source=NATIVE_SOURCE)
response=copy.deepcopy(B);sdk(response)['declared_ad']={'native_response':{'link':{}}}
add('native-top-link-url-required',response,False,
    exp('openrtb.profile.google.sdk_native_field_required',
        'ext.sdk_rendered_ad.declared_ad.native_response.link.url'),source=NATIVE_SOURCE)
# SDK-only rules must not leak into ordinary native markup.
payload=copy.deepcopy(B);get_bid(payload)['ext'].pop('sdk_rendered_ad')
get_bid(payload)['adm']=json.dumps({'ver':'1.2','assets':[{'id':1,'img':{'url':'https://ads.example/image'}}],
    'link':{'url':'https://advertiser.example'}})
add('ordinary-native-image-type-optional',payload)

PA=[{'key':'account','value':'A'},{'key':'unit','value':'banner'}]
PB=[{'key':'account','value':'B'},{'key':'unit','value':'native'}]
r=copy.deepcopy(R);r['imp'][0]['ext']={'ad_unit_mapping':[{'format':1,'keyvals':PA},{'format':3,'keyvals':PB}]}
for name,params,valid in [('first',PA,True),('second',PB,True),('reordered',list(reversed(PA)),True),
    ('wrong-value',[{'key':'account','value':'unknown'},{'key':'unit','value':'banner'}],False),
    ('subset',[PA[0]],False),('cross-group',[PA[0],PB[1]],False),
    ('extra',PA+[{'key':'extra','value':'x'}],False),('empty',[],True),
    ('empty-response-value',[{'key':'account','value':''},{'key':'unit','value':'banner'}],False),
    ('empty-response-key',[{'key':'','value':'A'},{'key':'unit','value':'banner'}],False),
    ('duplicate-response-key',PA+[PA[0]],True)]:
    payload=copy.deepcopy(B);sdk(payload)['sdk_params']=copy.deepcopy(params)
    add('mapping-'+name,payload,valid,[] if valid else exp('openrtb.profile.google.sdk_params_not_offered',
        'ext.sdk_rendered_ad.sdk_params'),r,PROTO_SOURCE)
payload=copy.deepcopy(B);add('mapping-selection-omitted-no-inferred-requirement',payload,request=r,source=PROTO_SOURCE)
for name,mappings in [('absent',None),('empty',[]),('malformed-object',{}),
    ('missing-keyvals',[{'format':1}]),('empty-keyvals',[{'keyvals':[]}]),
    ('non-string-value',[{'keyvals':[{'key':'account','value':7}]}]),
    ('duplicate-keys',[{'keyvals':[PA[0],PA[0]]}]),
    ('malformed-format',[{'format':'unknown','keyvals':PA}]),
    ('future-format',[{'format':1000,'keyvals':PA}]),
    ('partial-alternative',[{'keyvals':PA},{'keyvals':[{'key':'unit'}]}]),
    ('blank-value',[{'keyvals':[{'key':'account','value':''}]}])]:
    req=copy.deepcopy(R)
    if mappings is not None:req['imp'][0]['ext']={'ad_unit_mapping':mappings}
    payload=copy.deepcopy(B);sdk(payload)['sdk_params']=[{'key':'unknown','value':'unknown'}]
    add('mapping-reference-'+name+'-defers',payload,request=req,source=PROTO_SOURCE)
for name,format in [('omitted',None),('unknown-enum-zero',0)]:
    req=copy.deepcopy(R);mapping={'keyvals':PA}
    if format is not None:mapping['format']=format
    req['imp'][0]['ext']={'ad_unit_mapping':[mapping]}
    payload=copy.deepcopy(B);sdk(payload)['sdk_params']=list(reversed(PA))
    add('mapping-format-'+name+'-complete-keyvals',payload,request=req,source=PROTO_SOURCE)
for name,params in [('null',None),('wrong-container',{}),('malformed-item',[{'key':'account','value':7}])]:
    payload=copy.deepcopy(B);sdk(payload)['sdk_params']=params
    add('mapping-response-'+name+'-wire-error',payload,False,
        exp('openrtb.profile.google.value_invalid','ext.sdk_rendered_ad.sdk_params'+('[0].value' if name=='malformed-item' else '')),r,PROTO_SOURCE)
for name,payload in [('ascii128','a'*128),('ascii129','a'*129),('utf8-64','é'*64),('utf8-65','é'*65),
                      ('base64-looking128',base64.b64encode(b'a'*96).decode()),
                      ('base64-looking172',base64.b64encode(b'a'*127).decode()),
                      ('empty','')]:
    response=copy.deepcopy(B);get_bid(response)['ext']['event_notification_token']={'payload':payload}
    add('token-'+name,response,errors=exp('openrtb.profile.google.event_token_ignored',
        'ext.event_notification_token.payload',True) if len(payload.encode())>128 else [],source=PROTO_SOURCE)

FILE.write_text(json.dumps(fixture,indent=2)+'\n')
print(json.dumps({'cases':len(fixture['cases']),'sdk_controls':sum(c['id'].startswith('google-sdk-depth-') for c in fixture['cases']),
                 'positive':sum(c['valid'] for c in fixture['cases'])}))
