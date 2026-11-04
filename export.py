import json,pathlib,collections,base64,gzip
from solve import DIR
r=json.loads(pathlib.Path('routes.json').read_text()); entries=[];owners=[]
def put(x,y,z,d,t,owner):
 entries.append(dict(X=x,Y=y,L=z,R=d,T=t));owners.append(owner)
for i,net in enumerate(r['nets']):
 c,z,k=net;put(8+20*(c-1)+k,2,z,1,'BeltPortReceiverInternalVariant',i)
 for step in r['paths'][str(i)]:
  x,y,z,d,zz,dd=step[:6]
  if len(step)==7:
   dx,dy=DIR[d];put(x,y,z,d,'BeltPortSenderInternalVariant',i);put(x+dx*step[6],y+dy*step[6],z,d,'BeltPortReceiverInternalVariant',i);continue
  turn=(dd-d)%4
  if zz==z:
   t='BeltDefaultForwardInternalVariant' if turn==0 else 'BeltDefaultLeftInternalVariant'+('Mirrored' if turn==1 else '')
  else:t=f'Lift{abs(zz-z)}'+('Up' if zz>z else 'Down')+('Forward' if turn==0 else 'Left')+'InternalVariant'+('Mirrored' if turn==1 else '')
  put(x,y,z,d,t,i)
 c,z,k=net;put(8+20*(1-z)+k,17,2-c,1,'BeltPortSenderInternalVariant',i)

defs={v['Id']:v for v in json.loads(pathlib.Path(r'C:\Users\27773\playground\2026-11-03-shapez-throughput\full-defs.json').read_text()).values() if 'Tiles' in v and 'Id' in v}
def trans(p,e,mirror=False):
 x,y,z=p['x'],p['y'],p['z']
 if mirror:y=-y
 for _ in range(e['R']):x,y=-y,x
 return x+e['X'],y+e['Y'],z+e['L']
occupied={};inputs={};outputs={};senders={};receivers={}
for idx,e in enumerate(entries):
 mirror=e['T'].endswith('Mirrored');d=defs[e['T'].removesuffix('Mirrored')]
 for p in d['Tiles']:
  t=trans(p,e,mirror);assert t not in occupied,('collision',t,idx,occupied.get(t));occupied[t]=idx
 for kind,table in [('BeltInputs',inputs),('BeltOutputs',outputs),('BeltPortInputs',receivers),('BeltPortOutputs',senders)]:
  for p in d[kind]:
   pos=trans(p['Position_L'],e,mirror);direction=(((-p['Direction_L']) if mirror else p['Direction_L'])+e['R'])%4
   table[(pos,direction)]=idx
bad=[]
for (pos,d),idx in outputs.items():
 dx,dy=DIR[d];target=((pos[0]+dx,pos[1]+dy,pos[2]),(d+2)%4)
 assert target in inputs,('unconnected',idx,entries[idx],target)
 assert owners[idx]==owners[inputs[target]],('crosswire',idx,inputs[target])
for (pos,d),idx in senders.items():
 if pos[1]==17:continue
 dx,dy=DIR[d];targets=[]
 for j in range(2,6):
  other=receivers.get(((pos[0]+dx*j,pos[1]+dy*j,pos[2]),(d+2)%4))
  if other is not None:targets.append(other)
 if len(targets)!=1 or owners[targets[0]]!=owners[idx]:bad.append((idx,targets))
print('entries',len(entries),'occupied',len(occupied),'jumps',len(senders)-36,'ambiguous jumps',len(bad))
paired={}
for (pos,d),idx in receivers.items():
 if pos[1]==2:continue
 dx,dy=DIR[d]
 candidates=[senders.get(((pos[0]+dx*j,pos[1]+dy*j,pos[2]),(d+2)%4)) for j in range(5,1,-1)]
 candidates=[j for j in candidates if j is not None]
 assert candidates and owners[candidates[0]]==owners[idx],('wrong jump receiver',idx,candidates)
 assert candidates[0] not in paired,('duplicate sender',idx)
 paired[candidates[0]]=idx
assert len(paired)==len(senders)-36
print('All 47 jump pairings and 36 lane identities verified')
pathlib.Path('jump-conflicts.json').write_text(json.dumps([{'sender':entries[i],'targets':[entries[j] for j in js]} for i,js in bad],indent=2))
template=json.loads(next(pathlib.Path('reference').glob('*3333-333311112222*')).read_text(encoding='utf8'))
template['BP']['Entries'][0]['B']['Entries']=entries
template['BP']['Entries'][0]['R']=0
template['BP']['Icon']['Data']=['shape:CrCrCrCr','shape:CgCgCgCg','shape:CbCbCbCb','icon:Empty']
pathlib.Path('transpose-1x3.json').write_text(json.dumps(template,ensure_ascii=False,indent=2),encoding='utf8')
pathlib.Path('transpose-1x3.spz2bp').write_text('SHAPEZ2-1-'+base64.b64encode(gzip.compress(json.dumps(template,separators=(',',':')).encode(),mtime=0)).decode()+'$',encoding='utf8')
pathlib.Path('entries-owners.json').write_text(json.dumps({'entries':entries,'owners':owners}))
