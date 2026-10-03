import json
from scan import load, background, ray
px,W,H,MM=load("ir_design/photos/img20260922_10535866.jpg"); XLIM=int(W*0.62)
bx,by=1275.559,1901.376
bg=background(px,W,H,XLIM)
prof={}
for i in range(3600):
    got=ray(bg,W,H,XLIM,(bx,by),i*0.1)
    if got: prof[round(i*0.1,1)]=round(got*MM,4)
json.dump(prof, open("ir_design/back_profile.json","w"))   # read by 04_merge.py
print("back profile samples:",len(prof), "min",min(prof.values()),"max",max(prof.values()))
