import heapq, json, random, time, pathlib, collections

DIR=((1,0),(0,1),(-1,0),(0,-1))
WIDTH=54; XMIN=-17; XMAX=36; YMIN=3; YMAX=16
def solve(seed=0, rounds=120):
 rng=random.Random(seed)
 nets=[(c,z,k) for c in range(3) for z in range(3) for k in range(4)]
 terminals={}
 for i,(c,z,k) in enumerate(nets):
  terminals[(8+20*(c-1)+k,3,z)]=i
  terminals[(8+20*(2-z-1)+k,16,2-c)]=i
 occ=collections.Counter();hist=collections.Counter();paths={}
 def route(i,pres):
  c,z,k=nets[i];start=(8+20*(c-1)+k,3,z,1);goal=(8+20*(1-z)+k,17,2-c,1)
  def h(s):return abs(s[0]-goal[0])+abs(s[1]-goal[1])+abs(s[2]-goal[2])*.3
  q=[(h(start),0,start)];dist={start:0};prev={}
  while q:
   _,g,s=heapq.heappop(q)
   if g!=dist.get(s):continue
   if s==goal:
    path=[]
    while s!=start:
     old,step=prev[s];path.append(step);s=old
    return path[::-1]
   x,y,z,d=s
   if not (XMIN<=x<=XMAX and YMIN<=y<=YMAX):continue
   # A sender and catcher bridge up to four intervening cells.
   if y not in (3,16):
    dx,dy=DIR[d]
    for jump in range(2,6):
     rx,ry=x+dx*jump,y+dy*jump
     if not(XMIN<=rx<=XMAX and YMIN<ry<YMAX):continue
     used=((x,y,z),(rx,ry,z))
     if any(t in terminals and terminals[t]!=i for t in used):continue
     n=(rx+dx,ry+dy,z,d)
     cost=g+jump+1+.3+sum(hist[t]+pres*occ[t] for t in used)
     if cost<dist.get(n,1e100):
      dist[n]=cost;prev[n]=(s,(x,y,z,d,z,d,jump));heapq.heappush(q,(cost+h(n),cost,n))
   for zz in range(3):
    cells=tuple((x,y,l) for l in range(min(z,zz),max(z,zz)+1))
    if any(t in terminals and terminals[t]!=i for t in cells):continue
    if y==3 and (z!=zz or d!=1):continue
    extra=sum(hist[t]+pres*occ[t] for t in cells)
    for dd in (d,(d+1)%4,(d+3)%4):
     if y==16 and dd!=1:continue
     dx,dy=DIR[dd];n=(x+dx,y+dy,zz,dd)
     if n!=goal and not(XMIN<=n[0]<=XMAX and YMIN<=n[1]<=YMAX):continue
     cost=g+1+(.13 if dd!=d else 0)+.5*abs(z-zz)+extra
     if cost<dist.get(n,1e100):
      dist[n]=cost;prev[n]=(s,(x,y,z,d,zz,dd));heapq.heappush(q,(cost+h(n),cost,n))
  return None
 def cells(p):
  result=[]
  for step in p:
   x,y,z,d,zz,dd=step[:6]
   if len(step)==7:
    dx,dy=DIR[d];result.extend([(x,y,z),(x+dx*step[6],y+dy*step[6],z)])
   else:result.extend((x,y,l) for l in range(min(z,zz),max(z,zz)+1))
  return result
 best=99999
 for it in range(rounds):
  order=list(range(36));rng.shuffle(order)
  if it:order.sort(key=lambda i:sum(occ[t]>1 for t in cells(paths[i])),reverse=True)
  for i in order:
   if i in paths:occ.subtract(cells(paths[i]))
   p=route(i,1+it*.8)
   if p is None: return None
   paths[i]=p;occ.update(cells(p))
  conflicts={t:n for t,n in occ.items() if n>1}
  score=sum(n-1 for n in conflicts.values())
  if score<best or it%10==0:
   print(seed,it,'overlap',score,'buildings',sum(map(len,paths.values())),flush=True);best=min(best,score)
  if not score:
   out={'nets':nets,'paths':paths,'bounds':[XMIN,XMAX,YMIN,YMAX]};pathlib.Path('routes.json').write_text(json.dumps(out));return out
  for t,n in conflicts.items():hist[t]+=.4*(n-1)
 return None

if __name__=='__main__':
 for seed in range(5):
  if solve(seed):break
