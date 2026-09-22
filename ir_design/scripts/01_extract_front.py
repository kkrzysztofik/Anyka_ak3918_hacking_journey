from PIL import Image
from collections import deque
import math, json
Image.MAX_IMAGE_PIXELS=None
P="ir_design/photos/img20260922_10485355.jpg"
im=Image.open(P); im.draft('L',(im.size[0]//4,im.size[1]//4)); im=im.convert('L')
W,H=im.size; px=im.load(); MM=120.3/W; TH=110
XLIM=int(W*0.60)   # keep the ruler out

def solve3(A,b):
    M=[r[:]+[b[i]] for i,r in enumerate(A)]
    for c in range(3):
        p=max(range(c,3),key=lambda r:abs(M[r][c])); M[c],M[p]=M[p],M[c]
        for r in range(3):
            if r!=c and M[c][c]:
                f=M[r][c]/M[c][c]
                for k in range(c,4): M[r][k]-=f*M[c][k]
    return [M[i][3]/M[i][i] for i in range(3)]
def kasa(pts):
    n=len(pts); Sx=Sy=Sxx=Syy=Sxy=Sz=Szx=Szy=0.0
    for x,y in pts:
        z=x*x+y*y; Sx+=x;Sy+=y;Sxx+=x*x;Syy+=y*y;Sxy+=x*y;Sz+=z;Szx+=z*x;Szy+=z*y
    D,E,F=solve3([[Sxx,Sxy,Sx],[Sxy,Syy,Sy],[Sx,Sy,float(n)]],[-Szx,-Szy,-Sz])
    cx,cy=-D/2,-E/2
    return cx,cy,math.sqrt(max(cx*cx+cy*cy-F,0))
def bnd(pts):
    s=set(pts)
    return [(x,y) for x,y in pts if any((x+dx,y+dy) not in s for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)))]

# background flood
sd=None
for y in range(0,H,7):
    for x in range(0,XLIM,7):
        if px[x,y]>=200: sd=(x,y); break
    if sd: break
bg=bytearray(W*H); q=deque([sd]); bg[sd[1]*W+sd[0]]=1
while q:
    x,y=q.popleft()
    for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)):
        a,b=x+dx,y+dy
        if 0<=a<W and 0<=b<H and not bg[b*W+a] and px[a,b]>=TH:
            bg[b*W+a]=1; q.append((a,b))

# enclosed light regions -> bore + holes
seen=bytearray(W*H); regs=[]
for y in range(H):
    for x in range(XLIM):
        i=y*W+x
        if px[x,y]>=TH and not bg[i] and not seen[i]:
            q=deque([(x,y)]); seen[i]=1; pts=[]
            while q:
                a,b=q.popleft(); pts.append((a,b))
                for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)):
                    c,d=a+dx,b+dy; j=d*W+c
                    if 0<=c<XLIM and 0<=d<H and not seen[j] and px[c,d]>=TH and not bg[j]:
                        seen[j]=1; q.append((c,d))
            if len(pts)>150: regs.append(pts)
regs.sort(key=len,reverse=True)
bx,by,brad=kasa(bnd(regs[0]))
print(f"BORE dia {2*brad*MM:.3f} mm at px({bx:.1f},{by:.1f})")

holes=[]
for pts in regs[1:12]:
    cx,cy,r=kasa(bnd(pts))
    d=math.hypot(cx-bx,cy-by)*MM; a=math.degrees(math.atan2(cy-by,cx-bx))%360
    dia=2*r*MM
    if 1.2<dia<2.6 and 14.0<d<18.5: holes.append((dia,d,a,cx,cy))
print(f"\nMOUNTING HOLES on front scan: {len(holes)}")
for dia,d,a,cx,cy in sorted(holes,key=lambda t:t[2]):
    print(f"  dia {dia:.3f} mm  r {d:.3f} mm  angle {a:6.2f} deg")

# emitters (bright)
seen2=bytearray(W*H); em=[]
for y in range(H):
    for x in range(XLIM):
        i=y*W+x
        if px[x,y]>=205 and not seen2[i]:
            q=deque([(x,y)]); seen2[i]=1; pts=[]
            while q:
                a,b=q.popleft(); pts.append((a,b))
                for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)):
                    c,d=a+dx,b+dy; j=d*W+c
                    if 0<=c<XLIM and 0<=d<H and not seen2[j] and px[c,d]>=205:
                        seen2[j]=1; q.append((c,d))
            if 3000<=len(pts)<=40000: em.append(pts)
emit=[]
for pts in em:
    cx=sum(p[0] for p in pts)/len(pts); cy=sum(p[1] for p in pts)/len(pts)
    d=math.hypot(cx-bx,cy-by)*MM; a=math.degrees(math.atan2(cy-by,cx-bx))%360
    emit.append((a,d))
emit.sort()
print(f"\nEMITTERS: {len(emit)}")
for a,d in emit: print(f"  angle {a:6.2f}  radius {d:.3f} mm")

# outline: first true background going outward
out=[]
for adeg in range(3600):
    ang=math.radians(adeg/10.0); got=None
    for i in range(300*4,1100*4):
        r=i/4.0
        x=int(bx+r*math.cos(ang)); y=int(by+r*math.sin(ang))
        if not(0<=x<XLIM and 0<=y<H): break
        if bg[y*W+x]: got=r*MM; break
    if got: out.append((adeg/10.0, round(got,4)))
print(f"\nOUTLINE: {len(out)} samples, min {min(o[1] for o in out):.3f} max {max(o[1] for o in out):.3f} mm")
json.dump({"scale_mm_per_px":MM,"bore_centre_px":[bx,by],"bore_dia_mm":2*brad*MM,
           "holes":[{"dia_mm":h[0],"r_mm":h[1],"angle_deg":h[2]} for h in sorted(holes,key=lambda t:t[2])],
           "emitters":[{"angle_deg":a,"r_mm":d} for a,d in emit],
           "outline_polar":out,
           "frame":"FRONT scan, +x right, +y DOWN in image, origin = bore centre"},
          open("ir_design/geometry.json","w"), indent=1)
print("wrote ir_design/geometry.json")
