import json,pathlib
r=json.loads(pathlib.Path('routes.json').read_text())
html='''<!doctype html><html><head><meta charset="utf-8"><style>
body{font-family:var(--font-sans,Arial);color:var(--foreground,#ddd);margin:0}button,select{font:inherit;padding:6px 10px;background:var(--secondary,#292b31);color:inherit;border:1px solid var(--border,#555);border-radius:6px}p{font-size:13px;color:var(--muted-foreground,#aaa)}svg{width:100%;height:auto;display:block}.level{margin:16px 0 4px;font-weight:600} .route{cursor:pointer} .route:hover{filter:brightness(1.6)}
</style></head><body><div><select id="pick"><option value="all">显示全部 36 路</option><option value="0">只看 R 输入</option><option value="1">只看 G 输入</option><option value="2">只看 B 输入</option></select> <button onclick="document.getElementById('pick').value='all';draw()">复位</button></div><p>北侧进、南侧出。箭头为带方向；虚线为跳带；圆圈标注升降的目标层。点击线路可单独追踪。</p><div id="layers"></div><script>
const data=DATA;const colors=['#fa6977','#54d598','#67a5ff'];let selected=null;
const D=[[1,0],[0,1],[-1,0],[0,-1]];
function draw(){let out='';const chosen=document.getElementById('pick').value;
for(let l=2;l>=0;l--){out+='<div class="level">第 '+(l+1)+' 层</div><svg viewBox="-21 -1 62 22" xmlns="http://www.w3.org/2000/svg">';
out+='<rect x="-17.5" y="2.5" width="54" height="14" fill="var(--card,#202329)" stroke="var(--border,#555)" stroke-width=".08"/>';
for(let x=-17;x<=37;x++)out+=`<path d="M ${x-.5} 2.5 V16.5" stroke="var(--border,#555)" opacity=".3" stroke-width=".025"/>`;
for(let y=3;y<=17;y++)out+=`<path d="M -17.5 ${y-.5} H36.5" stroke="var(--border,#555)" opacity=".3" stroke-width=".025"/>`;
for(let c=0;c<3;c++){out+=`<text x="${-10+20*c}" y="1" font-size="1" text-anchor="middle" fill="${colors[c]}">${'RGB'[c]} 输入</text><text x="${-10+20*c}" y="19" font-size=".9" text-anchor="middle" fill="${colors[2-l]}">出口 ${c+1} · ${'RGB'[2-l]}</text>`;}
for(const [id,steps] of Object.entries(data.paths)){const [c,z,k]=data.nets[id];const opacity=(selected!==null?selected===id:chosen==='all'||Number(chosen)===c)?1:.07;out+=`<g class="route" onclick="selected=selected==='${id}'?null:'${id}';draw()" stroke="${colors[c]}" fill="none" opacity="${opacity}"><title>${'RGB'[c]} 输入第${z+1}层，第${k+1}带</title>`;
for(const s of steps){let [x,y,z,d,zz,dd,jump]=s;let [dx,dy]=D[d];let [ex,ey]=D[dd];
if(jump&&z===l){out+=`<path d="M ${x} ${y} l ${dx*jump} ${dy*jump}" stroke-width=".12" stroke-dasharray=".22 .18"/><rect x="${x-.28}" y="${y-.28}" width=".56" height=".56" stroke-width=".1"/><circle cx="${x+dx*jump}" cy="${y+dy*jump}" r=".25" stroke-width=".1"/>`;continue;}
if(z===zz&&z===l)out+=`<path d="M ${x-dx*.5} ${y-dy*.5} L ${x} ${y} L ${x+ex*.5} ${y+ey*.5}" stroke-width=".16"/><path d="M ${x+ex*.2-ey*.12} ${y+ey*.2+ex*.12} L ${x+ex*.4} ${y+ey*.4} L ${x+ex*.2+ey*.12} ${y+ey*.2-ex*.12}" stroke-width=".08"/>`;
else if(z!==zz&&l>=Math.min(z,zz)&&l<=Math.max(z,zz)){out+=`<circle cx="${x}" cy="${y}" r=".38" stroke-width=".1"/><text x="${x}" y="${y+.14}" text-anchor="middle" font-size=".45" stroke="none" fill="${colors[c]}">${zz+1}</text>`;if(z===l)out+=`<path d="M ${x-dx*.5} ${y-dy*.5} L ${x} ${y}" stroke-width=".12"/>`;if(zz===l)out+=`<path d="M ${x} ${y} L ${x+ex*.5} ${y+ey*.5}" stroke-width=".12"/>`;}
}out+='</g>';}
out+='</svg>';}
document.getElementById('layers').innerHTML=out;}
document.getElementById('pick').onchange=()=>{selected=null;draw()};draw();</script></body></html>'''.replace('DATA',json.dumps(r))
pathlib.Path('layout.html').write_text(html,encoding='utf8')
