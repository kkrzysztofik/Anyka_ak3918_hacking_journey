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
check(not intr, f"front copper clear above the hot loop (F.Cu plane solid): {intr or 'nothing there'}")
pcbnew.SaveBoard(PCB, B)
print(f"placed {len(FP)} footprints -> {os.path.normpath(PCB)}")
sys.exit(1 if bad else 0)
