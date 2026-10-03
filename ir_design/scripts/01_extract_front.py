import math, json
from scan import load, background, regions, centroid, TH
px,W,H,MM=load("ir_design/photos/img20260922_10485355.jpg")
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

bg=background(px,W,H,XLIM)

# enclosed light regions -> bore + holes
regs=[r for r in regions(px,W,H,XLIM,TH,bg) if len(r)>150]
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
em=[r for r in regions(px,W,H,XLIM,205) if 3000<=len(r)<=40000]
emit=[]
for pts in em:
    cx,cy=centroid(pts)
    d=math.hypot(cx-bx,cy-by)*MM; a=math.degrees(math.atan2(cy-by,cx-bx))%360
    emit.append((a,d))
emit.sort()
print(f"\nEMITTERS: {len(emit)}")
for a,d in emit: print(f"  angle {a:6.2f}  radius {d:.3f} mm")

# no outline here: 04 takes it from the back scan (02), which has a clean silhouette
json.dump({"scale_mm_per_px":MM,"bore_centre_px":[bx,by],"bore_dia_mm":2*brad*MM,
           "holes":[{"dia_mm":h[0],"r_mm":h[1],"angle_deg":h[2]} for h in sorted(holes,key=lambda t:t[2])],
           "emitters":[{"angle_deg":a,"r_mm":d} for a,d in emit],
           "frame":"FRONT scan, +x right, +y DOWN in image, origin = bore centre"},
          open("ir_design/geometry.json","w"), indent=1)
print("wrote ir_design/geometry.json")
