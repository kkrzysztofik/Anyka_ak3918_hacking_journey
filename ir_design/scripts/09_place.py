"""Place every footprint on ir-ring.kicad_pcb from parts.json / nets.json.

Coordinates are the verified front frame from geometry.json: origin at the
bore centre, +x right, +y down (KiCad's own convention). Parts are oriented by
a pad-to-pad direction in board coordinates, so flipping to the back needs no
special-casing. Run after 05 (outline) and 07 (schematic). Asserts its own
geometry and exits non-zero if a check fails.
"""
import json, math, os, sys
import pcbnew
HERE = os.path.dirname(os.path.abspath(__file__)); KD = os.path.join(HERE, "..", "kicad")
PCB = os.path.join(KD, "ir-ring.kicad_pcb")
geo = json.load(open(os.path.join(HERE, "..", "geometry.json")))
parts = json.load(open(os.path.join(KD, "parts.json")))
nets = json.load(open(os.path.join(KD, "nets.json")))
mm = pcbnew.FromMM; tomm = pcbnew.ToMM

# Build from scratch. pcbnew.LoadBoard() corrupts the SWIG type table in the
# KiCad 9.0.8 Python build (later FootprintLoad/GetDesignSettings return bare
# SwigPyObjects and the process segfaults); CreateEmptyBoard() does not.
B = pcbnew.CreateEmptyBoard()

def seg(p, q, layer, w=0.1):
    s = pcbnew.PCB_SHAPE(B); s.SetShape(pcbnew.SHAPE_T_SEGMENT)
    s.SetStart(pcbnew.VECTOR2I(mm(p[0]), mm(p[1]))); s.SetEnd(pcbnew.VECTOR2I(mm(q[0]), mm(q[1])))
    s.SetLayer(layer); s.SetWidth(mm(w)); B.Add(s)
def circ(c, dia, layer, w=0.1):
    s = pcbnew.PCB_SHAPE(B); s.SetShape(pcbnew.SHAPE_T_CIRCLE)
    s.SetStart(pcbnew.VECTOR2I(mm(c[0]), mm(c[1]))); s.SetEnd(pcbnew.VECTOR2I(mm(c[0] + dia / 2), mm(c[1])))
    s.SetLayer(layer); s.SetWidth(mm(w)); B.Add(s)
def dp(P, eps):
    if len(P) < 3: return P
    (x1, y1), (x2, y2) = P[0], P[-1]; n = math.hypot(x2 - x1, y2 - y1)
    dist = [abs((y2 - y1) * x - (x2 - x1) * y + x2 * y1 - y2 * x1) / n if n else math.hypot(x - x1, y - y1) for x, y in P]
    k = max(range(1, len(P) - 1), key=lambda i: dist[i])
    return dp(P[:k + 1], eps)[:-1] + dp(P[k:], eps) if dist[k] > eps else [P[0], P[-1]]
sys.setrecursionlimit(20000)
ring = [(r * math.cos(math.radians(a)), r * math.sin(math.radians(a))) for a, r in geo["outline_polar_front"]]
ring = dp(ring + [ring[0]], 0.015)      # outline is already smoothed + connector-corrected by 05
for i in range(len(ring) - 1): seg(ring[i], ring[i + 1], pcbnew.Edge_Cuts)
circ((0, 0), geo["bore_dia_mm"], pcbnew.Edge_Cuts)
for h in geo["mounting_holes"]: circ(tuple(h["xy_mm"]), h["dia_mm"], pcbnew.Edge_Cuts)

ds = B.GetDesignSettings()
ds.SetBoardThickness(mm(1.0))
for attr, val in [("m_TrackMinWidth", 0.15), ("m_MinClearance", 0.15), ("m_ViasMinSize", 0.6),
                  ("m_MinThroughDrill", 0.3), ("m_CopperEdgeClearance", 0.3), ("m_HoleToHoleMin", 0.25)]:
    assert hasattr(ds, attr), attr
    setattr(ds, attr, mm(val))

# netclasses: widths per current. DSN export carries these to Freerouting.
ns = ds.m_NetSettings
def netclass(name, track, clear):
    nc = pcbnew.NETCLASS(name); nc.SetTrackWidth(mm(track)); nc.SetClearance(mm(clear))
    nc.SetViaDiameter(mm(0.6)); nc.SetViaDrill(mm(0.3)); return nc
dflt = ns.GetDefaultNetclass(); dflt.SetTrackWidth(mm(0.2)); dflt.SetClearance(mm(0.15))
dflt.SetViaDiameter(mm(0.6)); dflt.SetViaDrill(mm(0.3))
ns.SetNetclass("Power", netclass("Power", 0.5, 0.15))      # width for ~0.5 A peak; 0.15 mm clearance is
                                                           # ample at <=16 V running / 33 V clamp (IPC-2221 ~0.1 mm to 50 V)
ns.SetNetclass("LED", netclass("LED", 0.3, 0.15))          # 100 mA string current
for pat in ["/+5V", "/GND", "/SW", "/VOUT"]: ns.SetNetclassPatternAssignment(pat, "Power")
for pat in ["/STR*", "/FB", "/R1B_Q"]: ns.SetNetclassPatternAssignment(pat, "LED")

NET = {}
for name in nets:
    n = pcbnew.NETINFO_ITEM(B, "/" + name); B.Add(n); NET[name] = n
pin_net = {(r, p): n for n, nodes in nets.items() for r, p in nodes}

LIBS = {"ir-ring": os.path.join(KD, "ir-ring.pretty")}
def libpath(lib): return LIBS.get(lib, f"/usr/share/kicad/footprints/{lib}.pretty")

def kang(dx, dy):                       # KiCad angle of a board vector (y down, CCW positive)
    return math.degrees(math.atan2(-dy, dx))
def pad(fp, num):
    return [p for p in fp.Pads() if p.GetNumber() == num][0]
def ppos(fp, num):
    q = pad(fp, num).GetPosition(); return (tomm(q.x), tomm(q.y))

FP = {}
def make(ref):
    lib, name = parts[ref]["footprint"].split(":")
    fp = pcbnew.FootprintLoad(libpath(lib), name)
    assert fp is not None, f"{ref}: footprint {lib}:{name} not found"
    fp.SetFPID(pcbnew.LIB_ID(lib, name))      # FootprintLoad(path) leaves the lib nickname empty
    fp.SetReference(ref); fp.SetValue(parts[ref]["value"])
    if parts[ref]["lcsc"]:
        fp.SetField("LCSC", parts[ref]["lcsc"])
    fp.SetPath(pcbnew.KIID_PATH("/" + parts[ref]["uuid"]))
    for p in fp.Pads():
        if p.GetNumber() and (ref, p.GetNumber()) in pin_net:
            p.SetNet(NET[pin_net[(ref, p.GetNumber())]])
    B.Add(fp); FP[ref] = fp
    return fp

def place(ref, at, a_pad, b_pad, direction_deg, back=False, anchor=None):
    """Put `ref` so the vector a_pad->b_pad points along board angle direction_deg
    (y-down screen degrees, 0 = +x, 90 = +y). `at` is where the anchor pad (or the
    footprint origin, if anchor is None) lands."""
    fp = FP.get(ref) or make(ref)
    fp.SetPosition(pcbnew.VECTOR2I(0, 0)); fp.SetOrientationDegrees(0)
    if back and not fp.IsFlipped():
        fp.Flip(pcbnew.VECTOR2I(0, 0), pcbnew.FLIP_DIRECTION_LEFT_RIGHT)
    fp.SetOrientationDegrees(0)
    (ax, ay), (bx, by) = ppos(fp, a_pad), ppos(fp, b_pad)
    want = kang(math.cos(math.radians(direction_deg)), math.sin(math.radians(direction_deg)))
    fp.SetOrientationDegrees((want - kang(bx - ax, by - ay)) % 360)
    if anchor:
        px, py = ppos(fp, anchor); fp.Move(pcbnew.VECTOR2I(mm(at[0] - px), mm(at[1] - py)))
    else:
        fp.SetPosition(pcbnew.VECTOR2I(mm(at[0]), mm(at[1])))
    return fp

def polar(r, th): return (r * math.cos(math.radians(th)), r * math.sin(math.radians(th)))
def tangent(th): return (th + 90) % 360          # direction of increasing angle, y-down degrees

# ---- FRONT: the eight emitters, string order around the ring from the gap ----
em = sorted(geo["emitters"], key=lambda e: (e["angle_deg"] - 300) % 360)   # starts just past the gap
for i, e in enumerate(em):
    ref = f"D{i+2}"
    # pad 2 (anode) -> pad 1 (cathode) points along the string: cathode faces the next emitter
    place(ref, tuple(e["xy_mm"]), "2", "1", tangent(e["angle_deg"]))
# ---- FRONT: light sensor on the stock LDR position (r 15.0 @ 280.2 deg, under the 9th dome) ----
LDR_XY, LDR_TH = (2.67, -14.78), 280.2
place("U2", LDR_XY, "2", "1", tangent(LDR_TH))
place("R7", polar(15.2, 290.7), "2", "1", tangent(LDR_TH))    # beside U2 (courtyards need >= 9.9 deg at r 15.2)
place("R6", polar(15.2, 269.7), "2", "1", tangent(LDR_TH))

# ---- BACK: converter in the top gap. Local frame: tv tangential (+ = increasing angle), tu radial ----
PHI, R0 = 280.0, 14.0
def L(tv, tu):
    ox, oy = polar(R0, PHI); t = math.radians(PHI)
    return (ox + tv * -math.sin(t) + tu * math.cos(t), oy + tv * math.cos(t) + tu * math.sin(t))
V = tangent(PHI); U_ = PHI                          # board directions of +tv and +tu
Vm, Um = (V + 180) % 360, (U_ + 180) % 360
# U1 pin1->pin3 toward -tv puts the LX/GND/FB column on the INNER side (checked in the
# render). Everything is built around that: L1 beside LX, D1 under it, C2 on D1's cathode
# with its ground next to U1 GND, so the LX -> D1 -> C2 -> GND hot loop stays tiny. The
# feedback group sits left, toward D9 (the FB end of the string); J1's nets arrive from below.
# Local extents from the real courtyards (SOT-23-6 standard pads, not hand-solder).
# U1 pin1->pin3 = -tv puts LX/GND/FB on the INNER column, LX at +tv (right), FB left.
place("U1", L(0.0, -0.6), "1", "3", Vm, back=True)        # box tv[-1.73,1.73] tu[-2.68,1.48]
place("L1", L(4.25, -0.3), "1", "2", Vm, back=True)       # pad2 = SW on the left, 1.8 mm from LX
place("D1", L(0.6, -3.96), "2", "1", Vm, back=True)       # under U1: A(SW) right, K(VOUT) left
place("C2", L(-3.61, -3.96), "1", "2", Vm, back=True)     # butts D1's cathode; GND via to plane
place("R1a", L(-3.54, -1.74), "1", "2", Vm, back=True)    # FB right -> GND left
place("R1b", L(-3.54, 0.35), "1", "2", Vm, back=True)     # FB right -> R1B_Q left
place("Q1", L(-7.4, 1.8), "1", "2", U_, back=True)        # above the D9 thermal island
place("R4", L(-8.3, 4.3), "1", "2", Vm, back=True)        # Q1_G -> GND
place("R3", L(-6.1, 4.3), "2", "1", Vm, back=True)        # beside R4: at tu 5.4 it sat 0.12 mm from the edge
place("C3", L(0.95, 2.52), "1", "2", U_, back=True)       # right over IN
place("R2", L(-0.95, 2.52), "1", "2", U_, back=True)      # right over EN
place("C1", L(7.8, 1.0), "1", "2", U_, back=True)         # past L1's +5V end, where +5V arrives

# ---- BACK: J1 on the measured contacts. Stock "+" pad is at the left end = footprint pad 5 ----
j = json.load(open(os.path.join(HERE, "..", "j1_contacts.json")))
place("J1", tuple(j["centre"]), "5", "1", j["row_angle_deg"], back=True, anchor="3")
# J1's GND pin is the whole board's return, and the four signal tracks leaving beside it leave
# room for only one relief spoke: connect it solid (one PicoBlade pin is still easy to solder)
pad(FP["J1"], "4").SetLocalZoneConnection(pcbnew.ZONE_CONNECTION_FULL)

# ---- zones: keepouts, per-emitter thermal islands, GND pours. Built BEFORE the DSN so
# Freerouting sees GND as a plane (vias to it, no GND tracks) and routes around keepouts.
def zone(points, layers, net=None, priority=0, keepout=False, name=""):
    z = pcbnew.ZONE(B)
    ls = pcbnew.LSET()
    for l in layers: ls.AddLayer(l)
    z.SetLayerSet(ls)
    o = z.Outline(); o.NewOutline()
    for x, y in points: o.Append(mm(x), mm(y))
    if keepout:
        z.SetIsRuleArea(True); z.SetDoNotAllowTracks(True); z.SetDoNotAllowVias(True)
        z.SetDoNotAllowZoneFills(True) if hasattr(z, "SetDoNotAllowZoneFills") else z.SetDoNotAllowCopperPour(True)
        z.SetDoNotAllowPads(False); z.SetDoNotAllowFootprints(False)
    else:
        z.SetNetCode(net.GetNetCode()); z.SetAssignedPriority(priority)
        z.SetLocalClearance(mm(0.2)); z.SetMinThickness(mm(0.2))
        z.SetThermalReliefGap(mm(0.25)); z.SetThermalReliefSpokeWidth(mm(0.3))
        z.SetPadConnection(pcbnew.ZONE_CONNECTION_THT_THERMAL)   # solid for SMD (hot-loop return), relief for THT
        z.SetIslandRemovalMode(pcbnew.ISLAND_REMOVAL_MODE_ALWAYS)  # drop fragments nothing connects to
    if name: z.SetZoneName(name)
    B.Add(z); return z
def circle_pts(c, r, n=48):
    return [(c[0] + r * math.cos(2 * math.pi * k / n), c[1] + r * math.sin(2 * math.pi * k / n)) for k in range(n)]
CU = [pcbnew.F_Cu, pcbnew.B_Cu]
# keepouts: bore band, and screw-head clearance round each mounting hole
zone(circle_pts((0, 0), geo["bore_dia_mm"] / 2 + 0.4), CU, keepout=True, name="keepout_bore")
for k, h in enumerate(geo["mounting_holes"]):
    zone(circle_pts(tuple(h["xy_mm"]), 1.8), CU, keepout=True, name=f"keepout_hole{k+1}")
# per-emitter thermal islands on B.Cu, each on its own anode net, above the GND pour
for i, e in enumerate(em):
    ref = f"D{i+2}"; an = pin_net[(ref, "2")]
    th = math.radians(e["angle_deg"]); cx, cy = e["xy_mm"]
    rad = (math.cos(th), math.sin(th)); tan = (-math.sin(th), math.cos(th))
    hw_t, hw_r = 2.4, 2.4
    corners = [(cx + a * tan[0] * hw_t + b * rad[0] * hw_r, cy + a * tan[1] * hw_t + b * rad[1] * hw_r)
               for a, b in [(-1, -1), (1, -1), (1, 1), (-1, 1)]]
    zi = zone(corners, [pcbnew.B_Cu], net=NET[an], priority=2, name=f"thermal_{ref}")
    zi.SetPadConnection(pcbnew.ZONE_CONNECTION_FULL)   # thermal vias exist to conduct heat: no relief spokes
# GND pours, whole board, both layers (lowest priority): F.Cu is the hot-loop return plane
outline_pts = ring[:-1]
zone(outline_pts, [pcbnew.B_Cu], net=NET["GND"], priority=1, name="gnd_back")
zone(outline_pts, [pcbnew.F_Cu], net=NET["GND"], priority=0, name="gnd_front")

# GND stitching vias round the outer band: without them the front and back GND pours
# are not joined anywhere. Clear of J1 (~92-109 deg), the mounting holes and the converter.
STITCH = [5, 30, 80, 125, 150, 175, 195, 235, 325, 350]
outline_r = {round(a): r for a, r in geo["outline_polar_front"]}
for a in STITCH:
    r_st = min(18.2, outline_r[round(a)] - 1.2)            # the outline pulls in to ~18.7 mm near 30/150 deg
    for h in geo["mounting_holes"]:
        assert abs((a - h["angle_deg"] + 180) % 360 - 180) > 8, f"stitch {a} too close to a hole"
    v = pcbnew.PCB_VIA(B); v.SetViaType(pcbnew.VIATYPE_THROUGH)
    v.SetPosition(pcbnew.VECTOR2I(*[mm(c) for c in polar(r_st, a)]))
    v.SetWidth(mm(0.6)); v.SetDrill(mm(0.3)); v.SetNet(NET["GND"]); B.Add(v)

# converter GND pads -> F.Cu plane directly above: one via beside each, the hot-loop return path
isl = [z for z in B.Zones() if z.GetZoneName().startswith("thermal_")]
placed_vias = []
def via_free(x, y, rv=0.3, clr=0.2):
    if math.hypot(x, y) < geo["bore_dia_mm"] / 2 + 0.4 + rv + 0.1: return False
    for h in geo["mounting_holes"]:
        if math.dist((x, y), h["xy_mm"]) < 1.8 + rv + 0.1: return False
    pt = pcbnew.VECTOR2I(mm(x), mm(y))
    if any(z.Outline().Contains(pt) for z in isl): return False
    if any(math.dist((x, y), v) < 2 * rv + clr for v in placed_vias): return False
    for f in FP.values():
        for q in f.Pads():
            if q.GetNetname() == "/GND": continue
            bb = q.GetBoundingBox()
            x0, y0 = tomm(bb.GetLeft()) - rv - clr, tomm(bb.GetTop()) - rv - clr
            x1, y1 = tomm(bb.GetRight()) + rv + clr, tomm(bb.GetBottom()) + rv + clr
            if x0 < x < x1 and y0 < y < y1: return False
    return True
hot = []
for ref, num in [("C2", "2"), ("U1", "2"), ("C1", "2"), ("C3", "2"), ("R1a", "2"), ("Q1", "2"), ("R2", "2"), ("R4", "2")]:
    x, y = ppos(FP[ref], num); q = pad(FP[ref], num)
    base = max(tomm(q.GetBoundingBox().GetWidth()), tomm(q.GetBoundingBox().GetHeight())) / 2
    spot = next(((x + d * math.cos(math.radians(a)), y + d * math.sin(math.radians(a)))
                 for d in (base + 0.35, base + 0.6, base + 0.9) for a in range(0, 360, 20)
                 if via_free(x + d * math.cos(math.radians(a)), y + d * math.sin(math.radians(a)))), None)
    if spot is None: hot.append(f"{ref}.{num}: no room"); continue
    v = pcbnew.PCB_VIA(B); v.SetViaType(pcbnew.VIATYPE_THROUGH)
    v.SetPosition(pcbnew.VECTOR2I(mm(spot[0]), mm(spot[1]))); v.SetWidth(mm(0.6)); v.SetDrill(mm(0.3))
    v.SetNet(NET["GND"]); B.Add(v); placed_vias.append(spot); hot.append(f"{ref}.{num}")

# Two connections Freerouting cannot finish, pre-routed so it treats them as fixed:
#  STR7 D8.K -> D9.A: the J1 signals cut across the D8-D9 gap on F.Cu and the hole keepout
#  closes the outside, so route the link first, along the ring, and let the signals go round it.
def nearest_pad(ref, num, target):
    return min((q for q in FP[ref].Pads() if q.GetNumber() == num and q.GetDrillSize().x == 0),
               key=lambda q: math.dist((tomm(q.GetPosition().x), tomm(q.GetPosition().y)), target))
def qpos(q): return (tomm(q.GetPosition().x), tomm(q.GetPosition().y))
def track(pts, layer, net, w):
    for a, b in zip(pts, pts[1:]):
        t = pcbnew.PCB_TRACK(B); t.SetStart(pcbnew.VECTOR2I(mm(a[0]), mm(a[1]))); t.SetEnd(pcbnew.VECTOR2I(mm(b[0]), mm(b[1])))
        t.SetLayer(layer); t.SetWidth(mm(w)); t.SetNet(net); B.Add(t)
d9c = tuple(FP["D9"].GetPosition()); d9c = (tomm(d9c[0]), tomm(d9c[1]))
d8c = (tomm(FP["D8"].GetPosition().x), tomm(FP["D8"].GetPosition().y))
k8 = qpos(nearest_pad("D8", "1", d9c)); a9 = qpos(nearest_pad("D9", "2", d8c))
r_link = (math.hypot(*k8) + math.hypot(*a9)) / 2
th0, th1 = math.degrees(math.atan2(k8[1], k8[0])), math.degrees(math.atan2(a9[1], a9[0]))
if th1 < th0: th1 += 360
arc = [k8] + [polar(r_link, th0 + (th1 - th0) * f) for f in (0.25, 0.5, 0.75)] + [a9]
track(arc, pcbnew.F_Cu, NET["STR7"], 0.3)
hole = min(geo["mounting_holes"], key=lambda h: math.dist(h["xy_mm"], polar(r_link, (th0 + th1) / 2)))
gap_hole = min(math.dist(pt, hole["xy_mm"]) for pt in arc) - 1.8 - 0.15
#  VOUT -> D2: Freerouting's track stopped short of D2's thermal island. Give it an explicit
#  VOUT via inside the island, on D2's anode side (the side facing the gap and the converter).
e2 = em[0]; th2 = math.radians(e2["angle_deg"]); c2 = e2["xy_mm"]
tan2 = (-math.sin(th2), math.cos(th2)); rad2 = (math.cos(th2), math.sin(th2))
vv = (c2[0] - 2.05 * tan2[0] - 1.2 * rad2[0], c2[1] - 2.05 * tan2[1] - 1.2 * rad2[1])
v = pcbnew.PCB_VIA(B); v.SetViaType(pcbnew.VIATYPE_THROUGH); v.SetPosition(pcbnew.VECTOR2I(mm(vv[0]), mm(vv[1])))
v.SetWidth(mm(0.6)); v.SetDrill(mm(0.3)); v.SetNet(NET["VOUT"]); B.Add(v)
track([vv, qpos(nearest_pad("D2", "2", vv))], pcbnew.F_Cu, NET["VOUT"], 0.5)   # via -> D2 anode on F.Cu
in_island = [z for z in B.Zones() if z.GetZoneName() == "thermal_D2"][0].Outline().Contains(pcbnew.VECTOR2I(mm(vv[0]), mm(vv[1])))

# Each emitter's anode is two copper pads (centre/thermal + outer) joined only INSIDE the
# package (pins 2+3). KiCad 9.0.8 has no "duplicate pads are jumpers" footprint flag, so join
# them with a short F.Cu track across the 0.5 mm gap: same net, and the bare board then has
# real continuity from the string link to the thermal pad.
for i in range(8):
    ref = f"D{i+2}"
    anodes = [q for q in FP[ref].Pads() if q.GetNumber() == "2" and q.GetDrillSize().x == 0]
    centre = max(anodes, key=lambda q: q.GetSize().x)                    # the 1.0 mm-wide pad
    outer = min((q for q in anodes if q is not centre), key=lambda q: -q.GetSize().y)   # 0.6 x 3.3, not the tab
    track([qpos(outer), qpos(centre)], pcbnew.F_Cu, NET[pin_net[(ref, "2")]], 0.3)


# ---- checks ----
bad = 0
def check(cond, msg):
    global bad
    print(("  ok   " if cond else "  FAIL ") + msg); bad += (not cond)
cx = [c for c in j["contacts_front"]]
for k, num in enumerate(["5", "4", "3", "2", "1"]):
    x, y = ppos(FP["J1"], num); dx = math.hypot(x - cx[k][0], y - cy[k][1]) if False else math.hypot(x - cx[k][0], y - cx[k][1])
    check(dx < 0.25, f"J1 pad {num} ({pin_net[('J1',num)]}) on stock contact {k+1}: {dx:.3f} mm off")
bb = FP["J1"].GetBoundingBox(False)
bcx, bcy = tomm(bb.GetCenter().x), tomm(bb.GetCenter().y)
check(math.hypot(bcx, bcy) > math.hypot(*j["centre"]), f"J1 body faces outward (body centre r {math.hypot(bcx,bcy):.2f} > pin row r {math.hypot(*j['centre']):.2f})")
def d(a, b):
    (r1, p1), (r2, p2) = a, b; (x1, y1), (x2, y2) = ppos(FP[r1], p1), ppos(FP[r2], p2)
    return math.hypot(x1 - x2, y1 - y2)
for a, b, lim, what in [(("U1","1"),("D1","2"),3.5,"SW: U1 LX - D1 anode"), (("U1","1"),("L1","2"),3.5,"SW: U1 LX - L1"),
                        (("D1","1"),("C2","1"),2.5,"VOUT: D1 - C2"), (("C2","2"),("U1","2"),5.5,"hot-loop return: C2 GND - U1 GND (closes through the F.Cu plane above)"),
                        (("U1","6"),("C3","1"),2.5,"IN decoupling: U1 IN - C3"), (("U1","3"),("R1a","1"),2.0,"FB: U1 - R1a"), (("Q1","3"),("R1b","2"),3.0,"R1B_Q: Q1 drain - R1b")]:
    v = d(a, b); check(v < lim, f"{what}: {v:.2f} mm (< {lim})")
for e_ref in [f"D{i+2}" for i in range(7)]:
    nxt = f"D{int(e_ref[1:])+1}"
    v = d((e_ref, "1"), (nxt, "2")); check(v < 12.0, f"string {e_ref}.K -> {nxt}.A: {v:.2f} mm")
# ---- render both sides for eyeballing (/tmp/place_front.png, /tmp/place_back.png) ----
def render(side, path, S=26):
    from PIL import Image, ImageDraw
    W = H = int(46 * S); im = Image.new("RGB", (W, H), "white"); dr = ImageDraw.Draw(im)
    P = lambda x, y: (W / 2 + x * S, H / 2 + y * S)
    dr.line([P(*q) for q in ring], fill="black", width=2)
    br = geo["bore_dia_mm"] / 2; dr.ellipse([P(-br, -br), P(br, br)], outline="black", width=2)
    for h in geo["mounting_holes"]:
        x, y = h["xy_mm"]; r = h["dia_mm"] / 2; dr.ellipse([P(x - r, y - r), P(x + r, y + r)], outline="black", width=2)
    col = {}
    pal = ["#e6194b","#3cb44b","#4363d8","#f58231","#911eb4","#42d4f4","#f032e6","#9a6324","#800000","#469990","#000075","#808000","#ffe119","#aaffc3","#fabed4","#dcbeff","#a9a9a9","#000000"]
    by_net = {}
    for ref, fp in FP.items():
        on = fp.IsFlipped() == (side == "back")
        for p in fp.Pads():
            n = p.GetNetname().lstrip("/")
            if not n: continue
            x, y = tomm(p.GetPosition().x), tomm(p.GetPosition().y)
            th = p.GetDrillSize().x > 0
            if on or th: by_net.setdefault(n, []).append((x, y))
            if not (on or th): continue
            c = col.setdefault(n, pal[len(col) % len(pal)])
            w, h = tomm(p.GetBoundingBox().GetWidth()) / 2, tomm(p.GetBoundingBox().GetHeight()) / 2
            dr.rectangle([P(x - w, y - h), P(x + w, y + h)], fill=c)
        if on:
            bb = fp.GetBoundingBox(False); x0, y0 = tomm(bb.GetX()), tomm(bb.GetY())
            dr.rectangle([P(x0, y0), P(x0 + tomm(bb.GetWidth()), y0 + tomm(bb.GetHeight()))], outline="#999999")
            cx, cy = tomm(fp.GetPosition().x), tomm(fp.GetPosition().y); dr.text(P(cx + 0.4, cy - 0.9), ref, fill="black")
    for n, pts in by_net.items():          # nearest-neighbour ratsnest per net
        left = pts[1:]; tree = [pts[0]]
        while left:
            a, b = min(((a, b) for a in tree for b in left), key=lambda ab: math.dist(*ab))
            dr.line([P(*a), P(*b)], fill=col.get(n, "gray"), width=1); tree.append(b); left.remove(b)
    y = 6
    for n, c in col.items(): dr.rectangle([6, y, 20, y + 12], fill=c); dr.text((24, y), n, fill="black"); y += 16
    dr.text((W - 260, 8), f"{side.upper()} side, viewed from the FRONT", fill="black")
    im.save(path)
render("front", "/tmp/place_front.png"); render("back", "/tmp/place_back.png")
# reference designators to the fab layer: the silkscreen is too dense for them, JLCPCB
# does not need them, and they stay in the design. Polarity marks stay on silk.
for ref, f in FP.items():
    f.Reference().SetLayer(pcbnew.B_Fab if f.IsFlipped() else pcbnew.F_Fab)
# J1 net names on the back silkscreen, one per pin, rotated to run in line with the pin
jr = math.radians(j["row_angle_deg"]); inward = (math.sin(jr), -math.cos(jr))     # perpendicular, toward the bore
if math.hypot(*j["centre"]) < math.hypot(j["centre"][0] + inward[0], j["centre"][1] + inward[1]):
    inward = (-inward[0], -inward[1])
for num, label in [("5", "5V"), ("4", "GND"), ("3", "LDR"), ("2", "IL"), ("1", "WL")]:
    x, y = ppos(FP["J1"], num)
    t = pcbnew.PCB_TEXT(B); t.SetText(label); t.SetLayer(pcbnew.B_SilkS)
    t.SetTextSize(pcbnew.VECTOR2I(mm(0.8), mm(0.8))); t.SetTextThickness(mm(0.12))
    t.SetPosition(pcbnew.VECTOR2I(mm(x + inward[0] * 1.9), mm(y + inward[1] * 1.9)))
    t.SetTextAngleDegrees((kang(*inward) ) % 360); t.SetMirrored(True); B.Add(t)

# the hot loop returns through the F.Cu GND plane directly above it: nothing on the front may sit there
def local(xy):
    ox, oy = polar(R0, PHI); t = math.radians(PHI); dx, dy = xy[0] - ox, xy[1] - oy
    return (dx * -math.sin(t) + dy * math.cos(t), dx * math.cos(t) + dy * math.sin(t))
intr = [f"{r}.{p.GetNumber()}" for r, f in FP.items() if not f.IsFlipped() for p in f.Pads()
        if p.GetDrillSize().x == 0 and -5.5 < local(ppos(f, p.GetNumber()))[1] < -1.0 and -5.5 < local(ppos(f, p.GetNumber()))[0] < 3.5]
check(gap_hole > 0.1, f"pre-routed STR7 D8->D9 arc at r {r_link:.2f} clears the hole keepout by {gap_hole:.2f} mm")
check(in_island, "VOUT target via sits inside D2's thermal island")
check(all("no room" not in h for h in hot), f"GND stitching via beside converter GND pads: {hot}")
check(not intr, f"front copper clear above the hot loop (F.Cu plane solid): {intr or 'nothing there'}")
# Built around the bore centre (0,0); shift onto the middle of an A4 sheet so the GUI shows it
# on the page. Done before DSN export / SES import so the routed geometry stays consistent.
SHEET = (148.5, 105.0)
off = pcbnew.VECTOR2I(mm(SHEET[0]), mm(SHEET[1]))
for item in list(B.GetFootprints()) + list(B.GetDrawings()) + list(B.GetTracks()) + list(B.Zones()):
    item.Move(off)
ds.SetAuxOrigin(off); ds.SetGridOrigin(off)        # drill/placement files keep the bore centre as origin

SES = sys.argv[sys.argv.index("--ses") + 1] if "--ses" in sys.argv else None
if SES:
    ok = pcbnew.ImportSpecctraSES(B, SES)
    check(ok, f"imported routes from {SES}: {len(B.GetTracks())} track/via items")
    thin = [t for t in B.GetTracks() if t.GetClass() == "PCB_TRACK" and t.GetWidth() < mm(0.15)]
    for t in thin: t.SetWidth(mm(0.15))
    check(True, f"widened {len(thin)} autorouter neck-downs below the 0.15 mm board minimum")
pass  # zones filled by 10_fill.py (ZONE_FILLER segfaults on a CreateEmptyBoard board)
pcbnew.SaveBoard(PCB, B)
DSN = os.path.join(KD, "ir-ring.dsn")
if not SES:
    check(pcbnew.ExportSpecctraDSN(B, DSN), f"exported {os.path.normpath(DSN)} for Freerouting")
    # Freerouting undershoots clearance by ~1 um (0.1488 vs 0.15 in DRC); give it 10 um of margin
    import re
    txt = open(DSN).read()
    txt = re.sub(r"\(clearance (\d+(?:\.\d+)?)", lambda m: f"(clearance {float(m.group(1)) + 10:g}", txt)
    # everything routed before Freerouting (STR7 arc, VOUT via + stub, stitching vias) is
    # intentional: mark it protected, or Freerouting rips it up and reroutes it
    txt = txt.replace("(type route)", "(type protect)")
    open(DSN, "w").write(txt)
print(f"placed {len(FP)} footprints -> {os.path.normpath(PCB)}")
sys.exit(1 if bad else 0)
