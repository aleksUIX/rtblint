from pathlib import Path
import json,copy
cases=[]
R={'id':'tl-request','imp':[{'id':'1','tagid':'placement-1','banner':{'format':[{'w':320,'h':50}]}}]}
NR={'ver':'1.2','assets':[{'id':1,'required':1,'title':{'len':25}}],'eventtrackers':[{'event':1,'methods':[1,2]}]}
NA={'ver':'1.2','assets':[{'id':1,'title':{'text':'Native title'}}],'link':{'url':'https://advertiser.example'}}
N=copy.deepcopy(R);N['imp'][0].pop('banner');N['imp'][0]['native']={'ver':'1.2','request':json.dumps(NR)}
B={'id':'tl-request','seatbid':[{'bid':[{'id':'bid-1','impid':'1','price':1.2,'mtype':4,'adm':json.dumps(NA)}]}]}

def display(path):
 s=''
 for p in path:s+=f'[{p}]' if isinstance(p,int) else ('.' if s else '')+p
 return s
def setv(data,path,value):
 cur=data
 for key in path[:-1]:cur=cur[key] if isinstance(key,int) else cur.setdefault(key,{})
 cur[path[-1]]=copy.deepcopy(value)
def expect(id,path,warning=False):return [{'id':id,'path':path,'severity':'warning' if warning else 'error'}]
def add(id,direction,data,valid=True,expected=None,request=None,profile='triplelift-supplier'):
 c={'id':id,'profile':profile,'direction':direction,'input':copy.deepcopy(data),'valid':valid,'expected':expected or []}
 if request is not None:c['request']=copy.deepcopy(request)
 cases.append(c)
def shape(direction,payload,path,good,bad):
 for value,suffix,valid in [(good,'published-shape',True),(bad,'wrong-shape',False)]:
  inner=copy.deepcopy(payload)
  if len(path)>2 and path[0]=='assets' and path[2] in ('title','img','video','data'):
   for kind in ('title','img','video','data'):
    if kind!=path[2]:inner['assets'][path[1]].pop(kind,None)
  setv(inner,path,value)
  x=copy.deepcopy(N if direction=='request' else B)
  if direction=='request':x['imp'][0]['native']['request']=json.dumps(inner);prefix='imp[0].native.request'
  else:x['seatbid'][0]['bid'][0]['adm']=json.dumps(inner);prefix='seatbid[0].bid[0].adm'
  expected=[] if valid else expect('openrtb.profile.field_type',prefix+'.'+display(path))
  add(direction+'-'+display(path)+'-'+suffix,direction,x,valid,expected)

add('supplier-minimum','request',R)
add('native-request-minimum','request',N)
add('native-response-minimum','response',B)
# The current request table makes neither device nor app/site mandatory.
x=copy.deepcopy(R);x['device']={};add('missing-identifiers-and-consent-unknown-valid','request',x)
x=copy.deepcopy(R);x['imp'][0].pop('tagid');add('placement-required','request',x,False,expect('openrtb.profile.field_required','imp[0].tagid'))
x=copy.deepcopy(R);x['imp'][0]['banner']={};add('banner-size-missing','request',x,False,expect('openrtb.profile.field_required','imp[0].banner.format'))
x=copy.deepcopy(R);x['imp'][0]['banner']={'w':320,'h':50};add('banner-exact-fallback-contract-conflict','request',x,True,expect('openrtb.profile.triplelift.banner_format_fallback','imp[0].banner.format',True))
x=copy.deepcopy(R);x['imp'][0]['banner']={'format':[]};add('banner-format-empty','request',x,False,expect('openrtb.profile.triplelift.banner_format_empty','imp[0].banner.format'))
x=copy.deepcopy(R);x['app']={'id':'app-1'};add('bundle-optional-prose-contract-conflict','request',x,True,expect('openrtb.profile.triplelift.bundle_contract_conflict','app.bundle',True))
x=copy.deepcopy(R);x['app']={'bundle':'com.app'};add('bundle-published','request',x)
x=copy.deepcopy(R);x['imp'][0]['audio']={'mimes':['audio/mpeg']};add('multi-format-and-audio-currently-supported','request',x)
for path,good,bad in [(['ver'],'1.2',1.2),(['context'],1,True),(['plcmttype'],1,True),(['plcmtcnt'],1,1.5),(['aurlsupport'],1,True),(['assets'],[{'id':1,'title':{'len':25}}],{}),(['eventtrackers'],[{'event':1,'methods':[1,2]}],{})]:shape('request',NR,path,good,bad)
for path,good,bad in [(['id'],1,True),(['required'],1,True),(['title'],{'len':25},[]),(['title','len'],25,True),(['img'],{'type':3,'wmin':300,'hmin':250},[]),(['video'],{'mimes':['video/mp4'],'minduration':1,'maxduration':30,'protocols':[2,3]},[]),(['data'],{'type':1,'len':25},[])]:shape('request',NR,['assets',0]+path,good,bad)
IMAGE={'type':3,'w':300,'h':250,'wmin':300,'hmin':250,'mimes':['image/png']}
x=copy.deepcopy(NR);x['assets']=[{'id':1,'img':IMAGE}]
for field in ['type','w','h','wmin','hmin']:shape('request',x,['assets',0,'img',field],IMAGE[field],False)
shape('request',x,['assets',0,'img','mimes'],['image/png'],{})
VIDEO={'mimes':['video/mp4'],'minduration':1,'maxduration':30,'protocols':[2,3]}
x=copy.deepcopy(NR);x['assets']=[{'id':1,'video':VIDEO}]
for field,bad in [('mimes',{}),('minduration',False),('maxduration',False),('protocols',{})]:shape('request',x,['assets',0,'video',field],VIDEO[field],bad)
x=copy.deepcopy(NR);x['assets']=[{'id':1,'data':{'type':1,'len':25}}]
for field in ['type','len']:shape('request',x,['assets',0,'data',field],x['assets'][0]['data'][field],False)
for field,good,bad in [('event',1,True),('methods',[1,2],{})]:shape('request',NR,['eventtrackers',0,field],good,bad)
# Item types, required image declarations, and flags are independent mutations.
x=copy.deepcopy(NR);x['assets']=[{'id':1,'img':{'wmin':300,'hmin':250}}];y=copy.deepcopy(N);y['imp'][0]['native']['request']=json.dumps(x);add('image-type-required','request',y,False,expect('openrtb.profile.field_required','imp[0].native.request.assets[0].img.type'))
x=copy.deepcopy(NR);x['assets']=[{'id':1,'img':{'type':3}}];y=copy.deepcopy(N);y['imp'][0]['native']['request']=json.dumps(x);add('image-dimensions-recommended','request',y,True,expect('openrtb.profile.triplelift.image_size_recommended','imp[0].native.request.assets[0].img.wmin',True))
x=copy.deepcopy(NR);x['assets'][0]['required']=2;y=copy.deepcopy(N);y['imp'][0]['native']['request']=json.dumps(x);add('asset-required-closed-flag','request',y,False,expect('openrtb.profile.field_type','imp[0].native.request.assets[0].required'))
x=copy.deepcopy(NR);x['eventtrackers'][0]['methods']=[True];y=copy.deepcopy(N);y['imp'][0]['native']['request']=json.dumps(x);add('tracker-method-item-type','request',y,False,expect('openrtb.profile.field_type','imp[0].native.request.eventtrackers[0].methods[0]'))
# Published response table types, without obsolete required adomain or dimensions.
for path,good,bad in [(['ver'],'1.2',1.2),(['assets'],[{'id':1,'title':{'text':'Title'}}],{}),(['assetsurl'],'https://assets.example/creative',False),(['dcourl'],'https://assets.example/dco',False),(['link'],{'url':'https://advertiser.example'},[]),(['imptrackers'],['https://track.example/imp'],{}),(['jstracker'],'<script>track();</script>',False),(['privacy'],'https://advertiser.example/privacy',False),(['eventtrackers'],[{'event':1,'method':1,'url':'https://track.example/imp'}],{})]:shape('response',NA,path,good,bad)
for path,good,bad in [(['id'],1,True),(['required'],1,True),(['title'],{'text':'Title'},[]),(['title','text'],'Title',1),(['title','len'],5,False),(['img'],{'url':'https://assets.example/image'},[]),(['video'],{'vasttag':'<VAST version="4.0"></VAST>'},[]),(['link'],{'url':'https://advertiser.example/asset'},[])]:shape('response',NA,['assets',0]+path,good,bad)
x=copy.deepcopy(NA);x['assets']=[{'id':1,'img':{'url':'https://assets.example/image','type':3,'w':300,'h':250}}]
for field,good,bad in [('url','https://assets.example/image',1),('type',3,False),('w',300,False),('h',250,False)]:shape('response',x,['assets',0,'img',field],good,bad)
x=copy.deepcopy(NA);x['assets']=[{'id':1,'video':{'vasttag':'<VAST version="4.0"></VAST>'}}];shape('response',x,['assets',0,'video','vasttag'],'<VAST version="4.0"></VAST>',1)
for field,good,bad in [('url','https://advertiser.example',1),('clicktrackers',['https://track.example/click'],{}),('fallback','https://advertiser.example',1)]:shape('response',NA,['link',field],good,bad)
x=copy.deepcopy(NA);x['eventtrackers']=[{'event':1,'method':1,'url':'https://track.example','customdata':{}}]
for field,good,bad in [('event',1,True),('method',1,True),('url','https://track.example',1),('customdata',{'key':'opaque'},[])]:shape('response',x,['eventtrackers',0,field],good,bad)
x=copy.deepcopy(NA);x['jstracker']='track()';y=copy.deepcopy(B);y['seatbid'][0]['bid'][0]['adm']=json.dumps(x);add('jstracker-must-be-script-markup','response',y,False,expect('openrtb.profile.triplelift.jstracker_markup','seatbid[0].bid[0].adm.jstracker'))
for field in ['event','method']:
 x=copy.deepcopy(NA);x['eventtrackers']=[{'event':1,'method':1}];x['eventtrackers'][0].pop(field);y=copy.deepcopy(B);y['seatbid'][0]['bid'][0]['adm']=json.dumps(x);add('tracker-'+field+'-required','response',y,False,expect('openrtb.profile.field_required','seatbid[0].bid[0].adm.eventtrackers[0].'+field))
for field,value in [('event',0),('method',3)]:
 x=copy.deepcopy(NA);x['eventtrackers']=[{'event':1,'method':1}];x['eventtrackers'][0][field]=value;y=copy.deepcopy(B);y['seatbid'][0]['bid'][0]['adm']=json.dumps(x);add('tracker-'+field+'-closed-standard-range','response',y,False,expect('openrtb.profile.triplelift.event_enum','seatbid[0].bid[0].adm.eventtrackers[0].'+field))
x=copy.deepcopy(NA);x['eventtrackers']=[{'event':500,'method':500}];y=copy.deepcopy(B);y['seatbid'][0]['bid'][0]['adm']=json.dumps(x);add('native-vendor-event-method-allowed','response',y)
# Paired remote asset and tracking negotiation boundaries.
x=copy.deepcopy(NA);x['eventtrackers']=[{'event':1,'method':2,'url':'https://track.example/js'}];y=copy.deepcopy(B);y['seatbid'][0]['bid'][0]['adm']=json.dumps(x);add('pair-offered-event-and-method','pair',y,request=N)
x['eventtrackers'][0]['event']=2;y['seatbid'][0]['bid'][0]['adm']=json.dumps(x);add('pair-event-unoffered','pair',y,False,expect('openrtb.profile.triplelift.event_unoffered','seatbid[0].bid[0].adm.eventtrackers[0]'),N)
x=copy.deepcopy(NR);x['eventtrackers'][0]['methods']=['future'];r=copy.deepcopy(N);r['imp'][0]['native']['request']=json.dumps(x);add('pair-malformed-offer-defers','pair',y,request=r)
x={'link':{'url':'https://advertiser.example'},'assetsurl':'https://assets.example/creative'};y=copy.deepcopy(B);y['seatbid'][0]['bid'][0]['adm']=json.dumps(x)
r=copy.deepcopy(N);nr=copy.deepcopy(NR);nr['aurlsupport']=1;r['imp'][0]['native']['request']=json.dumps(nr);add('pair-assetsurl-supported','pair',y,request=r)
add('pair-assetsurl-omission-default-unsupported','pair',y,False,expect('openrtb.profile.triplelift.assetsurl_unsupported','seatbid[0].bid[0].adm.assetsurl'),N)
nr['aurlsupport']=0;r['imp'][0]['native']['request']=json.dumps(nr);add('pair-assetsurl-explicitly-unsupported','pair',y,False,expect('openrtb.profile.triplelift.assetsurl_unsupported','seatbid[0].bid[0].adm.assetsurl'),r)
nr['aurlsupport']='future';r['imp'][0]['native']['request']=json.dumps(nr);add('pair-assetsurl-malformed-support-defers','pair',y,request=r)
x=copy.deepcopy(NA);x['eventtrackers']=[{'event':2,'method':1}];y=copy.deepcopy(B);y['seatbid'][0]['bid'][0]['adm']=json.dumps({'native':x});add('pair-wrapper-event-path','pair',y,False,expect('openrtb.profile.triplelift.event_unoffered','seatbid[0].bid[0].adm.native.eventtrackers[0]'),N)
x=copy.deepcopy(NR);x['unknown_future_field']={'opaque':True};y=copy.deepcopy(N);y['imp'][0]['native']['request']=json.dumps(x);add('future-native-extensions-open','request',y)
x=copy.deepcopy(R);x['imp'][0].pop('tagid');add('spec-does-not-inherit-placement-requiredness','request',x,profile='spec')

p=Path(__file__).resolve().parents[2]/'fixtures/exchange-depth/triplelift/cases.json';p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps({'schema':'rtblint-independent-exchange-cases/v1','reviewed_at':'2026-10-09','sources':'../../../docs/exchange-profiles/triplelift-sources.json','cases':cases},indent=2)+'\n');print(len(cases),'cases',sum(c['valid'] for c in cases),'positive')
