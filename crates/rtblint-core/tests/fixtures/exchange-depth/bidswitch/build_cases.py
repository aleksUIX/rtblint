import json,copy
from pathlib import Path
root=Path(__file__).resolve().parents[6]
out=root/'crates/rtblint-core/tests/fixtures/exchange-depth/bidswitch';out.mkdir(parents=True,exist_ok=True)
def clone(x):return copy.deepcopy(x)
nr={'ver':'1.2','assets':[{'id':1,'required':1,'title':{'len':40}}]}
nresp={'ver':'1.2','assets':[{'id':1,'title':{'text':'Example'}}],'link':{'url':'https://advertiser.example'}}
def request(s=False,media='banner'):
 imp={'id':'i','banner':{'w':300,'h':250},'secure':1}
 if media!='banner':
  imp.pop('banner')
  imp[media]=({'mimes':['video/mp4'],'minduration':1,'maxduration':30,'protocols':[6,8],'ext':{'vast_url_rq':1}} if media=='video' else {'mimes':['audio/mpeg'],'protocols':[9,10],'ext':{'format':1}} if media=='audio' else {'request_native' if s else 'request':clone(nr)})
 r={'id':'r','imp':[imp],'device':{'ip':'203.0.113.2','geo':{'country':'USA'}},'site':{'id':'site','domain':'publisher.example','publisher':{'id':'pub'}},'user':{},'tmax':120,'cur':['USD'],'ext':{'ssp':'example','media_src':'example'}}
 return r
def response(s=False):return {'id':'r','seatbid':[{'seat':'buyer','bid':[{'id':'b','impid':'i','price':1.2,'burl':'http://buyer.example/bill?p=${AUCTION_PRICE}','adomain':['advertiser.example'],'crid':'creative','adm':'<div>Example</div>','iurl':'https://buyer.example/preview'}]}],'ext':{'protocol':'5.7'}}
cases=[]
def issue(id,path):return {'id':id,'path':path}
def add(name,s=False,mode='request',payload=None,req=None,errors=None,required=None,forbidden=None,valid=None,source=None,warnings=None):
 c={'name':name,'profile':'bidswitch-supplier' if s else 'bidswitch','mode':mode,'payload':clone(payload)}
 if req is not None:c['request']=clone(req)
 if errors is not None:c['expected_profile_errors']=errors
 if required is not None:c['required_issues']=required
 if forbidden is not None:c['forbidden_issues']=forbidden
 if valid is not None:c['valid']=valid
 if source:c['source']=source
 if warnings is not None:c['expected_profile_warnings']=warnings
 cases.append(c)
def locate(obj,s):
 r=request(s);b=response(s);mode='request';path=''
 if obj=='BidRequest':target=r
 elif obj=='Imp':target=r['imp'][0];path='imp[0]'
 elif obj in ['Video','Audio','Native','Banner']:
  r=request(s,obj.lower());target=r['imp'][0][obj.lower()];path='imp[0].'+obj.lower()
 elif obj=='BidResponse':target=b;mode='response'
 elif obj=='Bid':target=b['seatbid'][0]['bid'][0];path='seatbid[0].bid[0]';mode='response'
 elif obj=='Device':target=r['device'];path='device'
 elif obj=='User':target=r['user'];path='user'
 elif obj=='App':r.pop('site');r['app']={'id':'app','publisher':{'id':'pub'}};target=r['app'];path='app'
 elif obj=='Site':target=r['site'];path='site'
 elif obj=='Source':r['source']={};target=r['source'];path='source'
 elif obj=='Data':r['user']['data']=[{'name':'provider','segment':[{'id':'one','name':'one'}]}];target=r['user']['data'][0];path='user.data[0]'
 elif obj=='Deal':r['imp'][0]['pmp']={'deals':[{'id':'deal'}]};target=r['imp'][0]['pmp']['deals'][0];path='imp[0].pmp.deals[0]'
 elif obj=='Regs':r['regs']={};target=r['regs'];path='regs'
 else:raise ValueError(obj)
 return (b if mode=='response' else r),target,mode,path
def setpath(obj,path,value):
 parts=path.split('.')
 for p in parts[:-1]:
  if p.endswith('[]'):
   p=p[:-2]
   if p not in obj or not obj[p]:
    obj[p]=[{'id':'i','impid':'i','igbuyer':[{'origin':'https://buyer.example'}],'ignurl':'https://buyer.example/group'}] if p=='igbid' else [{'origin':'https://buyer.example'}] if p=='igbuyer' else [{}]
   obj=obj[p][0]
  else:
   if p not in obj or not isinstance(obj[p],dict):obj[p]={}
   obj=obj[p]
 obj[parts[-1]]=clone(value)
def valuefor(d):
 kind=d['shape'];p=d['path']
 if kind=='String':
  if p=='ext.protocol':return '5.7'
  if p.endswith('burl'):return 'https://buyer.example/p?price=${AUCTION_PRICE}'
  if p.endswith('ver') or p.endswith('version'):return '1.2' if 'native' in p or p=='ver' else '2.0'
  return 'example'
 if kind=='Integer':return 1
 if kind=='Number':return 1.25
 if kind=='Flag':return True
 if kind=='Object':
  if p in ['request','request_native']:return nr
  if p in ['ext.native','adm_native']:return nresp
  return {}
 return []
facts=json.load(open(root/'docs/exchange-profiles/bidswitch-descriptors.json'))['descriptors']
for i,d in enumerate(facts):
 s=d['supplier'];payload,target,mode,path=locate(d['object'],s)
 abs_path=(path+'.' if path else '')+d['path'].replace('[]','[0]')
 ident=issue('openrtb.profile.field_type',abs_path)
 for suffix,v,good in [('shape',valuefor(d),True),('malformed',False if d['shape']!='Flag' else 'yes',False),('null',None,True)]:
  p,t,m,base=locate(d['object'],s);setpath(t,d['path'],v)
  add(f'descriptor-{i:03}-{suffix}',s,m,p,required=None if good else [ident],forbidden=[ident] if good else None,source=d['source'])
for s in (False,True):
 label='supplier' if s else 'buyer'
 r=request(s);b=response(s)
 add(label+'-control',s,payload=r,errors=[],valid=True)
 add(label+'-banner-response-control',s,'pair',b,r,errors=[],valid=True)
 add(label+'-privacy-empty-user',s,payload=r,errors=[],valid=True)
 p=clone(r);del p['site'];add(label+'-inventory-absent',s,payload=p,errors=[issue('openrtb.profile.bidswitch.inventory_required','')])
 p=clone(r);p['imp'][0]['pmp']={};add(label+'-pmp-missing-deals',s,payload=p,errors=[issue('openrtb.profile.field_required','imp[0].pmp.deals')])
 p['imp'][0]['pmp']['deals']=[{'id':'deal'}];add(label+'-pmp-deals-control',s,payload=p,errors=[],valid=True)
 for field in (['device'] if s else ['device','user','tmax','cur','ext.ssp','ext.media_src']):
  p=clone(r);parts=field.split('.');o=p
  for part in parts[:-1]:o=o[part]
  del o[parts[-1]]
  add(label+'-missing-'+field,s,payload=p,required=[issue('openrtb.profile.field_required',field)])
 if s:
  q=request(True,'native');q['imp'][0]['native']['request_native']['ver']='1.1';add('supplier-native-version-unsupported',True,payload=q,errors=[issue('openrtb.profile.bidswitch.native_version','imp[0].native.request_native.ver')])
  p=clone(r)
  for f in ['user','tmax','cur','ext']:del p[f]
  add('supplier-optional-root-fields',True,payload=p,errors=[],valid=True)
 else:
  p=clone(b);del p['seatbid'][0]['bid'][0]['crid'];add('buyer-creative-id-advisory',False,'response',p,errors=[],valid=True,warnings=[issue('openrtb.profile.bidswitch.creative_id_recommended','seatbid[0].bid[0]')])
  p=clone(b);p['ext']['protocol']='3.0';add('buyer-markup-version-unsupported',False,'response',p,errors=[issue('openrtb.profile.bidswitch.markup_version','seatbid[0].bid[0].adm')])
  q=request(False,'video');q['imp'][0]['video']['ext']['vast_url_rq']=0;p=clone(b);bid=p['seatbid'][0]['bid'][0];bid['protocol']=6;del bid['adm'];add('buyer-video-fallback-url-absent',False,'pair',p,q,errors=[issue('openrtb.profile.bidswitch.video_url','seatbid[0].bid[0].ext.vast_url')])
  q=request(False,'audio');q['imp'][0]['audio']['ext']['format']=3;p=clone(b);del p['seatbid'][0]['bid'][0]['adm'];add('buyer-audio-either-url-absent',False,'pair',p,q,errors=[issue('openrtb.profile.bidswitch.audio_url','seatbid[0].bid[0].ext')])
  p=clone(r);del p['device']['geo'];add('buyer-geo-recommended',payload=p,errors=[],valid=True)
 p=clone(r);p['wseat']=['buyer'];p['bseat']=['buyer'];add(label+'-allowlist-precedence',s,'pair',b,p,errors=[],valid=True)
 p=clone(r);del p['site'];p['dooh']={'id':'screen','publisher':{'id':'pub'}};add(label+'-dooh-control',s,payload=p,errors=[],valid=True)
 p=clone(r);p['dooh']={'publisher':{'id':'pub'}};add(label+'-dooh-site-conflict',s,payload=p,required=[issue('openrtb.profile.bidswitch.inventory_conflict','dooh')])
 p=clone(b);p['seatbid'].append(clone(p['seatbid'][0]));p['seatbid'][1]['bid'][0]['id']='b2';add(label+'-duplicate-seat',s,'response',p,required=[issue('openrtb.profile.bidswitch.seat_unique','seatbid[1].seat')])
 p=clone(b);del p['seatbid'][0]['seat'];add(label+'-account-dependent-seat-absent',s,'pair',p,r,errors=[],valid=True)
 if s:
  q=clone(r);q['wseat']=['buyer'];add('supplier-seat-required-under-wseat',True,'pair',p,q,errors=[issue('openrtb.profile.field_required','seatbid[0].seat')])
 for media in ['video','audio','native']:
  q=request(s,media);p=response(s);bid=p['seatbid'][0]['bid'][0];bid.pop('adm');bid.pop('iurl')
  if media=='native':bid['adm_native' if s else 'ext']=(clone(nresp) if s else {'native':clone(nresp)})
  elif s:bid['adm']='<VAST version="3.0"></VAST>'
  elif media=='video':bid['ext']={'vast_url':'https://buyer.example/vast'}
  else:bid['ext']={'daast_url':'https://buyer.example/daast'}
  if media=='video':bid['protocol']=6
  add(label+'-'+media+'-control',s,'pair',p,q,errors=[],valid=True)
  missing=('adm_native' if s else 'ext.native') if media=='native' else 'adm' if s else ('ext.vast_url' if media=='video' else 'ext.daast_url')
  invalid=clone(p);o=invalid['seatbid'][0]['bid'][0];parts=missing.split('.')
  for f in parts[:-1]:o=o[f]
  del o[parts[-1]]
  add(label+'-'+media+'-missing-markup',s,'pair',invalid,q,required=[issue('openrtb.profile.field_required','seatbid[0].bid[0].'+missing)])
 # Explicit source-derived length boundaries.
 for size in [50,51]:
  p=clone(r);p['user']['id']='a'*size;expected=[] if size==50 else [issue('openrtb.profile.bidswitch.user_id_length','user.id')]
  add(label+f'-user-length-{size}',s,payload=p,errors=expected)
 for size in [100,101]:
  p=clone(b);p['seatbid'][0]['bid'][0]['ext']={'dsa':{'behalf':'b'*size,'paid':'payer'}}
  add(label+f'-dsa-length-{size}',s,'response',p,errors=[] if size==100 else [issue('openrtb.profile.bidswitch.dsa_name_length','seatbid[0].bid[0].ext.dsa.behalf')])
 for ascii_ in [True,False]:
  p=clone(b);p['seatbid'][0]['bid'][0]['adomain']=['xn--bcher-kva.example' if ascii_ else 'bücher.example'];add(label+f'-domain-{ascii_}',s,'response',p,errors=[] if ascii_ else [issue('openrtb.profile.bidswitch.domain_ascii','seatbid[0].bid[0].adomain[0]')])
# Buyer-specific response rules.
b=response();r=request()
p=clone(b);p['seatbid'][0]['bid'][0]['burl']='https://buyer.example/bill';add('buyer-billing-macro-required',mode='response',payload=p,errors=[issue('openrtb.profile.bidswitch.billing_macro','seatbid[0].bid[0].burl')])
for count in [1,2]:
 p=clone(b);p['seatbid'][0]['bid'][0]['adm']='${AUCTION_PRICE}'*count;add(f'buyer-adm-price-macro-{count}',mode='response',payload=p,errors=[] if count==1 else [issue('openrtb.profile.bidswitch.impression_macro','seatbid[0].bid[0].adm')])
p=clone(b);p['seatbid'][0]['bid'][0].update(adm='${AUCTION_PRICE}',nurl='https://buyer.example/win?p=${AUCTION_PRICE}');add('buyer-adm-and-nurl-price-conflict',mode='response',payload=p,errors=[issue('openrtb.profile.bidswitch.impression_macro','seatbid[0].bid[0].adm')])
for field in ['vast_url','daast_url']:
 p=clone(b);p['seatbid'][0]['bid'][0]['ext']={field:'https://buyer.example/doc?p=${AUCTION_PRICE}'};add('buyer-'+field+'-price-forbidden',mode='response',payload=p,errors=[issue('openrtb.profile.bidswitch.markup_macro','seatbid[0].bid[0].ext.'+field)])
for count in [2,3]:
 p=clone(b);base=p['seatbid'][0]['bid'][0];p['seatbid'][0]['bid']=[dict(clone(base),id=f'b{i}') for i in range(count)];add(f'buyer-bids-per-slot-{count}',mode='response',payload=p,errors=[] if count==2 else [issue('openrtb.profile.bidswitch.bid_count','seatbid[0].bid[2].impid')])
for s2s in [0,1]:
 q=clone(r);q['imp'][0]['ext']={'s2s_nurl':s2s};p=clone(b);p['seatbid'][0]['bid'][0]['adm']='${AUCTION_PRICE}';add(f'buyer-s2s-adm-{s2s}',mode='pair',payload=p,req=q,errors=[] if s2s==0 else [issue('openrtb.profile.bidswitch.s2s_markup_macro','seatbid[0].bid[0].adm')])
 p=clone(b);p['seatbid'][0]['bid'][0]['nurl']='http://buyer.example/win';add(f'buyer-http-notice-s2s-{s2s}',mode='pair',payload=p,req=q,errors=[] if s2s else [issue('openrtb.profile.bidswitch.secure_notification','seatbid[0].bid[0].nurl')],valid=bool(s2s))
for fmt in [1,2,3]:
 q=request(False,'audio');q['imp'][0]['audio']['ext']['format']=fmt
 p=clone(b);p['seatbid'][0]['bid'][0]['ext']={'vast_url':'https://buyer.example/vast'}
 add(f'buyer-audio-format-{fmt}-vast',mode='pair',payload=p,req=q,errors=[issue('openrtb.profile.field_required','seatbid[0].bid[0].ext.daast_url')] if fmt==1 else [])
for sup,fields in [('rubicon',['cid']),('nexage',['cid']),('mopub',['cid','cat']),('smaato',['cid','cat']),('pubmatic',['w','h']),('yieldone',['w','h','cat']),('adscale',['ext.advertiser_name','ext.agency_name']),('centro',['ext.advertiser_name']),('brx',['ext.advertiser_name']),('fyber',['bundle']),('trustx',['nurl'])]:
 q=clone(r);q['ext']['ssp']=sup;add('buyer-supplier-'+sup+'-requirements',mode='pair',payload=b,req=q,errors=[issue('openrtb.profile.field_required','seatbid[0].bid[0].'+f) for f in fields])
# Primary macro reference distinguishes buyer and supplier click tokens.
for sup in [False,True]:
 token='${CLICK_URL_ENC}' if sup else '${CLICK_URL:URLENCODE}'
 for count in [1,2]:
  p=response(sup);p['seatbid'][0]['bid'][0]['adm']=token*count
  add(f'click-macro-{sup}-{count}',sup,'response',p,errors=[] if count==1 else [issue('openrtb.profile.bidswitch.click_macro','seatbid[0].bid[0].adm')])
for requested in [0,1]:
 q=request();q['ext']['clktrkrq']=requested;p=response()
 add(f'buyer-banner-click-required-{requested}',mode='pair',payload=p,req=q,errors=[] if requested==0 else [issue('openrtb.profile.bidswitch.click_macro','seatbid[0].bid[0].adm')])
for sup in [False,True]:
 p={'id':'r','seatbid':[]}
 if sup:p['ext']={}
 add(f'no-bid-control-{sup}',sup,'response',p,errors=[],valid=True)
# Interest-group only response can omit classical seats; nested buyer signals are arbitrary JSON.
for s in [False,True]:
 g={'id':'i','igbuyer':[{'origin':'https://buyer.example','buyerdata':{'any':[1,True,None]}}]}
 if s:g.update(impid='i',ignurl='https://buyer.example/group')
 p={'id':'r','ext':{'protocol':'5.7','igbid':[g]}};add(('supplier' if s else 'buyer')+'-ig-only',s,'response',p,errors=[],valid=True)
 p=clone(p);del p['ext']['igbid'][0]['igbuyer'][0]['origin'];add(('supplier' if s else 'buyer')+'-ig-origin-required',s,'response',p,errors=[issue('openrtb.profile.field_required','ext.igbid[0].igbuyer[0].origin')])
# Explicit enum boundaries are separate prose oracles.
for obj,path,good,bad in [('Imp','ext.notification_type',3,4),('Imp','ext.ae',1,2),('Deal','ext.deal_type',4,5),('Deal','ext.guaranteed',1,2),('Bid','ext.at1',1,0)]:
 for sup in [False,True] if obj!='Bid' else [False]:
  for valid_value,value in [(True,good),(False,bad)]:
   p,t,mode,base=locate(obj,sup);setpath(t,path,value);target=(base+'.' if base else '')+path
   add(f'{obj}-{path}-enum-{sup}-{value}',sup,mode,p,errors=[] if valid_value else [issue('openrtb.profile.bidswitch.value_invalid',target)])
# Nullable optional members are accepted at the type layer; whole payload validity remains independent.
# Impression media are supplied objects. A malformed competing container cannot
# establish the sole media format, even if the banner object itself is well formed.
for s in (False,True):
 label='supplier' if s else 'buyer'
 q=request(s);p=response(s);bid=p['seatbid'][0]['bid'][0];del bid['adm'];del bid['iurl']
 if not s:q['ext']['clktrkrq']=1
 contextual=[issue('openrtb.profile.field_required','seatbid[0].bid[0].adm'),issue('openrtb.profile.field_required','seatbid[0].bid[0].iurl')]
 if not s:contextual.append(issue('openrtb.profile.bidswitch.click_macro','seatbid[0].bid[0].adm'))
 add(label+'-sole-banner-missing-markup-context',s,'pair',p,q,errors=contextual,valid=False)
 for field in ['video','audio','native','banner']:
  for kind,value in [('boolean',False),('string','unknown'),('array',[]),('null',None)]:
   malformed=clone(q);malformed['imp'][0][field]=value
   shape=issue('openrtb.type.mismatch','imp[0].'+field)
   add(f'{label}-malformed-{field}-{kind}-standalone',s,payload=malformed,required=[shape],valid=False,source='https://github.com/InteractiveAdvertisingBureau/openrtb2.x/blob/main/2.6.md#32-object-imp')
   add(f'{label}-malformed-{field}-{kind}-media-context-defers',s,'pair',p,malformed,errors=[],forbidden=contextual,source='Pinned BidSwitch media-specific response conditions require a known impression media object.')
 multi=clone(q);multi['imp'][0]['video']=request(s,'video')['imp'][0]['video']
 add(label+'-multiple-well-formed-media-context-defers',s,'pair',p,multi,errors=[],forbidden=contextual)
(out/'cases.jsonl').write_text(''.join(json.dumps(c,separators=(',',':'),ensure_ascii=False)+'\n' for c in cases))
(out/'oracle.json').write_text(json.dumps({'count':len(cases),'type_facts':len(facts),'source_inventory':'docs/exchange-profiles/bidswitch-sources.json','approach':'All field name and type expectations come from public vendor tables. Rule conditions and directional boundaries are independently selected from prose. Descriptor shape controls assert type acceptance only; valid:true controls assert full payload validity. Negative cases never learn expectations from validator output.','synthetic':True},indent=2)+'\n')
print(len(cases),'cases')
