from PIL import Image, ImageDraw
import json, math
g=json.load(open("ir_design/geometry.json"))
S=22  # px per mm
W=H=int(50*S)
im=Image.new("RGB",(W,H),"white"); d=ImageDraw.Draw(im)
def P(x,y): return (W/2+x*S, H/2+y*S)
# outline
pts=[P(r*math.cos(math.radians(a)), r*math.sin(math.radians(a))) for a,r in g["outline_polar_front"]]
d.polygon(pts, outline="black")
d.line(pts+[pts[0]], fill="black", width=3)
# connector sector highlighted
c0,c1=g["outline_connector_sector_deg"]
cs=[P(r*math.cos(math.radians(a)), r*math.sin(math.radians(a))) for a,r in g["outline_polar_front"] if c0<=a<=c1]
if len(cs)>1: d.line(cs, fill="red", width=5)
# bore
br=g["bore_dia_mm"]/2
d.ellipse([P(-br,-br),P(br,br)], outline="blue", width=3)
# holes
for h in g["mounting_holes"]:
    x,y=h["xy_mm"]; r=h["dia_mm"]/2
    d.ellipse([P(x-r,y-r),P(x+r,y+r)], outline="green", width=3)
# emitters 3535 bodies
for e in g["emitters"]:
    x,y=e["xy_mm"]
    d.rectangle([P(x-1.75,y-1.75),P(x+1.75,y+1.75)], outline="magenta", width=2)
d.text((10,10),"black=Edge.Cuts  red=connector sector (reconstructed)  blue=bore  green=holes  magenta=3535 emitters",fill="black")
im.save("/tmp/check.png"); print("ok")
