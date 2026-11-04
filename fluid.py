"""Map the solved routing to actual bidirectional fluid connectors and validate it."""
import json,pathlib,collections,gzip,base64
from solve import DIR
ROOT=pathlib.Path(__file__).parent
defs={v['Id']:v for v in json.loads(pathlib.Path(r'C:\Users\27773\playground\2026-11-03-shapez-throughput\full-defs.json').read_text()).values() if 'Tiles' in v and 'Id' in v}
source=json.loads((ROOT/'entries-owners.json').read_text()); owners=source['owners']
def point(p,e,mirror):
 x,y,z=p['x'],p['y'],p['z']
 if mirror:y=-y
 for _ in range(e['R']):x,y=-y,x
 return x+e['X'],y+e['Y'],z+e['L']
def geometry(e,fields):
 mirror=e['T'].endswith('Mirrored');d=defs[e['T'].removesuffix('Mirrored')]
 tiles={point(p,e,mirror) for p in d['Tiles']}
 ports=set()
 for field in fields:
  for p in d[field]:
   direction=((-p['Direction_L'] if mirror else p['Direction_L'])+e['R'])%4
   ports.add((point(p['Position_L'],e,mirror),direction))
 return tiles,ports

pipe_types=[k for k in defs if k.startswith('Pipe') and k.endswith('InternalVariant') and len(defs[k]['FluidJunctionIOs'])==2 and k!='PipeGateDefaultInternalVariant']
entries=[]
for old in source['entries']:
 if old['T'].startswith('BeltPort'):
  entries.append(dict(old,T=old['T'].replace('BeltPort','FluidPort')));continue
 expected=geometry(old,['BeltInputs','BeltOutputs']);chosen=None
 for t in pipe_types:
  for mirror in [False,True] if not defs[t]['SymmetricOnXAxis'] else [False]:
   for rot in range(4):
    e=dict(X=old['X'],Y=old['Y'],L=min(p[2] for p in expected[0]),R=rot,T=t+('Mirrored' if mirror else ''))
    if geometry(e,['FluidJunctionIOs'])==expected:chosen=e;break
   if chosen:break
  if chosen:break
 assert chosen,('no fluid equivalent',old,expected)
 entries.append(chosen)

occupied={};connectors={};senders={};receivers={};graph=collections.defaultdict(set)
for i,e in enumerate(entries):
 tiles,ports=geometry(e,['FluidJunctionIOs','FluidProviderConnectorIOs','FluidConsumerConnectorIOs'])
 for p in tiles:
  assert p not in occupied,('collision',p);occupied[p]=i
 for p in ports:
  assert p not in connectors;connectors[p]=i
 if e['T']=='FluidPortSenderInternalVariant':senders[((e['X'],e['Y'],e['L']),e['R'])]=i
 if e['T']=='FluidPortReceiverInternalVariant':receivers[((e['X'],e['Y'],e['L']),e['R'])]=i
for (p,d),i in connectors.items():
 dx,dy=DIR[d];peer=((p[0]+dx,p[1]+dy,p[2]),(d+2)%4)
 assert peer in connectors,('open pipe',i,entries[i],peer)
 j=connectors[peer];assert owners[i]==owners[j],('fluid crossover',i,j)
 graph[i].add(j);graph[j].add(i)
paired={}
for (p,d),i in receivers.items():
 if p[1]==2:continue
 dx,dy=DIR[d];candidates=[senders.get(((p[0]-dx*n,p[1]-dy*n,p[2]),d)) for n in range(5,1,-1)]
 candidates=[j for j in candidates if j is not None]
 assert candidates and owners[candidates[0]]==owners[i],('wrong jump',i,candidates)
 j=candidates[0];assert j not in paired;paired[j]=i;graph[i].add(j);graph[j].add(i)
assert len(paired)==47
components=[];seen=set()
for i in range(len(entries)):
 if i in seen:continue
 todo=[i];component=set()
 while todo:
  j=todo.pop()
  if j in component:continue
  component.add(j);todo.extend(graph[j]-component)
 seen.update(component)
 assert len({owners[j] for j in component})==1
 inlet=[j for j in component if entries[j]['T']=='FluidPortReceiverInternalVariant' and entries[j]['Y']==2]
 outlet=[j for j in component if entries[j]['T']=='FluidPortSenderInternalVariant' and entries[j]['Y']==17]
 assert len(inlet)==len(outlet)==1
 a,b=entries[inlet[0]],entries[outlet[0]]
 c=(a['X']+12)//20;k=(a['X']+12)%20
 assert b['L']==2-c and b['X']==8+20*(1-a['L'])+k
 components.append({'input':a,'output':b,'buildings':len(component)})
assert len(components)==36
bp=json.loads((ROOT/'transpose-1x3.json').read_text(encoding='utf8'));bp['BP']['Entries'][0]['B']['Entries']=entries
dest=ROOT/'fluid-transpose-1x3'
dest.with_suffix('.json').write_text(json.dumps(bp,ensure_ascii=False,indent=2),encoding='utf8')
dest.with_suffix('.spz2bp').write_text('SHAPEZ2-1-'+base64.b64encode(gzip.compress(json.dumps(bp,separators=(',',':')).encode(),mtime=0)).decode()+'$',encoding='utf8')
(ROOT/'fluid-validation.json').write_text(json.dumps({'buildings':len(entries),'occupied':len(occupied),'jump_pairs':len(paired),'components':components},indent=2))
print('PASS: 36 separate fluid networks, correct 36 source/destination pairs, 47 jump pairs, no overlaps or open connectors.')
print(collections.Counter(e['T'] for e in entries))
