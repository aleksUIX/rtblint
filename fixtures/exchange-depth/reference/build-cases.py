"""Publish independently selected rule-reference branches and scope controls."""
from pathlib import Path
import copy
import json

root = Path(__file__).resolve().parents[3]
out = Path(__file__).resolve().parent
cases = []
urls = {
 'dv360': 'https://developers.google.com/display-video/ortb-spec',
 'index-exchange': 'https://kb.indexexchange.com/dsps/open-rtb/list_of_supported_openrtb_bid_request_fields_dsp.htm',
 'index-exchange-seller': 'https://kb.indexexchange.com/publishers/openrtb_integration/list_of_supported_openrtb_bid_request_fields_for_sellers.htm',
 'digital-turbine': 'https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs',
}
def clone(x): return copy.deepcopy(x)
def req(profile):
 if profile == 'dv360': return {'id':'req','device':{'ua':'browser','ip':'192.0.2.1'},'user':{},'site':{'domain':'publisher.example'},'imp':[{'id':'1','banner':{'w':300,'h':250}}]}
 if profile == 'index-exchange-seller': return {'id':'req','tmax':100,'device':{'ip':'192.0.2.1'},'site':{'domain':'publisher.example'},'imp':[{'id':'1','banner':{'w':300,'h':250}}]}
 return {'id':'req','tmax':100,'ext':{},'device':{},'site':{'id':'site','publisher':{'id':'pub'}},'imp':[{'id':'1','secure':1,'banner':{'w':300,'h':250,'topframe':1}}]}
def resp(): return {'id':'req','seatbid':[{'seat':'buyer','bid':[{'id':'bid','impid':'1','price':1.0,'adm':'<div>Ad</div>','adomain':['advertiser.example'],'mtype':1}]}]}
def issue(id,path,severity='error'): return {'id':id,'path':path,'severity':severity}
def add(id,profile,direction,payload,expected=None,forbidden=None,valid=None,request=None,version='2.6-202606'):
 c={'id':id,'profile':profile,'direction':direction,'input':clone(payload),'version':version,'expected':expected or [],'forbidden':forbidden or [],'source':urls.get(profile,'https://github.com/aleksUIX/rtblint/blob/main/crates/rtblint-core/src/profile.rs')}
 if direction!='request' and profile.startswith('index-exchange'): c['source']='https://kb.indexexchange.com/dsps/open-rtb/list_of_supported_openrtb_bid_response_fields_dsp.htm'
 if valid is not None:c['valid']=valid
 if request is not None:c['request']=clone(request)
 cases.append(c)
for profile in ['dv360','index-exchange','index-exchange-seller']:
 add(profile+'-reference-control',profile,'request',req(profile),valid=True)
add('dv360-response-reference-control','dv360','response',resp(),valid=True)
r=req('dv360');del r['site'];add('dv360-inventory-absent','dv360','request',r,[issue('openrtb.profile.dv360.inventory_required','site')],valid=False)
r=req('dv360');del r['device']['ip'];add('dv360-address-absent','dv360','request',r,[issue('openrtb.profile.dv360.ip_required','device.ip')],valid=False)
r=req('dv360');r['regs']={'gdpr':0,'ext':{'gdpr':0}};add('dv360-privacy-locations','dv360','request',r,[issue('openrtb.profile.dv360.privacy_locations','regs.ext.gdpr','warning')],valid=True)
add('dv360-schain-account-advisory','dv360','request',req('dv360'),[issue('openrtb.profile.dv360.schain_expected','source.schain','warning')],valid=True)
r=req('dv360');r['test']=0;add('dv360-ignored-test','dv360','request',r,[issue('openrtb.profile.dv360.ignored_field','test','warning')],valid=True)
for empty in [True,False]:
 r=req('dv360');r['imp'][0].pop('banner');r['imp'][0]['video']={'mimes':['video/mp4'],'protocols':[] if empty else [6]}
 add('dv360-protocol-list-'+str(empty),'dv360','request',r,[issue('openrtb.profile.dv360.protocols_required','imp[0].video.protocols')] if empty else [],[issue('openrtb.profile.dv360.protocols_required','imp[0].video.protocols')] if not empty else [],valid=not empty)
b=resp();del b['seatbid'][0]['bid'][0]['adm'];add('dv360-banner-markup-absent','dv360','response',b,[issue('openrtb.profile.dv360.banner_markup','seatbid[0].bid[0].adm')],valid=False)
r=req('index-exchange');r['cur']=['EUR'];add('index-nonusd-currency','index-exchange','request',r,[issue('openrtb.profile.index.currency','cur[0]')],valid=False)
r=req('index-exchange-seller');r['site']={};add('index-site-identity-absent','index-exchange-seller','request',r,[issue('openrtb.profile.index.site_identity','site.domain')],valid=False)
for protocols in [[],[1],[2,3,5,6,7,8,11,12,13,14]]:
 r=req('index-exchange-seller');r['imp'][0].pop('banner');r['imp'][0]['video']={'mimes':['video/mp4'],'protocols':protocols,'minduration':1,'maxduration':30,'w':300,'h':250}
 path='imp[0].video.protocols' if not protocols else 'imp[0].video.protocols[0]'
 add('index-video-protocol-'+str(protocols),'index-exchange-seller','request',r,[issue('openrtb.profile.index.video_protocol',path)] if len(protocols)<2 else [],valid=len(protocols)>2)
for supported in [False,True]:
 r=req('index-exchange-seller');r['imp'][0].pop('banner');r['imp'][0]['video']={'mimes':['application/javascript'],'protocols':[6],'minduration':1,'maxduration':30,'w':300,'h':250}
 if supported:r['imp'][0]['video']['api']=[2]
 add('index-vpaid-'+str(supported),'index-exchange-seller','request',r,[] if supported else [issue('openrtb.profile.index.vpaid_api','imp[0].video.api')],valid=supported)
for disc in [0.0,0.01,1.0,1.01]:
 r=req('index-exchange');r['imp'][0]['pmp']={'deals':[{'id':'deal','at':1,'ext':{'disc':disc}}]}
 add('index-deal-discount-'+str(disc),'index-exchange','request',r,[issue('openrtb.profile.index.deal_discount','imp[0].pmp.deals[0].ext.disc')] if disc in [0.0,1.01] else [],valid=disc in [0.01,1.0])
b=resp();b['seatbid'][0]['bid'][0]['adomain']=[];add('index-adomain-account-advisory','index-exchange','response',b,[issue('openrtb.profile.index.adomain_expected','seatbid[0].bid[0].adomain','warning')],valid=True)
b=resp();b['seatbid'][0].pop('seat');add('index-seat-account-advisory','index-exchange','response',b,[issue('openrtb.profile.index.seat_expected','seatbid[0].seat','warning')],valid=True)
for n in [128,129]:
 b=resp();b['seatbid'][0]['bid'][0]['adomain']=['a'*(n-8)+'.example'];add('index-domain-length-'+str(n),'index-exchange','response',b,[] if n==128 else [issue('openrtb.profile.index.adomain_length','seatbid[0].bid[0].adomain[0]')],valid=n==128)
for n in [100,101]:
 b=resp();b['seatbid'][0]['bid'][0]['ext']={'dsa':{'paid':'é'*n}};add('index-dsa-length-'+str(n),'index-exchange','response',b,[] if n==100 else [issue('openrtb.profile.index.dsa_length','seatbid[0].bid[0].ext.dsa.paid')],valid=n==100)
native={'ver':'1.2','assets':[{'id':1,'title':{'text':'Example'}}],'link':{'url':'https://advertiser.example'}}
for wrapped in [False,True]:
 b=resp();bid=b['seatbid'][0]['bid'][0];bid['mtype']=4;bid['adm']=json.dumps({'native':native} if wrapped else native)
 add('index-native-wrapper-'+str(wrapped),'index-exchange','response',b,[] if wrapped else [issue('openrtb.profile.index.native_wrapper','seatbid[0].bid[0].adm')],valid=wrapped)
for matched in [False,True]:
 b=resp();bid=b['seatbid'][0]['bid'][0];bid['bundle']='123';bid['ext']={'skadn':{'version':'2.2','network':'abcd123456.skadnetwork','campaign':'1','itunesitem':'123' if matched else '456','sourceapp':'789','nonce':'00000000-0000-4000-8000-000000000001','timestamp':'1700000000000','signature':'example'}}
 add('index-skad-bundle-'+str(matched),'index-exchange','pair',b,[] if matched else [issue('openrtb.profile.index.skad_bundle','seatbid[0].bid[0].ext.skadn.itunesitem')],[] if not matched else [issue('openrtb.profile.index.skad_bundle','seatbid[0].bid[0].ext.skadn.itunesitem')],request=req('index-exchange'))
for profile,direction in [('applovin-alx','request'),('index-exchange-seller','response'),('commerce-grid','response'),('sovrn','response')]:
 add('scope-'+profile+'-'+direction,profile,direction,req('dv360') if direction=='request' else resp(),[issue('openrtb.profile.scope.unsupported',None,'warning')],valid=True)
dt=json.loads((root/'fixtures/exchange-depth/dt-depth/dt-supported-complete-request.json').read_text())
add('scope-digital-turbine-request-supported','digital-turbine','request',dt,forbidden=[issue('openrtb.profile.scope.unsupported',None,'warning')],valid=True)
r={'openrtb':{'ver':'3.0','domainspec':'adcom','domainver':'1.0','request':{'id':'req','item':[{'id':'1','spec':{'placement':{'tagid':'placement','display':{'w':300,'h':250}}}}]}}}
add('scope-google-ab-openrtb3','google-ab','request',r,[issue('openrtb.profile.scope.unsupported',None,'warning')],valid=True,version='3.0')
(out/'cases.json').write_text(json.dumps({'approach':'Expected IDs, paths and boundaries independently selected from published contracts. Account exemptions use warnings. Positive validity is asserted where complete canonical captures permit it. Scoped unsupported contracts preserve specification validation.','cases':cases},indent=2,ensure_ascii=False)+'\n')
print(len(cases),'reference and scope cases')
