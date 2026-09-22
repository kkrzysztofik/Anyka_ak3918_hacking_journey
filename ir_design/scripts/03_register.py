from PIL import Image
from collections import deque
import math, json
Image.MAX_IMAGE_PIXELS=None
exec(open('/tmp/reg.py').read().split('pf,cf,mmf=')[0])
pf,cf,mmf=profile("ir_design/photos/img20260922_10485355.jpg",0.60,(0.25,0.45,0.45,0.65))
pb,cb,mmb=profile("ir_design/photos/img20260922_10535866.jpg",0.62,(0.25,0.45,0.45,0.65))

def resid(offset):
    out=[]
    for a,r in pf.items():
        b=round(((offset-a)%360)*2)/2
        if b in pb: out.append((a,abs(r-pb[b])))
    return out
# refine: trimmed mean, reject worst 25% (connector sectors)
best=None
for k10 in range(7200):
    k=k10*0.05
    rs=sorted(x[1] for x in resid(k))
    if len(rs)<600: continue
    trimmed=rs[:int(len(rs)*0.75)]
    c=sum(trimmed)/len(trimmed)
    if best is None or c<best[0]: best=(c,k)
c,k=best
print(f"refined offset {k:.2f} deg (mirror), trimmed mean |dr| {c:.3f} mm")
rs=resid(k)
bad=sorted(rs,key=lambda t:-t[1])[:40]
segs=sorted(a for a,_ in bad)
print(f"worst-residual front angles (connector/wires): {segs[0]:.0f}-{segs[-1]:.0f} deg")

# predict front positions of the back-scan mounting holes
back_holes=[(1.818,16.161,130.3),(1.863,16.030,330.6),(1.855,16.121,209.7)]
print("\nPredicted mounting-hole positions in the FRONT frame:")
pred=[]
for dia,r,ba in back_holes:
    fa=(k-ba)%360
    pred.append((dia,r,fa))
    print(f"  dia {dia:.2f} mm  r {r:.3f} mm  front angle {fa:6.2f} deg")

# now look for them on the front scan
im=load("ir_design/photos/img20260922_10485355.jpg"); W,H=im.size; px=im.load(); MM=120.3/W; TH=110; XLIM=int(W*0.60)
bx,by=cf
sd=None
for y in range(0,H,7):
    for x in range(0,XLIM,7):
        if px[x,y]>=200: sd=(x,y);break
    if sd:break
bg=bytearray(W*H); q=deque([sd]); bg[sd[1]*W+sd[0]]=1
while q:
    x,y=q.popleft()
    for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)):
        a,b=x+dx,y+dy
        if 0<=a<W and 0<=b<H and not bg[b*W+a] and px[a,b]>=TH:
            bg[b*W+a]=1; q.append((a,b))
seen=bytearray(W*H); found=[]
for y in range(H):
    for x in range(XLIM):
        i=y*W+x
        if px[x,y]>=TH and not bg[i] and not seen[i]:
            q=deque([(x,y)]); seen[i]=1; pts=[]
            while q:
                a,b=q.popleft(); pts.append((a,b))
                for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)):
                    c2,d=a+dx,b+dy; j=d*W+c2
                    if 0<=c2<XLIM and 0<=d<H and not seen[j] and px[c2,d]>=TH and not bg[j]:
                        seen[j]=1; q.append((c2,d))
            if len(pts)>150:
                cx=sum(p[0] for p in pts)/len(pts); cy=sum(p[1] for p in pts)/len(pts)
                dd=math.hypot(cx-bx,cy-by)*MM; aa=math.degrees(math.atan2(cy-by,cx-bx))%360
                ad=2*math.sqrt(len(pts)/math.pi)*MM
                found.append((ad,dd,aa))
print("\nNearest front-scan feature to each prediction:")
ok=True
for dia,r,fa in pred:
    cands=[f for f in found if abs(f[1]-r)<2.0]
    if not cands: print(f"  {fa:6.2f} deg -> NOTHING within 2 mm of r={r:.1f}"); ok=False; continue
    bestf=min(cands,key=lambda f: min(abs(f[2]-fa),360-abs(f[2]-fa)))
    da=min(abs(bestf[2]-fa),360-abs(bestf[2]-fa))
    flag="OK" if da<3 else "MISMATCH"
    if da>=3: ok=False
    print(f"  {fa:6.2f} deg -> found dia {bestf[0]:.2f} r {bestf[1]:.2f} ang {bestf[2]:6.2f}  (off {da:.2f} deg) {flag}")
print("\nREGISTRATION", "VERIFIED" if ok else "NOT VERIFIED")
