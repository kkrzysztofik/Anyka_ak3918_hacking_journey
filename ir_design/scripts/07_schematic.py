"""Generate ir-ring.kicad_sch: a wired, readable schematic, from one netlist table.

NETS below is the single source of truth for connectivity. The drawing - symbol
placement, wires, power symbols, labels - is laid out per functional block, but
every wire endpoint is computed from the transformed pin positions, never typed.
Wires are split wherever another connection lands mid-segment, junction dots go
where three or more things meet, and every non-power net carries one label so its
KiCad name (/SW, /FB, ...) matches the PCB. 08_verify_netlist.py then diffs the
netlist KiCad computes from this drawing against NETS.
"""
import uuid, os, sys, json
HERE = os.path.dirname(os.path.abspath(__file__))
exec(open(os.path.join(HERE, "sexp.py")).read())

# Deterministic UUIDs: the PCB links footprints to symbols by UUID.
NS = uuid.UUID("6b1f0c8e-2d4a-4b5e-9c1d-7a3e5f2b8c90")
def U(key): return str(uuid.uuid5(NS, key))
ROOT = U("root")

FPR = "Resistor_SMD:R_0805_2012Metric"          # 0805 throughout: hand-reworkable (R6/R7 await the ADC measurement)
FPT = "TestPoint:TestPoint_Pad_D1.5mm"            # bring-up pads, on the back; no part, not in the BOM
PARTS = {
 "J1":  ("Connector_Generic:Conn_01x05", "PicoBlade/MX1.25 5p R/A THT", "ir-ring:Molex_PicoBlade_53048-0510_1x05_P1.25mm_Horizontal_Ring0.275", "C588276"),   # genuine Molex 530480510
 "U1":  ("ir-ring:SY7200A", "SY7200A", "Package_TO_SOT_SMD:SOT-23-6", "C107309"),
 "L1":  ("Device:L", "33uH SWPA4030S330MT", "ir-ring:L_Sunlord_SWPA4030S", "C83470"),
 "D1":  ("Device:D_Schottky", "B5819W 40V 1A", "Diode_SMD:D_SOD-123", "C8598"),   # JLCPCB Basic; was RB160M-60 C77343 (Extended)
 "C1":  ("Device:C", "10uF 25V", "Capacitor_SMD:C_0805_2012Metric", "C15850"),
 "C3":  ("Device:C", "100nF", "Capacitor_SMD:C_0805_2012Metric", "C49678"),
 "C2":  ("Device:C", "4.7uF 50V X7R", "Capacitor_SMD:C_1206_3216Metric", "C29823"),
 "R1a": ("Device:R", "4.02R 1%", "Resistor_SMD:R_0805_2012Metric", "C367870"),
 "R1b": ("Device:R", "4.02R 1%", "Resistor_SMD:R_0805_2012Metric", "C367870"),
 "Q1":  ("Transistor_FET:Q_NMOS_GSD", "AO3400A", "Package_TO_SOT_SMD:SOT-23", "C20917"),
 "R2":  ("Device:R", "1M", FPR, "C17514"),
 "R3":  ("Device:R", "1k", FPR, "C17513"),
 "R4":  ("Device:R", "100k", FPR, "C149504"),
 "U2":  ("Device:Q_Photo_NPN", "TEMT6200FX01", "ir-ring:TEMT6200_0805", "C143695"),
 "R7":  ("Device:R", "100k", FPR, "C149504"),
 "R6":  ("Device:R", "100k", FPR, "C149504"),
 "TP1": ("Connector:TestPoint", "VOUT", FPT, ""),
 "TP2": ("Connector:TestPoint", "FB", FPT, ""),
 "TP3": ("Connector:TestPoint", "GND", FPT, ""),
}
for i in range(8):
    PARTS[f"D{i+2}"] = ("Device:LED", "IR 850nm 120deg", "ir-ring:LED_3535_JNJ_EW120", "C22447930")

# ---- THE netlist: net -> [(ref, pin)] ----
# J1 pin numbers are REVERSED relative to the stock board's "+ - LDR IR HB"
# silkscreen order. J1 is a PicoBlade-class 1.25 mm right-angle THT part with
# its body on the BACK and its opening facing the board edge. Flipped to the
# back and facing outward, the footprint's pad 5 lands on the stock "+" pad
# (front-frame x = -5.52 mm). The cable wire order along the row is unchanged.
# Measured 2026-09-22; see ir_design/MEASUREMENTS.md "J1".
NETS = {
 "+5V":   [("J1","5"),("U1","6"),("L1","1"),("C1","1"),("C3","1"),("R7","1")],
 "GND":   [("J1","4"),("U1","2"),("C1","2"),("C3","2"),("C2","2"),("R1a","2"),("Q1","2"),("R2","2"),("R4","2"),("R6","2"),("TP3","1")],
 "LDR":   [("J1","3"),("U2","2"),("R6","1")],
 "IL_EN": [("J1","2"),("U1","4"),("R2","1")],
 "WL_EN": [("J1","1"),("R3","1")],
 "SW":    [("U1","1"),("L1","2"),("D1","2")],
 "VOUT":  [("D1","1"),("C2","1"),("U1","5"),("D2","2"),("TP1","1")],
 "FB":    [("D9","1"),("U1","3"),("R1a","1"),("R1b","1"),("TP2","1")],
 "R1B_Q": [("R1b","2"),("Q1","3")],
 "Q1_G":  [("R3","2"),("Q1","1"),("R4","1")],
 "U2_C":  [("R7","2"),("U2","1")],
}
for i in range(7):
    NETS[f"STR{i+1}"] = [(f"D{i+2}","1"),(f"D{i+3}","2")]

POWER = {"+5V", "GND"}                      # drawn with power symbols: global KiCad names, no "/"
KNAME = {n: (n if n in POWER else "/" + n) for n in NETS}

# ---------------------------------------------------------------- symbols
def lib_symbol(lib_id):
    lib, name = lib_id.split(":")
    if lib == "ir-ring":
        s = parse(open(os.path.join(HERE, "..", "kicad", "ir-ring.kicad_sym")).read())[0]
        s = [e for e in s if isinstance(e, list) and e[0] == "symbol" and e[1] == f'"{name}"'][0]
    else:
        s = find_sym(lib, name)
    assert extends(s) is None, f"{lib_id} uses extends; pick the base symbol"
    s = list(s); s[1] = f'"{lib_id}"'
    return s

# ---------------------------------------------------------------- layout (mm, y down, 1.27 grid)
# ref -> (x, y, angle, mirror, ref_xy, value_xy, justify)
def vt(x, y): return ((x + 2.54, y - 1.27), (x + 2.54, y + 1.27), "left")      # vertical 2-pin
def hz(x, y): return ((x, y - 3.81), (x, y + 3.81), "center")                   # horizontal
def vl(x, y): return ((x - 2.54, y - 1.27), (x - 2.54, y + 1.27), "right")     # vertical, text on the left
L = {}
def put(ref, x, y, a=0, mirror=None, text=None):
    L[ref] = (x, y, a, mirror) + (text if text else vt(x, y))
# input connector: mirrored so its pins face the circuit
put("J1", 45.72, 88.9, 0, "y", ((40.64, 80.01), (40.64, 97.79), "center"))
# boost converter
put("U1", 152.4, 88.9, 0, None, ((152.4, 80.01), (152.4, 97.79), "center"))
put("C1", 111.76, 76.2); put("C3", 127.0, 76.2); put("L1", 175.26, 76.2)
put("D1", 185.42, 85.09, 180, None, hz(185.42, 85.09)); put("C2", 199.39, 92.71)
put("R2", 132.08, 96.52)
# current switch: FB sense resistors, Q1, gate network
put("R1a", 170.18, 102.87, 0, None, vl(170.18, 102.87)); put("R1b", 180.34, 102.87)
put("Q1", 177.8, 114.3, 0, None, ((184.15, 113.03), (184.15, 115.57), "left"))
put("R3", 162.56, 114.3, 90, None, ((162.56, 111.76), (162.56, 116.84), "center"))
put("R4", 170.18, 120.65)
# light sensor
put("R7", 292.1, 68.58)
put("U2", 289.56, 81.28, 0, None, ((295.91, 80.01), (295.91, 82.55), "left"))
put("R6", 292.1, 96.52)
# test pads: symbol origin is the pin, the graphic sits above it
for i, r in enumerate(("TP1", "TP2", "TP3")):
    put(r, 231.14 + 12.7 * i, 111.76, 0, None, ((233.68 + 12.7 * i, 106.68), (233.68 + 12.7 * i, 109.22), "left"))
# LED string: one chain, anode left
for n in range(2, 10):
    x = 55.88 + 25.4 * (n - 2)
    put(f"D{n}", x, 160.02, 180, None, ((x, 156.21), (x, 164.47), "center"))

used = sorted({PARTS[r][0] for r in PARTS} | {"power:+5V", "power:GND", "power:PWR_FLAG"})
SYMS = {lid: lib_symbol(lid) for lid in used}
PINS = {lid: pins(SYMS[lid]) for lid in used}

def xform(px, py, a, mirror):
    if mirror == "y": px = -px
    a %= 360
    return {0: (px, py), 90: (-py, px), 180: (-px, -py), 270: (py, -px)}[a]
def P(ref, num):
    x, y, a, m = L[ref][:4]
    px, py = PINS[PARTS[ref][0]][num][:2]
    rx, ry = xform(px, py, a, m)
    return (round(x + rx, 4), round(y - ry, 4))       # symbol Y is up, schematic Y is down

# ---------------------------------------------------------------- drawing
WIRES, LABELS, PWR = [], [], []          # polylines; (name, x, y, angle); (lib, x, y, angle, value)
def W(*pts): WIRES.append([tuple(map(float, p)) for p in pts])
def LB(name, xy, a=0): LABELS.append((name, xy[0], xy[1], a))
def PS(kind, xy, a=0): PWR.append((kind, xy[0], xy[1], a))

# J1: pin order is reversed vs the stock silkscreen, see NETS comment
W(P("J1", "1"), (58.42, P("J1", "1")[1])); LB("WL_EN", (58.42, P("J1", "1")[1]))
W(P("J1", "2"), (58.42, P("J1", "2")[1])); LB("IL_EN", (58.42, P("J1", "2")[1]))
W(P("J1", "3"), (58.42, P("J1", "3")[1])); LB("LDR", (58.42, P("J1", "3")[1]))
W(P("J1", "4"), (63.5, P("J1", "4")[1])); PS("GND", (63.5, P("J1", "4")[1]), 90)
W(P("J1", "5"), (58.42, P("J1", "5")[1])); PS("+5V", (58.42, P("J1", "5")[1]), 270)

# +5V rail, input decoupling, inductor
RY = 68.58
PS("+5V", (99.06, RY)); PS("PWR_FLAG", (104.14, RY), 180)      # flag hangs below the rail
W((99.06, RY), (P("L1", "1")[0], RY), P("L1", "1"))
for c in ("C1", "C3"):
    W((P(c, "1")[0], RY), P(c, "1")); PS("GND", P(c, "2"))
W((137.16, RY), (137.16, P("U1", "6")[1]), P("U1", "6"))
# switch node: L1 - LX - D1 anode
SWN = (P("L1", "2")[0], P("U1", "1")[1])
W(P("L1", "2"), SWN); W(SWN, P("U1", "1")); W(SWN, P("D1", "2")); LB("SW", (167.64, SWN[1]))
# output: D1 cathode - C2 - to the string, and back to OVP
VN = (P("C2", "1")[0], P("D1", "1")[1])
W(P("D1", "1"), VN, P("C2", "1")); PS("GND", P("C2", "2"))
W(P("C2", "2"), (207.01, P("C2", "2")[1])); PS("PWR_FLAG", (207.01, P("C2", "2")[1]), 180)   # GND flag
W(VN, (209.55, VN[1])); LB("VOUT", (209.55, VN[1]))
W(P("U1", "5"), (167.64, P("U1", "5")[1])); LB("VOUT", (167.64, P("U1", "5")[1]))
# enable with its pull-down, IC ground
W(P("U1", "4"), (P("R2", "1")[0], P("U1", "4")[1]), (124.46, P("U1", "4")[1])); LB("IL_EN", (124.46, P("U1", "4")[1]), 180)
W((P("R2", "1")[0], P("U1", "4")[1]), P("R2", "1")); PS("GND", P("R2", "2"))
W(P("U1", "2"), (139.7, P("U1", "2")[1]), (139.7, 97.79)); PS("GND", (139.7, 97.79))
# feedback: sense resistors, R1b switched in by Q1
FB1, FB2 = (P("R1a", "1")[0], P("U1", "3")[1]), (P("R1b", "1")[0], P("U1", "3")[1])
W(P("U1", "3"), FB1, FB2, (190.5, FB2[1])); LB("FB", (190.5, FB2[1]))
W(FB1, P("R1a", "1")); PS("GND", P("R1a", "2"))
W(FB2, P("R1b", "1"))
W(P("R1b", "2"), P("Q1", "3")); LB("R1B_Q", (P("Q1", "3")[0], 107.95))
PS("GND", P("Q1", "2"))
GN = (P("R4", "1")[0], P("Q1", "1")[1])
W(P("R3", "2"), GN, P("Q1", "1")); W(GN, P("R4", "1")); LB("Q1_G", (167.64, GN[1]), 90)
PS("GND", P("R4", "2"))
W(P("R3", "1"), (153.67, P("R3", "1")[1])); LB("WL_EN", (153.67, P("R3", "1")[1]), 180)
# light sensor: R7 limits, U2 sources into R6, node to J1 pin 3
PS("+5V", (P("R7", "1")[0], 60.96)); W((P("R7", "1")[0], 60.96), P("R7", "1"))
W(P("R7", "2"), P("U2", "1")); LB("U2_C", (P("U2", "1")[0], 74.93))
LN = (P("U2", "2")[0], 88.9)
W(P("U2", "2"), LN, P("R6", "1")); PS("GND", P("R6", "2"))
W(LN, (302.26, LN[1])); LB("LDR", (302.26, LN[1]))
# test pads
for r, net in (("TP1", "VOUT"), ("TP2", "FB")):
    W(P(r, "1"), (P(r, "1")[0], 116.84)); LB(net, (P(r, "1")[0], 116.84), 270)
W(P("TP3", "1"), (P("TP3", "1")[0], 114.3)); PS("GND", (P("TP3", "1")[0], 114.3))
# LED string
W((45.72, 160.02), P("D2", "2")); LB("VOUT", (45.72, 160.02), 180)
for n in range(2, 9):
    a, b = P(f"D{n}", "1"), P(f"D{n+1}", "2")
    W(a, b); LB(f"STR{n-1}", ((a[0] + b[0]) / 2, a[1]))
W(P("D9", "1"), (P("D9", "1")[0] + 7.62, P("D9", "1")[1])); LB("FB", (P("D9", "1")[0] + 7.62, P("D9", "1")[1]))

# ---------------------------------------------------------------- split wires, find junctions
pinpts = [P(r, n) for r in PARTS for n in PINS[PARTS[r][0]]] + [(x, y) for _, x, y, _ in PWR]
segs = [(p, q) for w in WIRES for p, q in zip(w, w[1:]) if p != q]
attach = set(pinpts) | {(x, y) for _, x, y, _ in LABELS} | {p for s in segs for p in s}
def inside(pt, s):
    (x1, y1), (x2, y2) = s; x, y = pt
    if x1 == x2 == x: return min(y1, y2) < y < max(y1, y2)
    if y1 == y2 == y: return min(x1, x2) < x < max(x1, x2)
    return False
assert all(p[0] == q[0] or p[1] == q[1] for p, q in segs), "non-orthogonal wire"
split = []
for s in segs:
    cuts = sorted([pt for pt in attach if inside(pt, s)], key=lambda pt: (pt[0] - s[0][0]) ** 2 + (pt[1] - s[0][1]) ** 2)
    chain = [s[0]] + cuts + [s[1]]; split += list(zip(chain, chain[1:]))
deg = {}
for p, q in split:
    for pt in (p, q): deg[pt] = deg.get(pt, 0) + 1
for pt in pinpts: deg[pt] = deg.get(pt, 0) + 1
JUNCTIONS = sorted(pt for pt, d in deg.items() if d >= 3)

# every pin of every part must appear in exactly one net
seen = {}
for net, nodes in NETS.items():
    for nd in nodes:
        assert nd not in seen, f"{nd} in both {seen[nd]} and {net}"; seen[nd] = net
for ref, (lid, *_) in PARTS.items():
    for num in PINS[lid]:
        assert (ref, num) in seen, f"{ref} pin {num} is in no net"
labelled = {n for n, *_ in LABELS}
assert labelled >= (set(NETS) - POWER), f"nets without a naming label: {set(NETS) - POWER - labelled}"

# ---------------------------------------------------------------- emit
F = "(effects (font (size 1.27 1.27)))"
def eff(just="center", hide=False):
    j = "" if just == "center" else f" (justify {just})"
    return f"(effects (font (size 1.27 1.27)){j}{' (hide yes)' if hide else ''})"
out = [f'(kicad_sch (version 20250114) (generator "eeschema") (generator_version "9.0") (uuid "{ROOT}") (paper "A3")',
       '(title_block (title "IR ring replacement") (company "anyka-dev") (comment 1 "Generated by ir_design/scripts/07_schematic.py - edit NETS/layout there, not here"))',
       "(lib_symbols"] + [dump(SYMS[l], 1) for l in used] + [")"]
out.append(f'(text "IR ring replacement - SY7200A boost, 8 x 850 nm in series, 50/100 mA by switched sense resistor.\\n'
           f'Generated from NETS in 07_schematic.py; 08_verify_netlist.py diffs this drawing against it." '
           f'(exclude_from_sim no) (at 25.4 25.4 0) {eff("left")} (uuid "{U("note")}"))')
BLOCKS = [("INPUT - camera cable (PicoBlade 1.25)", 30.48, 72.39),
          ("BOOST CONVERTER - SY7200A", 96.52, 58.42), ("HALF / FULL POWER - HB switches R1b in", 147.32, 132.08),
          ("LIGHT SENSOR - populates the stock LDR position", 276.86, 53.34),
          ("LED STRING - 8 x JNJ 3535 850 nm, in series", 43.18, 147.32),
          ("TEST PADS - bring-up, back side", 226.06, 99.06)]
for i, (t, x, y) in enumerate(BLOCKS):
    out.append(f'(text "{t}" (exclude_from_sim no) (at {x} {y} 0) (effects (font (size 1.524 1.524) (bold yes)) (justify left)) (uuid "{U(f"blk{i}")}"))')

for ref, (lid, val, fp, lcsc) in PARTS.items():
    x, y, a, m, rxy, vxy, just = L[ref]
    mir = f" (mirror {m})" if m else ""
    pl = "".join(f'(pin "{n}" (uuid "{U(f"pin:{ref}:{n}")}"))' for n in PINS[lid])
    out.append(
      f'(symbol (lib_id "{lid}") (at {x} {y} {a}){mir} (unit 1) (exclude_from_sim no) (in_bom {"no" if fp == FPT else "yes"}) (on_board yes) (dnp no) (uuid "{U(f"sym:{ref}")}")'
      f'(property "Reference" "{ref}" (at {rxy[0]} {rxy[1]} 0) {eff(just)})'
      f'(property "Value" "{val}" (at {vxy[0]} {vxy[1]} 0) {eff(just)})'
      f'(property "Footprint" "{fp}" (at {x} {y} 0) {eff(hide=True)})'
      f'(property "Datasheet" "~" (at {x} {y} 0) {eff(hide=True)})'
      f'(property "Description" "" (at {x} {y} 0) {eff(hide=True)})'
      f'(property "LCSC" "{lcsc}" (at {x} {y} 0) {eff(hide=True)})'
      f'{pl}(instances (project "ir-ring" (path "/{ROOT}" (reference "{ref}") (unit 1)))))')

pwr_n = {"power:+5V": 0, "power:GND": 0, "power:PWR_FLAG": 0}
for k, (kind, x, y, a) in enumerate(PWR):
    lid = "power:" + kind; pwr_n[lid] += 1
    ref = ("#FLG" if kind == "PWR_FLAG" else "#PWR") + f"{k+1:02d}"
    val = kind
    vdx, vdy = {0: (0, -3.81), 90: (3.81, 0), 180: (0, 3.81), 270: (3.81, 0)}[a]
    if kind == "GND": vdx, vdy = {0: (0, 3.81), 90: (4.445, 0), 180: (0, -3.81), 270: (-4.445, 0)}[a]
    out.append(
      f'(symbol (lib_id "{lid}") (at {x} {y} {a}) (unit 1) (exclude_from_sim no) (in_bom yes) (on_board yes) (dnp no) (uuid "{U(f"pwr:{k}")}")'
      f'(property "Reference" "{ref}" (at {x} {y} 0) {eff(hide=True)})'
      f'(property "Value" "{val}" (at {x + vdx} {y + vdy} 0) {eff()})'
      f'(property "Footprint" "" (at {x} {y} 0) {eff(hide=True)})'
      f'(property "Datasheet" "" (at {x} {y} 0) {eff(hide=True)})'
      f'(property "Description" "" (at {x} {y} 0) {eff(hide=True)})'
      f'(pin "1" (uuid "{U(f"pwrpin:{k}")}"))(instances (project "ir-ring" (path "/{ROOT}" (reference "{ref}") (unit 1)))))')

for i, (p, q) in enumerate(split):
    out.append(f'(wire (pts (xy {p[0]} {p[1]}) (xy {q[0]} {q[1]})) (stroke (width 0) (type default)) (uuid "{U(f"w:{p}:{q}")}"))')
for pt in JUNCTIONS:
    out.append(f'(junction (at {pt[0]} {pt[1]}) (diameter 0) (color 0 0 0 0) (uuid "{U(f"j:{pt}")}"))')
for i, (name, x, y, a) in enumerate(LABELS):
    just = "left" if a in (0, 90) else "right"
    out.append(f'(label "{name}" (at {x} {y} {a}) (effects (font (size 1.27 1.27)) (justify {just} bottom)) (uuid "{U(f"lbl:{i}:{name}")}"))')

out.append('(sheet_instances (path "/" (page "1")))')
out.append("(embedded_fonts no))")
KD = os.path.join(HERE, "..", "kicad")
open(os.path.join(KD, "ir-ring.kicad_sch"), "w").write("\n".join(out))
json.dump({k: {"lib_id": v[0], "value": v[1], "footprint": v[2], "lcsc": v[3], "uuid": U(f"sym:{k}")} for k, v in PARTS.items()},
          open(os.path.join(KD, "parts.json"), "w"), indent=1)
json.dump(NETS, open(os.path.join(KD, "nets.json"), "w"), indent=1)
json.dump(KNAME, open(os.path.join(KD, "net_names.json"), "w"), indent=1)
print(f"wrote ir-ring.kicad_sch: {len(PARTS)} parts, {len(NETS)} nets, {len(split)} wire segments, "
      f"{len(JUNCTIONS)} junctions, {len(LABELS)} labels, {len(PWR)} power symbols")
