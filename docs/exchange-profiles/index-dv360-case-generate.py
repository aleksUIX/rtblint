import json,copy
from pathlib import Path
root=Path(__file__).resolve().parents[2]
data=json.loads((root/'docs/exchange-profiles/index-field-inventory.json').read_text())
base_seller={'id':'req','tmax':100,'device':{'ip':'192.0.2.1'},'site':{'domain':'publisher.example'},'imp':[{'id':'1','banner':{'w':300,'h':250}}]}
base_dsp={'id':'req','tmax':20,'ext':{},'device':{},'site':{'id':'site','publisher':{'id':'pub'}},'imp':[{'id':'1','secure':1,'ext':{'bcrid':[]},'banner':{'w':300,'h':250,'topframe':1}}]}
base_dv={'id':'req','device':{'ua':'browser','ip':'192.0.2.1'},'user':{},'site':{'domain':'publisher.example'},'imp':[{'id':'1','banner':{'w':300,'h':250}}]}
base_response={'id':'req','seatbid':[{'seat':'1','bid':[{'id':'bid','impid':'1','price':1.0,'adm':'<div>Ad</div>','adomain':['advertiser.example'],'mtype':1}]}]}
skad_request={'version':'2.0','versions':['2.0'],'sourceapp':'123','skadnetids':['example.skadnetwork'],'skadnetlist':{'max':1,'excl':[1],'addl':['example.skadnetwork']}}
skad_response={'version':'2.0','network':'example.skadnetwork','campaign':'1','itunesitem':'456','sourceapp':'123','nonce':'nonce','timestamp':'1','signature':'opaque','fidelities':[{'fidelity':0,'nonce':'nonce','timestamp':'1','signature':'opaque'}]}
cases=[]
def attach(profile,obj):
 p=copy.deepcopy(base_response if profile=='index-response' else base_seller if profile=='index-seller' else base_dsp)
 if profile=='index-response':
  return p,p if obj=='BidResponse' else p['seatbid'][0]['bid'][0],' ' if obj=='BidResponse' else 'seatbid[0].bid[0]'
 if obj in ['DSA','TRANSPARENCY']:
  parent=p.setdefault('regs',{}).setdefault('ext',{}).setdefault('dsa',{'dsarequired':1,'pubrender':1,'datatopub':1,'transparency':[{'domain':'example','dsaparams':[1]}]})
  return p,parent if obj=='DSA' else parent['transparency'][0],'regs.ext.dsa'+('' if obj=='DSA' else '.transparency[0]')
 if obj in ['SKAD','SKADLIST']:
  parent=p['imp'][0].setdefault('ext',{}).setdefault('skadn',copy.deepcopy(skad_request))
  return p,parent if obj=='SKAD' else parent['skadnetlist'],'imp[0].ext.skadn'+('' if obj=='SKAD' else '.skadnetlist')
 if obj=='PLACEMENT':return p,p.setdefault('ext',{}).setdefault('placement',{'name':'slot','private':1}),'ext.placement'
 if obj=='BidRequest':return p,p,''
 if obj=='Imp':return p,p['imp'][0],'imp[0]'
 if obj in ['Banner','Video','Native','Deal','Pmp']:
  imp=p['imp'][0]
  if obj=='Banner':return p,imp['banner'],'imp[0].banner'
  if obj in ['Deal','Pmp']:
   pmp=imp.setdefault('pmp',{'deals':[{'id':'deal'}]})
   return p,pmp['deals'][0] if obj=='Deal' else pmp,'imp[0].pmp'+('.deals[0]' if obj=='Deal' else '')
  imp.pop('banner',None)
  value={'mimes':['video/mp4'],'protocols':[3],'minduration':5,'maxduration':30,'w':640,'h':360} if obj=='Video' else {'request':json.dumps({'ver':'1.2','assets':[{'id':1,'title':{'len':50}}]})}
  imp[obj.lower()]=value;return p,value,'imp[0].'+obj.lower()
 if obj=='Source':return p,p.setdefault('source',{'ext':{'sourceType':1,'sourceOrigin':1}}),'source'
 if obj=='App':
  p.pop('site',None);return p,p.setdefault('app',{'id':'app','bundle':'com.example.app','publisher':{'id':'pub'}}),'app'
 if obj=='Content':return p,p.setdefault('site',{}).setdefault('content',{}),'site.content'
 if obj in ['EID','UID']:
  e=p.setdefault('user',{}).setdefault('eids',[{'source':'id.example','uids':[{'id':'abc','atype':1}]}])[0]
  return p,e if obj=='EID' else e['uids'][0],'user.eids[0]'+('' if obj=='EID' else '.uids[0]')
 if obj=='Data':return p,p.setdefault('user',{}).setdefault('data',[{'id':'data'}])[0],'user.data[0]'
 return p,p.setdefault(obj.lower(),{}),obj.lower()
def sample(field,kind):
 if field=='dsa':return {'paid':'Buyer'} if 'INDEX_RESPONSE' in kind else {'dsarequired':1,'pubrender':1,'datatopub':1,'transparency':[{'domain':'example','dsaparams':[1]}]}
 if field=='skadn':return copy.deepcopy(skad_response if 'INDEX_RESPONSE' in kind else skad_request)
 if field=='skadnetlist':return copy.deepcopy(skad_request['skadnetlist'])
 if field=='fidelities':return copy.deepcopy(skad_response['fidelities'])
 if field=='transparency':return [{'domain':'example','dsaparams':[1]}]
 if field=='placement':return {'name':'slot','private':1}
 if 'Array' in kind:return ['2.0'] if field=='versions' else ['example'] if 'String' in kind else [1] if 'Integer' in kind else [{}]
 if 'String' in kind:return '2.0' if field=='version' else '123' if field=='sourceapp' else '456' if field=='itunesitem' else '1' if field=='campaign' else 'example'
 if 'Object' in kind:return {}
 if 'Number' in kind:return 0.25 if field=='disc' else 1.0
 return 1
for i,row in enumerate(data['fields']):
 surface,obj,field,kind=[row[k] for k in ['surface','object','field','type']]
 p,container,path=attach(surface,obj)
 if surface=='index-response' and obj in ['DSA','TRANSPARENCY','SKAD','FIDELITY']:
  parent=p['seatbid'][0]['bid'][0].setdefault('ext',{})
  if obj in ['DSA','TRANSPARENCY']:
   parent=parent.setdefault('dsa',{'paid':'Buyer','transparency':[{'domain':'example','dsaparams':[1]}]});container=parent if obj=='DSA' else parent['transparency'][0];path='seatbid[0].bid[0].ext.dsa'+('' if obj=='DSA' else '.transparency[0]')
  else:
   parent=parent.setdefault('skadn',copy.deepcopy(skad_response));container=parent if obj=='SKAD' else parent['fidelities'][0];path='seatbid[0].bid[0].ext.skadn'+('' if obj=='SKAD' else '.fidelities[0]')
 elif obj not in ['DSA','TRANSPARENCY','SKAD','SKADLIST','PLACEMENT','FIDELITY']:
  container=container.setdefault('ext',{});path=(path.strip()+'.ext').strip('.')
 container[field]=sample(field,kind)
 payload_path=(path+'.'+field).strip('.')
 profile='index-exchange-seller' if surface=='index-seller' else 'index-exchange'
 direction='response' if surface=='index-response' else 'request'
 case={'id':f'index-shape-{i}-valid','profile':profile,'direction':direction,'input':p,'valid':True,'expected':[],'source':'index-published-tables'};cases.append(case)
 bad=copy.deepcopy(p)
 # Walk bracket paths for a one-field type mutation.
 cursor=bad
 import re
 parts=re.findall(r'[^.\[\]]+|\[\d+\]',payload_path)
 for part in parts[:-1]:cursor=cursor[int(part[1:-1])] if part.startswith('[') else cursor[part]
 cursor[parts[-1]]='wrong' if any(x in kind for x in ['Integer','Number','Flag','Object','Array']) else 123
 cases.append({'id':f'index-shape-{i}-wrong','profile':profile,'direction':direction,'input':bad,'valid':False,'expected':[{'id':'openrtb.profile.field_type','path':payload_path}],'source':'index-published-tables'})
def add(id,profile,p,valid=True,expected=None,direction='request',request=None):
 case={'id':id,'profile':profile,'direction':direction,'input':copy.deepcopy(p),'valid':valid,'expected':expected or [],'source':'hand-authored-primary-contract'}
 if request is not None:case['request']=copy.deepcopy(request)
 cases.append(case)
def issue(id,path):return {'id':id,'path':path}
for profile,base in [('index-exchange',base_dsp),('index-exchange-seller',base_seller),('dv360',base_dv)]:add(profile+'-base',profile,base)
for time in [99,100,101]:
 p=copy.deepcopy(base_seller);p['tmax']=time;add('index-seller-tmax-'+str(time),'index-exchange-seller',p,time>=100,[issue('openrtb.profile.index.tmax_minimum','tmax')] if time<100 else [])
p=copy.deepcopy(base_dsp);p['tmax']=1;add('index-dsp-has-no-inbound-minimum','index-exchange',p)
p=copy.deepcopy(base_seller);p['device']={'ipv6':'2001:db8::1'};add('index-seller-ipv6','index-exchange-seller',p)
p=copy.deepcopy(base_seller);p['device']={};add('index-seller-ip-missing','index-exchange-seller',p,False,[issue('openrtb.profile.index.ip_required','device.ip')])
p=copy.deepcopy(base_dsp);p['imp'][0]['banner']['format']=[{'w':300,'h':250}]
resp=copy.deepcopy(base_response)
add('index-multisize-missing-response-size','index-exchange',resp,False,[issue('openrtb.profile.field_required','seatbid[0].bid[0].w'),issue('openrtb.profile.field_required','seatbid[0].bid[0].h')],'pair',p)
resp['seatbid'][0]['bid'][0].update(w=300,h=250);add('index-multisize-response-size','index-exchange',resp,True,direction='pair',request=p)
p=copy.deepcopy(base_dv);p['at']=3;p['imp'][0]['pmp']={'deals':[{'id':'deal'}]};add('dv360-inherited-fixed-floor','dv360',p,False,[issue('openrtb.profile.field_required','imp[0].pmp.deals[0].bidfloor')]);p['imp'][0]['pmp']['deals'][0]['bidfloor']=0;add('dv360-fixed-zero-floor','dv360',p)
p=copy.deepcopy(base_dv);p['imp'][0]['pmp']={'deals':[{'id':'deal'},{'id':'deal'}]};add('dv360-duplicate-deals','dv360',p,False,[issue('openrtb.profile.dv360.duplicate_deal','imp[0].pmp.deals[1].id')])
for ext,valid in [({'disable_gma_format':1},False),({'gdemsignals':'opaque','disable_gma_format':1},True)]:
 p=copy.deepcopy(base_dv);p['ext']=ext;add('dv360-gma-'+str(valid),'dv360',p,valid,[issue('openrtb.profile.dv360.gma_dependency','ext.disable_gma_format')] if not valid else [])
p=copy.deepcopy(base_dv);p.pop('site');p['app']={'bundle':'com.example.app'};p['device']['os']='iOS';add('dv360-ios-bundle-reverse-dns','dv360',p,False,[issue('openrtb.profile.dv360.ios_bundle','app.bundle')]);p['app']['bundle']='123456';add('dv360-ios-bundle-numeric','dv360',p)
for plcmt,method,valid in [(1,2,False),(1,1,True),(2,2,True),(0,0,False),(2,0,True)]:
 p=copy.deepcopy(base_dv);p['imp'][0].pop('banner');p['imp'][0]['video']={'mimes':['video/mp4'],'protocols':[3],'plcmt':plcmt,'playbackmethod':[method]};add(f'dv360-sound-{plcmt}-{method}','dv360',p,valid,[issue('openrtb.profile.dv360.instream_sound','imp[0].video.playbackmethod')] if not valid else [])
p=copy.deepcopy(base_dv);p['imp'][0].pop('banner');p['imp'][0]['video']={'mimes':['video/mp4'],'protocols':[3],'plcmt':2,'placement':1,'playbackmethod':[2]};add('dv360-plcmt-precedes-placement','dv360',p)
add('dv360-invalid-request-id-zero','dv360',{'id':'0','nbr':2},True,direction='pair',request=base_dv)
add('dv360-id-zero-normal-bid-still-mismatch','dv360',dict(base_response,id='0'),False,[issue('openrtb.response.request_id_mismatch','id')],'pair',base_dv)
# Every DV360 typed extension has a positive and wrong-shape control.
dv_fields={'BidRequest':{'purch':1,'gdemsignals':'opaque','disable_gma_format':1,'schain':{}},'Source':{'omidpn':'partner','omidpv':'1','schain':{}},'Regs':{'gdpr':0,'us_privacy':'1---'},'Video':{'rewarded':1},'Deal':{'guaranteed':1},'Site':{'inventorypartnerdomain':'partner.example'},'App':{'inventorypartnerdomain':'partner.example'},'Data':{'segtax':1,'segclass':'1'},'Device':{'truncated_ip':1,'ifa_type':'idfa','attestation_token':'opaque','atts':1,'cdep':'label'},'User':{'consent':'','us_privacy':'1---','eids':[{'source':'id.example','uids':[{'id':'abc','atype':1}]}],'consented_providers_settings':{'consented_providers':[1]}},'BidResponse':{'err':'error','errHelp':'https://developers.google.com/display-video/ortb-spec'},'Bid':{'apis':[7]}}
for obj,fields in dv_fields.items():
 for field,val in fields.items():
  if obj in ['BidResponse','Bid']:
   p=copy.deepcopy(base_response);container=p if obj=='BidResponse' else p['seatbid'][0]['bid'][0];path='' if obj=='BidResponse' else 'seatbid[0].bid[0]';direction='response'
  else:
   # Use contextual placement from Index builder, then restore DV360 required top-level context.
   p,container,path=attach('index-dsp',obj)
   p['device'].update(base_dv['device']);p.setdefault('user',{});direction='request'
  if obj=='App':container['bundle']='com.example.app'
  ext=container.setdefault('ext',{});ext[field]=copy.deepcopy(val)
  if obj=='BidRequest' and field=='disable_gma_format':ext['gdemsignals']='opaque'
  # Empty schain objects are rejected by existing generic SupplyChain semantics. Use full valid community shape.
  if field=='schain':ext[field]={'ver':'1.0','complete':1,'nodes':[{'asi':'exchange.example','sid':'pub','hp':1}]}
  add('dv360-shape-'+obj+'-'+field,'dv360',p,True,direction=direction)
  ext[field]=False if isinstance(val,str) else 'wrong'
  add('dv360-shape-'+obj+'-'+field+'-wrong','dv360',p,False,[issue('openrtb.profile.field_type',(path+'.ext.'+field).strip('.'))],direction)
out=root/'fixtures/exchange-depth/index-dv360';out.mkdir(parents=True,exist_ok=True);(out/'cases.json').write_text(json.dumps({'cases':cases},indent=2)+'\n');print('Root cases:',len(cases))
