import json, statistics
g=json.load(open("ir_design/geometry.json"))
raw={a:r for a,r in g["outline_polar_front_raw"]}   # 04 output; never overwritten, so re-running 05 is idempotent
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
print("wrote ir_design/geometry.json")
