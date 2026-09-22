import json, math, statistics, os, sys
sys.setrecursionlimit(20000)
g=json.load(open("ir_design/geometry.json"))
raw={a:r for a,r in g["outline_polar_front"]}
angs=sorted(raw)
c0,c1=g["outline_connector_sector_deg"]

# 1. reconstruct the connector sector from clean neighbours
good=[a for a in angs if not (c0<=a<=c1)]
a_lo=max(a for a in good if a<c0); a_hi=min(a for a in good if a>c1)
work=dict(raw)
for a in angs:
    if c0<=a<=c1:
        work[a]=raw[a_lo]+(raw[a_hi]-raw[a_lo])*((a-a_lo)/(a_hi-a_lo))

# 2. smooth: 2.1 deg moving median kills scan jitter, keeps multi-degree features
N=len(angs); win=21
sm={}
for i,a in enumerate(angs):
    sm[a]=statistics.median([work[angs[(i+k)%N]] for k in range(-(win//2),win//2+1)])
jit_before=statistics.mean(abs(work[angs[i]]-work[angs[i-1]]) for i in range(N))
jit_after =statistics.mean(abs(sm[angs[i]]-sm[angs[i-1]]) for i in range(N))
print(f"edge jitter {jit_before*1000:.0f} um -> {jit_after*1000:.0f} um per 0.1 deg step")
dev=max(abs(sm[a]-work[a]) for a in angs if not (c0<=a<=c1))
print(f"max deviation from traced edge (outside connector sector): {dev:.3f} mm")

g["outline_polar_front"]=[[a,round(sm[a],4)] for a in angs]
g["outline_notes"]=("Connector sector reconstructed by interpolation between clean neighbours - "
                    "the scan ray stops at the connector body, not the board edge. "
                    "2.1 deg moving-median smoothing applied. VERIFY ON THE 1:1 PRINT.")
json.dump(g,open("ir_design/geometry.json","w"),indent=1)

# 3. KiCad
pts=[(sm[a]*math.cos(math.radians(a)), sm[a]*math.sin(math.radians(a))) for a in angs]
def dp(P,eps):
    if len(P)<3: return P
    def perp(p,a,b):
        (x,y),(x1,y1),(x2,y2)=p,a,b
        dx,dy=x2-x1,y2-y1; n=math.hypot(dx,dy)
        return abs(dy*x-dx*y+x2*y1-y2*x1)/n if n else math.hypot(x-x1,y-y1)
    dm=0; idx=0
    for i in range(1,len(P)-1):
        d2=perp(P[i],P[0],P[-1])
        if d2>dm: dm=d2; idx=i
    return dp(P[:idx+1],eps)[:-1]+dp(P[idx:],eps) if dm>eps else [P[0],P[-1]]
simp=dp(pts+[pts[0]],0.015)
print(f"outline {len(pts)} -> {len(simp)} vertices")
import pcbnew
b=pcbnew.CreateEmptyBoard()
def mm(v): return pcbnew.FromMM(float(v))
def seg(p,q,l,w=0.1):
    s=pcbnew.PCB_SHAPE(b); s.SetShape(pcbnew.SHAPE_T_SEGMENT)
    s.SetStart(pcbnew.VECTOR2I(mm(p[0]),mm(p[1]))); s.SetEnd(pcbnew.VECTOR2I(mm(q[0]),mm(q[1])))
    s.SetLayer(l); s.SetWidth(mm(w)); b.Add(s)
def circ(cx,cy,dia,l,w=0.1):
    s=pcbnew.PCB_SHAPE(b); s.SetShape(pcbnew.SHAPE_T_CIRCLE)
    s.SetStart(pcbnew.VECTOR2I(mm(cx),mm(cy))); s.SetEnd(pcbnew.VECTOR2I(mm(cx+dia/2.0),mm(cy)))
    s.SetLayer(l); s.SetWidth(mm(w)); b.Add(s)
for i in range(len(simp)-1): seg(simp[i],simp[i+1],pcbnew.Edge_Cuts)
circ(0,0,g["bore_dia_mm"],pcbnew.Edge_Cuts)
for h in g["mounting_holes"]: circ(h["xy_mm"][0],h["xy_mm"][1],h["dia_mm"],pcbnew.Edge_Cuts)
for e in g["emitters"]:
    x,y=e["xy_mm"]
    seg((x-1,y),(x+1,y),pcbnew.Dwgs_User,0.05); seg((x,y-1),(x,y+1),pcbnew.Dwgs_User,0.05)
    circ(x,y,3.5,pcbnew.Dwgs_User,0.05)
os.makedirs("ir_design/kicad",exist_ok=True)
pcbnew.SaveBoard("ir_design/kicad/ir-ring.kicad_pcb",b)
print("wrote ir_design/kicad/ir-ring.kicad_pcb")
