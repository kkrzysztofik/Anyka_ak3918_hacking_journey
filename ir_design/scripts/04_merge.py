import json, math, statistics
K=190.50   # back_angle = (K - front_angle) mod 360, mirrored
back=json.load(open("/tmp/back_profile.json"))
front=json.load(open("ir_design/geometry.json"))

# outline in FRONT frame, from the back scan (clean silhouette)
out={}
for i in range(3600):
    fa=round(i*0.1,1)
    ba=round(((K-fa)%360)*10)/10
    key=f"{ba:.1f}" if f"{ba:.1f}" in back else None
    if key is None:
        for d in (0.1,-0.1,0.2,-0.2):
            k2=f"{round((ba+d)%360,1):.1f}"
            if k2 in back: key=k2; break
    if key: out[fa]=back[key]
print(f"outline mapped: {len(out)} samples")

# median filter (5 samples = 0.5 deg) to kill single-pixel noise
ang=sorted(out)
sm={}
for idx,a in enumerate(ang):
    win=[out[ang[(idx+d)%len(ang)]] for d in (-2,-1,0,1,2)]
    sm[a]=statistics.median(win)

CONN=(87.5,113.5)   # connector sector in FRONT frame
bad=[a for a in ang if CONN[0]<=a<=CONN[1]]
print(f"connector sector {CONN[0]}-{CONN[1]} deg: {len(bad)} samples flagged")

bore_dia=16.806            # back scan, bare laminate
holes=[(1.82,16.161,60.20),(1.86,16.030,219.90),(1.85,16.121,340.80)]
emit=[(e["angle_deg"],e["r_mm"]) for e in front["emitters"]]

def xy(a,r): 
    t=math.radians(a); return (r*math.cos(t), r*math.sin(t))

geo={
 "frame":"FRONT view (component side). Origin = bore centre. +x right, +y DOWN, matching KiCad.",
 "registration":{"back_to_front":"back_angle = (190.50 - front_angle) mod 360, MIRRORED",
                 "verified":"3/3 mounting holes predicted within 1.81 deg; trimmed mean outline residual 0.268 mm"},
 "bore_dia_mm":bore_dia,
 "outline_polar_front":[[a,round(sm[a],4)] for a in ang],
 "outline_connector_sector_deg":[86.0,114.5],
 "mounting_holes":[{"dia_mm":d,"r_mm":r,"angle_deg":a,"xy_mm":[round(v,4) for v in xy(a,r)]} for d,r,a in holes],
 "emitters":[{"angle_deg":round(a,2),"r_mm":round(r,3),"xy_mm":[round(v,4) for v in xy(a,r)]} for a,r in emit],
}
json.dump(geo, open("ir_design/geometry.json","w"), indent=1)
print("wrote ir_design/geometry.json")
print(f"\nbore {bore_dia:.3f} mm")
print("mounting holes (front frame):")
for h in geo["mounting_holes"]:
    print(f"  dia {h['dia_mm']:.2f}  x {h['xy_mm'][0]:8.3f}  y {h['xy_mm'][1]:8.3f}  (r {h['r_mm']:.3f} @ {h['angle_deg']:.2f})")
print("emitters (front frame):")
for e in geo["emitters"]:
    print(f"  x {e['xy_mm'][0]:8.3f}  y {e['xy_mm'][1]:8.3f}  (r {e['r_mm']:.3f} @ {e['angle_deg']:6.2f})")
rs=[r for a,r in geo["outline_polar_front"]]
print(f"\noutline radius min {min(rs):.3f} max {max(rs):.3f} mm")
