import json,copy
from pathlib import Path
root=Path(__file__).resolve().parents[2]
cases=[]
INDEX='https://kb.indexexchange.com/dsps/open-rtb/list_of_supported_openrtb_bid_request_fields_dsp.htm'
SELLER='https://kb.indexexchange.com/publishers/openrtb_integration/list_of_supported_openrtb_bid_request_fields_for_sellers.htm'
DV='https://developers.google.com/display-video/ortb-spec'
ireq={'id':'req','tmax':200,'ext':{},'device':{},'imp':[{'id':'1','secure':1,'banner':{'w':320,'h':50,'topframe':0,'format':[{'w':320,'h':50},{'w':300,'h':250}]}}]}
sreq={'id':'req','tmax':200,'device':{'ip':'192.0.2.1'},'imp':[{'id':'1','banner':{'w':320,'h':50}}]}
dreq={'id':'req','device':{'ua':'Browser','ip':'192.0.2.1'},'user':{},'site':{'domain':'publisher.example'},'imp':[{'id':'1','banner':{'w':320,'h':50}}]}
resp={'id':'req','seatbid':[{'seat':'buyer','bid':[{'id':'bid','impid':'1','price':1.2,'mtype':1,'w':320,'h':50,'adomain':['advertiser.example'],'adm':'<div>Banner</div>'}]}]}
video={'mimes':['video/mp4'],'protocols':[3],'minduration':0,'maxduration':30,'w':640,'h':360}
native_req={'ver':'1.2','assets':[{'id':1,'required':1,'title':{'len':50}}]}
native_resp={'native':{'ver':'1.2','assets':[{'id':1,'title':{'text':'Title'}}],'link':{'url':'https://advertiser.example'}}}
skad={'version':'2.0','network':'network.skadnetwork','campaign':'1','itunesitem':'456','sourceapp':'123','nonce':'nonce','timestamp':'1','signature':'opaque'}
def issue(id,path): return {'id':id,'path':path,'severity':'error'}
def add(id,profile,p,valid=True,expected=None,request=None,forbidden=None,source=None):
 c={'id':id,'profile':profile,'direction':'pair' if request is not None else 'request','input':copy.deepcopy(p),'valid':valid,'expected':expected or [],'forbidden':forbidden or [],'source':source or (DV if profile=='dv360' else SELLER if profile.endswith('seller') else INDEX)}
 if request is not None: c['request']=copy.deepcopy(request)
 cases.append(c)
def pair(id,req,r,valid=True,expected=None,forbidden=None):add(id,'index-exchange',r,valid,expected,req,forbidden)
for ext in [{},{'hb':1},{'gpid':'slot'}]:
 q=copy.deepcopy(ireq);q['imp'][0]['ext']=ext;add('index-unavailable-bcrid-'+str(len(cases)),'index-exchange',q)
for size in [(320,50),(300,250),(728,90)]:
 r=copy.deepcopy(resp);r['seatbid'][0]['bid'][0].update(w=size[0],h=size[1]);pair('index-fixed-size-'+str(size),ireq,r,size!=(728,90),[issue('openrtb.profile.index.banner_size_unoffered','seatbid[0].bid[0].w')] if size==(728,90) else [])
for field in ['w','h']:
 r=copy.deepcopy(resp);r['seatbid'][0]['bid'][0].pop(field);pair('index-banner-missing-'+field,ireq,r,False,[issue('openrtb.profile.field_required','seatbid[0].bid[0].'+field)])
for mtype in [2,4,None]:
 q=copy.deepcopy(ireq);q['imp'][0]['video']=copy.deepcopy(video);q['imp'][0]['native']={'request':json.dumps(native_req),'ver':'1.2'}
 r=copy.deepcopy(resp);b=r['seatbid'][0]['bid'][0];b.pop('w');b.pop('h')
 if mtype is None: b.pop('mtype')
 else:b['mtype']=mtype
 if mtype==4:b['adm']=json.dumps(native_resp)
 elif mtype==2:b['adm']='<VAST version="3.0"></VAST>'
 pair('index-multiformat-no-banner-dimensions-'+str(mtype),q,r)
q=copy.deepcopy(ireq);q['imp'][0]['video']='malformed'
r=copy.deepcopy(resp);b=r['seatbid'][0]['bid'][0];b.pop('mtype');b.pop('w');b.pop('h')
pair('index-malformed-media-does-not-infer-banner',q,r,True,forbidden=[{'id':'openrtb.profile.field_required','path':'seatbid[0].bid[0].w'},{'id':'openrtb.profile.field_required','path':'seatbid[0].bid[0].h'}])
q=copy.deepcopy(ireq);r=copy.deepcopy(resp);b=r['seatbid'][0]['bid'][0];b['mtype']='malformed';b.pop('w');b.pop('h')
pair('index-malformed-mtype-does-not-infer-banner',q,r,False,forbidden=[{'id':'openrtb.profile.field_required','path':'seatbid[0].bid[0].w'}])
# Range, ratio and incomplete dimensions must not prove a fixed-size membership violation.
for name,formats,extra,valid in [('ratio',[{'wratio':16,'hratio':9,'wmin':300}],{},True),('range',[{'w':320,'h':50}],{'wmin':100,'wmax':1000},True),('empty',[],{},True),('incomplete',[{'w':320}],{},True),('malformed',[{'w':'wrong','h':50}],{},True)]:
 q=copy.deepcopy(ireq);q['imp'][0]['banner'].update(format=formats,**extra)
 r=copy.deepcopy(resp);r['seatbid'][0]['bid'][0].update(w=728,h=90)
 pair('index-nonfixed-format-'+name,q,r,valid,forbidden=[{'id':'openrtb.profile.index.banner_size_unoffered','path':'seatbid[0].bid[0].w'}])
for name,formats,valid in [('empty',[],True),('malformed',[{'w':'wrong','h':50}],True),('incomplete',[{'w':320}],True),('ratio',[{'wratio':16,'hratio':9,'wmin':300}],False)]:
 q=copy.deepcopy(ireq);q['imp'][0]['banner']['format']=formats
 r=copy.deepcopy(resp);r['seatbid'][0]['bid'][0].pop('w');r['seatbid'][0]['bid'][0].pop('h')
 pair('index-dimensions-reference-'+name,q,r,valid,[issue('openrtb.profile.field_required','seatbid[0].bid[0].w')] if not valid else [],forbidden=[{'id':'openrtb.profile.field_required','path':'seatbid[0].bid[0].w'}] if valid else [])
for profile,base in [('index-exchange',ireq),('index-exchange-seller',sreq)]:
 for at in [1,2,3,500,501]:
  q=copy.deepcopy(base);q['imp'][0]['pmp']={'deals':[{'id':'deal','at':at}]};valid=at in [1,3] or (at>=500 and profile.endswith('seller'));add(profile+'-deal-at-'+str(at),profile,q,valid,[issue('openrtb.profile.value_invalid','imp[0].pmp.deals[0].at')] if not valid else [])
for metric in ['click_through_rate','video_completion_rate','viewability','session_depth','future_metric']:
 for value in [-0.1,0,1,1.1,3]:
  q=copy.deepcopy(ireq);q['imp'][0]['metric']=[{'type':metric,'value':value}]
  valid=metric in ['session_depth','future_metric'] or 0<=value<=1
  add('index-metric-'+metric+'-'+str(value),'index-exchange',q,valid,[issue('openrtb.profile.index.metric_probability','imp[0].metric[0].value')] if not valid else [])
for name,fields,valid,expected in [('null',{'poddur':None},False,[issue('openrtb.type.mismatch','imp[0].video.poddur')]),('dynamic-complete',{'poddur':60,'maxseq':2,'podid':'pod'},True,[]),('no-maxseq',{'poddur':60,'podid':'pod'},False,[issue('openrtb.profile.field_required','imp[0].video.maxseq')]),('no-podid',{'poddur':60,'maxseq':2},False,[issue('openrtb.profile.field_required','imp[0].video.podid')]),('structured',{'podid':'pod'},True,[])]:
 q=copy.deepcopy(sreq);q['imp'][0].pop('banner');q['imp'][0]['video']=dict(video,**fields);add('index-seller-pod-'+name,'index-exchange-seller',q,valid,expected)
for name,offered,valid,expected in [('matching',['network.skadnetwork'],True,[]),('unoffered',['other.skadnetwork'],False,[issue('openrtb.profile.index.skad_network','seatbid[0].bid[0].ext.skadn.network')]),('malformed',[1],True,[]),('mixed',['other.skadnetwork',1],True,[]),('empty',[],True,[]),('blank',[''],True,[])]:
 q=copy.deepcopy(ireq);q['imp'][0]['ext']={'skadn':{'version':'2.0','sourceapp':'123','skadnetids':offered}}
 r=copy.deepcopy(resp);r['seatbid'][0]['bid'][0]['ext']={'skadn':copy.deepcopy(skad)}
 pair('index-skad-'+name,q,r,valid,expected,forbidden=[{'id':'openrtb.profile.index.skad_network','path':'seatbid[0].bid[0].ext.skadn.network'}] if name not in ['unoffered'] else [])
for publisher in [{'id':'pub'},{},{'id':''},{'id':None}]:
 q=copy.deepcopy(dreq);q['site']['publisher']=publisher;valid=publisher.get('id')=='pub';add('dv360-publisher-'+str(len(cases)),'dv360',q,valid,[issue('openrtb.profile.field_required','site.publisher.id')] if not valid else [])
for ver in [None,'1.0','1.1','1.2']:
 q=copy.deepcopy(dreq);q['imp'][0].pop('banner');q['imp'][0]['native']={'request':json.dumps(native_req)}
 if ver is not None:q['imp'][0]['native']['ver']=ver
 valid=ver not in ['1.0','1.1'];add('dv360-native-version-'+str(ver),'dv360',q,valid,[issue('openrtb.profile.dv360.native_version','imp[0].native.ver')] if not valid else [])
for plcmt in [None,0,1,2,3,4]:
 for methods in [[0],[2],[6],[2,6],[1],[3],[4],[5],[1,2],[]]:
  q=copy.deepcopy(dreq);q['imp'][0].pop('banner');q['imp'][0]['video']=dict(video,playbackmethod=methods)
  if plcmt is not None:q['imp'][0]['video']['plcmt']=plcmt
  valid=not(methods and all(m in [0,2,6] for m in methods) and plcmt in [None,0,1])
  add('dv360-sound-'+str(plcmt)+'-'+str(methods),'dv360',q,valid,[issue('openrtb.profile.dv360.instream_sound','imp[0].video.playbackmethod')] if not valid else [])
for plcmt,placement,valid in [(0,2,False),(1,2,False),(2,1,True),(None,2,True),(None,5,True)]:
 q=copy.deepcopy(dreq);q['imp'][0].pop('banner');q['imp'][0]['video']=dict(video,playbackmethod=[2],placement=placement)
 if plcmt is not None:q['imp'][0]['video']['plcmt']=plcmt
 add('dv360-placement-precedence-'+str((plcmt,placement)),'dv360',q,valid,[issue('openrtb.profile.dv360.instream_sound','imp[0].video.playbackmethod')] if not valid else [])
out=root/'fixtures/exchange-depth/index-dv360-peer-review';out.mkdir(parents=True,exist_ok=True);(out/'cases.json').write_text(json.dumps({'cases':cases},indent=2)+'\n');print(len(cases))
