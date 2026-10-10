from pathlib import Path
import json,copy
root=Path(__file__).resolve().parents[6]
out=root/'crates/rtblint-core/tests/fixtures/exchange-depth/equativ';out.mkdir(parents=True,exist_ok=True)
source='https://help.equativ.com/open-rtb-api-integration-bid-request-specification'
response_source='https://help.equativ.com/open-rtb-api-integration-bid-response-specification'
def clone(x):return copy.deepcopy(x)
def request():return {'id':'r','imp':[{'id':'i','tagid':'placement','banner':{'w':300,'h':250}}],'site':{'domain':'publisher.example','page':'https://publisher.example/page','publisher':{'id':'1234'}},'device':{'ip':'203.0.113.2'},'user':{'buyeruid':'matched-user'}}
def response():return {'id':'r','seatbid':[{'seat':'buyer','bid':[{'id':'b','impid':'i','price':1.2,'adm':'<div>Example</div>','w':300,'h':250}]}]}
cases=[]
def issue(id,path):return {'id':id,'path':path}
def add(name,s=False,mode='request',payload=None,req=None,errors=None,required=None,forbidden=None,valid=None,warnings=None):
 c={'name':name,'profile':'equativ-supplier' if s else 'equativ','mode':mode,'payload':clone(payload),'source':source if mode=='request' else response_source}
 if req is not None:c['request']=clone(req)
 if errors is not None:c['expected_profile_errors']=errors
 if required is not None:c['required_issues']=required
 if forbidden is not None:c['forbidden_issues']=forbidden
 if valid is not None:c['valid']=valid
 if warnings is not None:c['expected_profile_warnings']=warnings
 cases.append(c)
def locate(obj):
 r=request();b=response();mode='request';path=''
 if obj=='BidRequest':target=r
 elif obj=='Imp':target=r['imp'][0];path='imp[0]'
 elif obj in ['Banner','Video','Audio','Native']:
  imp=r['imp'][0]
  if obj!='Banner':
   del imp['banner'];imp[obj.lower()]={'mimes':['video/mp4'],'plcmt':1,'protocols':[6,8]} if obj=='Video' else {'mimes':['audio/mpeg']} if obj=='Audio' else {'request':json.dumps({'ver':'1.2','assets':[{'id':1,'required':1,'title':{'len':40}}]})}
  target=imp[obj.lower()];path='imp[0].'+obj.lower()
 elif obj=='Site':target=r['site'];path='site'
 elif obj=='Device':target=r['device'];path='device'
 elif obj=='User':target=r['user'];path='user'
 elif obj=='Publisher':target=r['site']['publisher'];path='site.publisher'
 elif obj in ['Regs','Source']:r[obj.lower()]={};target=r[obj.lower()];path=obj.lower()
 elif obj=='BidResponse':target=b;mode='response'
 elif obj=='Bid':target=b['seatbid'][0]['bid'][0];mode='response';path='seatbid[0].bid[0]'
 else:raise ValueError(obj)
 return (b if mode=='response' else r),target,mode,path
facts=[]
def fact(obj,path,shape,supplier_only=False):facts.append({'object':obj,'path':path,'shape':shape,'supplier_only':supplier_only,'source':response_source if obj in ['BidResponse','Bid'] else source})
fact('BidRequest','ext.network_id','Integer',True)
for obj in ['BidRequest','Imp']:
 fact(obj,'ext.bid_feedback','Objects')
 for field,kind in [('feedback_token','String'),('loss','Integer'),('price','Number')]:fact(obj,'ext.bid_feedback[].'+field,kind)
fact('Imp','ext.bidder','Object')
for field,kind in [('plcmtuuid','String'),('siteId','Integer'),('pageId','Integer'),('formatId','Integer')]:fact('Imp','ext.bidder.'+field,kind)
for field in ['gpid','dfp_ad_unit_code']:fact('Imp','ext.'+field,'String')
for obj in ['Banner','Video','Audio','Native']:fact(obj,'ext.bidfloor','Number')
for obj,path,kind in [('Video','ext.orientation','Integer'),('Video','ext.rewarded','Integer'),('Site','ext.amp','Integer'),('Device','ext.atts','Integer'),('Device','ext.ifa_type','String'),('User','ext.consent','String'),('Regs','ext.gdpr','Integer'),('Source','ext.omidpn','String'),('Source','ext.omidpv','String'),('Source','ext.schain','Object'),('Publisher','ext.directpay','Flag'),('BidResponse','ext.dealtype','String'),('BidResponse','ext.feedback_token','String'),('Bid','ext.dsa','Object'),('Regs','ext.dsa','Object')]:fact(obj,path,kind)
for f,k in [('complete','Integer'),('ver','String'),('nodes','Objects')]:fact('Source','ext.schain.'+f,k)
for f,k in [('asi','String'),('sid','String'),('hp','Integer'),('rid','String'),('name','String'),('domain','String')]:fact('Source','ext.schain.nodes[].'+f,k)
for f in ['behalf','paid']:fact('Bid','ext.dsa.'+f,'String')
fact('Bid','ext.dsa.adrender','Integer')
for f in ['dsarequired','pubrender','datatopub']:fact('Regs','ext.dsa.'+f,'Integer')
fact('User','ext.eids','Objects')
for f,k in [('source','String'),('inserter','String'),('matcher','String'),('mm','Integer'),('ext','Object'),('uids','Objects')]:fact('User','ext.eids[].'+f,k)
for f,k in [('id','String'),('atype','Integer'),('ext','Object')]:fact('User','ext.eids[].uids[].'+f,k)
for obj in ['Bid','Regs']:
 fact(obj,'ext.dsa.transparency','Objects');fact(obj,'ext.dsa.transparency[].domain','String');fact(obj,'ext.dsa.transparency[].dsaparams','Integers')
def setpath(obj,path,value):
 parts=path.split('.')
 for p in parts[:-1]:
  if p.endswith('[]'):
   p=p[:-2]
   if p not in obj or not obj[p]:obj[p]=[{}]
   obj=obj[p][0]
  else:
   if p not in obj or not isinstance(obj[p],dict):obj[p]={}
   obj=obj[p]
 obj[parts[-1]]=clone(value)
def example(d):
 k=d['shape'];p=d['path']
 if k=='String':return 'GuaranteedDeal' if p=='ext.dealtype' else '1.0' if p=='ext.schain.ver' else 'example'
 if k=='Integer':return 1
 if k=='Number':return 1.25
 if k=='Flag':return True
 if k=='Object':return {}
 return []
for index,d in enumerate(facts):
 for sup in [True] if d['supplier_only'] else [False,True]:
  for suffix,value,good in [('shape',example(d),True),('malformed',False if d['shape']!='Flag' else 'yes',False),('null',None,True)]:
   p,o,mode,base=locate(d['object']);setpath(o,d['path'],value);path=(base+'.' if base else '')+d['path'].replace('[]','[0]');e=issue('openrtb.profile.field_type',path)
   add(f'descriptor-{index:03}-{sup}-{suffix}',sup,mode,p,required=None if good else [e],forbidden=[e] if good else None)
for s in [False,True]:
 label='supplier' if s else 'bidder';r=request();b=response()
 add(label+'-control',s,payload=r,errors=[],valid=True)
 add(label+'-response-control',s,'pair',b,r,errors=[],valid=True)
 for app in [{'id':'store-id','publisher':{'id':'1234'}},{'publisher':{'id':'1234'}},{'bundle':'com.example','publisher':{'id':'1234'}}]:
  p=clone(r);del p['site'];p['app']=app
  add(label+'-app-'+('bundle' if 'bundle' in app else 'id-only' if 'id' in app else 'identity-absent'),s,payload=p,errors=[],valid=True,warnings=[issue('openrtb.profile.equativ.app_identity_advisory','app.bundle')] if s and 'bundle' not in app else [])
 for field,values in [('dsarequired',[0,1,2,3,4]),('pubrender',[0,1,2,3]),('datatopub',[0,1,2,3])]:
  for v in values:
   p=clone(r);p['regs']={'ext':{'dsa':{field:v}}};bad=v==values[-1]
   add(label+'-dsa-'+field+'-'+str(v),s,payload=p,errors=[issue('openrtb.profile.value_invalid','regs.ext.dsa.'+field)] if bad else [],valid=not bad)
 for requirement in [0,1,2,3]:
  q=clone(r);q['regs']={'ext':{'dsa':{'dsarequired':requirement}}}
  add(label+f'-dsa-pair-{requirement}-absent',s,'pair',b,q,errors=[issue('openrtb.profile.field_required','seatbid[0].bid[0].ext.dsa')] if requirement in [2,3] else [],valid=requirement in [0,1])
  p=clone(b);p['seatbid'][0]['bid'][0]['ext']={'dsa':{'behalf':'advertiser','paid':'payer','adrender':1}}
  add(label+f'-dsa-pair-{requirement}-present',s,'pair',p,q,errors=[],valid=True)
 p=clone(r);p['user']['ext']={'eids':[{'source':'id.example','uids':[{'id':'example','atype':1}]}]};add(label+'-legacy-eids-control',s,payload=p,errors=[],valid=True)
 p,_,_,_=locate('Video');p['imp'][0]['video']['ext']={'rewarded':1};add(label+'-obsolete-rewarded-advisory',s,payload=p,errors=[],warnings=[issue('openrtb.profile.equativ.obsolete_rewarded','imp[0].video.ext.rewarded')])
 # Optional/defaulted floor and bidder routing are not fabricated minima.
 add(label+'-defaulted-floor-zero',s,payload=r,errors=[],valid=True)
 for field in ['device','user']:
  p=clone(r);del p[field];add(label+'-missing-'+field,s,payload=p,errors=[issue('openrtb.profile.field_required',field)] if s else [])
 p=clone(r);p['user']={};add(label+'-empty-user',s,payload=p,errors=[issue('openrtb.profile.field_required','user.buyeruid')] if s else [])
 p=clone(r);del p['site']['publisher'];add(label+'-network-missing',s,payload=p,errors=[issue('openrtb.profile.equativ.network_id','ext.network_id')] if s else [])
 p['ext']={'network_id':0};add(label+'-network-alternate-zero',s,payload=p,errors=[],valid=True)
 p=clone(r);p['imp'][0]['banner']['w']=0;add(label+'-zero-width',s,payload=p,errors=[],warnings=[issue('openrtb.profile.equativ.zero_dimension','imp[0].banner.w')] if s else None)
 for count in [10,11]:
  p=clone(r);p['imp']=[dict(clone(p['imp'][0]),id=f'i{i}') for i in range(count)];add(label+f'-impressions-{count}',s,payload=p,errors=[issue('openrtb.profile.equativ.impression_limit','imp')] if s and count==11 else [])
  p=clone(b);p['seatbid'][0]['bid']=[dict(clone(p['seatbid'][0]['bid'][0]),id=f'b{i}') for i in range(count)];add(label+f'-bids-{count}',s,'response',p,errors=[issue('openrtb.profile.equativ.bid_limit','seatbid')] if s and count==11 else [])
 for plcmt in [1,5,9,10]:
  p,_,_,_=locate('Video');p['imp'][0]['video']['plcmt']=plcmt;add(label+f'-plcmt-{plcmt}',s,payload=p,errors=[issue('openrtb.profile.value_invalid','imp[0].video.plcmt')] if plcmt==10 else [],valid=plcmt!=10)
 p,_,_,_=locate('Video');p['imp'][0]['video'].pop('plcmt');add(label+'-video-placement-missing',s,payload=p,errors=[issue('openrtb.profile.equativ.video_placement','imp[0].video.plcmt')] if s else [])
 p['imp'][0]['video']['placement']=1;add(label+'-video-legacy-placement',s,payload=p,errors=[],valid=True)
 for size in [100,101]:
  p=clone(b);p['seatbid'][0]['bid'][0]['ext']={'dsa':{'behalf':'é'*size,'paid':'payer'}};add(label+f'-dsa-unicode-{size}',s,'response',p,errors=[] if size==100 else [issue('openrtb.profile.equativ.dsa_name_length','seatbid[0].bid[0].ext.dsa.behalf')])
 for dealtype in ['GuaranteedDeal','AuctionPackage','DirectDeal','PrivateAuction','OtherDeal']:
  p=clone(b);p['ext']={'dealtype':dealtype};add(label+'-dealtype-'+dealtype,s,'response',p,errors=[] if dealtype!='OtherDeal' else [issue('openrtb.profile.equativ.deal_type','ext.dealtype')])
 for tracking in ['https://tracker.example/pixel',['https://tracker.example/pixel'],[False]]:
  p=clone(b);p['ext']={'impression_tracking_url':tracking};add(label+'-tracking-'+str(tracking),s,'response',p,errors=[issue('openrtb.profile.field_type','ext.impression_tracking_url')] if tracking==[False] else [])
 for key in ['gdpr','consent','schain']:
  p=clone(r);obj='regs' if key=='gdpr' else 'user' if key=='consent' else 'source';p.setdefault(obj,{})
  value=0 if key=='gdpr' else '' if key=='consent' else {'ver':'1.0','complete':1,'nodes':[{'asi':'ssp.example','sid':'seller','hp':1}]}
  p[obj].setdefault('ext',{})[key]=clone(value);add(label+'-legacy-'+key,s,payload=p,errors=[],valid=True)
  p[obj][key]=clone(value)
  if key=='consent':p[obj][key]='example';p[obj]['ext'][key]='example'
  add(label+'-mainline-priority-'+key,s,payload=p,errors=[],warnings=[issue('openrtb.profile.equativ.mainline_precedence',obj+'.ext.'+key)])
 for obj,path,good,bad in [('Video','ext.orientation',2,3),('Device','ext.atts',3,4),('Site','ext.amp',1,2)]:
  for val in [good,bad]:
   p,o,m,base=locate(obj);setpath(o,path,val);add(label+'-'+path+'-'+str(val),s,m,p,errors=[] if val==good else [issue('openrtb.profile.value_invalid',base+'.'+path)])
# Supplied legacy supply chains receive the same required nested facts.
for supplier in [False,True]:
 for missing in ['complete','nodes','ver']:
  p=request();chain={'complete':1,'ver':'1.0','nodes':[{'asi':'ssp.example','sid':'seller','hp':1}]};del chain[missing];p['source']={'ext':{'schain':chain}}
  add(f'legacy-schain-{supplier}-missing-{missing}',supplier,payload=p,errors=[issue('openrtb.profile.field_required','source.ext.schain.'+missing)])
 p=request();p['source']={'ext':{'schain':{'complete':1,'ver':'1.0','nodes':[{'hp':1}]}}};add(f'legacy-schain-{supplier}-node-required',supplier,payload=p,errors=[issue('openrtb.profile.field_required','source.ext.schain.nodes[0].asi'),issue('openrtb.profile.field_required','source.ext.schain.nodes[0].sid')])
# Supplier operational limits and alternatives are separate source fixtures.
r=request();r['imp'].append(dict(clone(r['imp'][0]),id='i2',tagid='other'));add('supplier-mio-different-tags',True,payload=r,errors=[],warnings=[issue('openrtb.profile.equativ.mio_first_only','imp')])
for i in r['imp']:i.pop('tagid');add('supplier-mio-missing-tag-'+i['id'],True,payload=r,errors=[],warnings=[issue('openrtb.profile.equativ.mio_first_only','imp')])
r,_,_,_=locate('Video');r['imp'].append(dict(clone(r['imp'][0]),id='i2',tagid='other'));add('supplier-instream-separate-slots',True,payload=r,errors=[],valid=True)
r=request();r['site']['publisher']['id']=1234;add('supplier-integer-network-id',True,payload=r,errors=[],valid=True)
r=request();r['site']['publisher']['ext']={'directpay':True};r['app']={'bundle':'com.example','publisher':{'id':'1234','ext':{'directpay':False}}};add('supplier-directpay-once',True,payload=r,required=[issue('openrtb.profile.equativ.directpay_once','')])
(out/'cases.jsonl').write_text(''.join(json.dumps(c,separators=(',',':'),ensure_ascii=False)+'\n' for c in cases))
(out/'oracle.json').write_text(json.dumps({'count':len(cases),'extension_type_facts':len(facts),'sources':'docs/exchange-profiles/equativ-sources.json','approach':'Independent source-derived shape, boundary, mandatory, processing-limit and compatibility facts. Descriptor controls assert shape only. Full valid controls are explicitly marked. No expected results are copied from the validator.','synthetic':True},indent=2)+'\n')
(root/'docs/exchange-profiles/equativ-descriptors.json').write_text(json.dumps(facts,indent=2)+'\n')
print(len(cases),'cases',len(facts),'extension type facts')
