from PIL import Image
from collections import deque
import math, json
Image.MAX_IMAGE_PIXELS=None
im=Image.open("ir_design/photos/img20260922_10535866.jpg")
im.draft('L',(im.size[0]//4,im.size[1]//4)); im=im.convert('L')
W,H=im.size; px=im.load(); MM=120.3/W; TH=110; XLIM=int(W*0.62)
bx,by=1275.559,1901.376
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
prof={}
for i in range(3600):
    ang=math.radians(i*0.1); got=None
    for j in range(300*4,1100*4):
        r=j/4.0
        x=int(bx+r*math.cos(ang)); y=int(by+r*math.sin(ang))
        if not(0<=x<XLIM and 0<=y<H): break
        if bg[y*W+x]: got=r*MM; break
    if got: prof[round(i*0.1,1)]=round(got,4)
json.dump(prof, open("/tmp/back_profile.json","w"))
print("back profile samples:",len(prof), "min",min(prof.values()),"max",max(prof.values()))
