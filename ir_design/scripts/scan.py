"""Scan helpers shared by 01-03: load a flatbed scan at 1/4 scale, flood the
background, find enclosed regions, and trace the board silhouette by ray casting."""
from PIL import Image
from collections import deque
import math
Image.MAX_IMAGE_PIXELS=None
TH=110   # board/background threshold
def load(p):
    """-> (pixel access, W, H, mm per px). The scan bed is 120.3 mm wide."""
    im=Image.open(p); im.draft('L',(im.size[0]//4,im.size[1]//4)); im=im.convert('L')
    W,H=im.size
    return im.load(),W,H,120.3/W
def flood(px,W,H,xmax,seed,th,mark,excl=None,collect=True):
    """4-connected BFS from seed over px>=th, x<xmax, not excl; sets mark. -> the points (empty unless collect)."""
    q=deque([seed]); mark[seed[1]*W+seed[0]]=1; pts=[]
    while q:
        x,y=q.popleft()
        if collect: pts.append((x,y))
        for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)):
            a,b=x+dx,y+dy; j=b*W+a
            if 0<=a<xmax and 0<=b<H and not mark[j] and px[a,b]>=th and not (excl and excl[j]):
                mark[j]=1; q.append((a,b))
    return pts
def background(px,W,H,xlim):
    """Flood from the first bright pixel left of xlim (the ruler sits right of it)."""
    sd=next((x,y) for y in range(0,H,7) for x in range(0,xlim,7) if px[x,y]>=200)
    bg=bytearray(W*H); flood(px,W,H,W,sd,TH,bg,collect=False)
    return bg
def regions(px,W,H,xlim,th,excl=None,box=(0,0,1,1)):
    """Every 4-connected region of px>=th (and not excl) seeded inside box (fractions x0,y0,x1,y1)."""
    x0,y0,x1,y1=box; seen=bytearray(W*H); out=[]
    for y in range(int(H*y0),int(H*y1)):
        for x in range(int(W*x0),min(int(W*x1),xlim)):
            i=y*W+x
            if px[x,y]>=th and not seen[i] and not (excl and excl[i]):
                out.append(flood(px,W,H,xlim,(x,y),th,seen,excl))
    return out
def centroid(pts):
    return sum(p[0] for p in pts)/len(pts), sum(p[1] for p in pts)/len(pts)
def ray(bg,W,H,xlim,c,adeg):
    """Distance in px from c to the first background pixel along adeg, or None."""
    t=math.radians(adeg)
    for i in range(300*4,1100*4):
        r=i/4.0
        x=int(c[0]+r*math.cos(t)); y=int(c[1]+r*math.sin(t))
        if not(0<=x<xlim and 0<=y<H): return None
        if bg[y*W+x]: return r
def profile(path, xlimfrac, bore_seed_box):
    """Silhouette radius (mm) every 0.5 deg around the bore = biggest enclosed light region in the box.
    -> (profile, bore centre px, mm/px, background mask)."""
    px,W,H,MM=load(path); XLIM=int(W*xlimfrac)
    bg=background(px,W,H,XLIM)
    x0,x1,y0,y1=bore_seed_box
    c=centroid(max(regions(px,W,H,XLIM,TH,bg,(x0,y0,x1,y1)),key=len))
    prof={}
    for adeg in range(720):
        got=ray(bg,W,H,XLIM,c,adeg*0.5)
        if got: prof[adeg*0.5]=got*MM
    return prof,c,MM,bg
