"""Hand-soldering guide for the BOTTOM side: ir_design/fab/bottom_solder_guide.html.

Reads ir-ring.kicad_pcb (read-only LoadBoard is fine) and draws the board as seen
looking at the back: x is mirrored, text is not. Pads carry their net names,
pin 1 is outlined, tracks/vias are drawn so you can see where heat will go.
Run: /usr/bin/python3 ir_design/scripts/12_bottom_guide.py
"""
import math, pathlib, pcbnew

ROOT = pathlib.Path(__file__).resolve().parents[1]
mm = pcbnew.ToMM
b = pcbnew.LoadBoard(str(ROOT / "kicad/ir-ring.kicad_pcb"))
X = lambda x: -x  # look at the back: flip about the vertical axis

def corners(p):
    t = math.radians(p.GetOrientationDegrees()); c, s = math.cos(t), math.sin(t)
    w, h = mm(p.GetSize().x), mm(p.GetSize().y)
    cx, cy = mm(p.GetPosition().x), mm(p.GetPosition().y)
    return [(X(cx + dx*c + dy*s), cy - dx*s + dy*c) for dx, dy in ((-w/2, -h/2), (w/2, -h/2), (w/2, h/2), (-w/2, h/2))]

svg = []
# outline
for d in b.GetDrawings():
    if d.GetLayer() == pcbnew.Edge_Cuts:
        a, e = d.GetStart(), d.GetEnd()
        svg.append(f'<line x1="{X(mm(a.x)):.3f}" y1="{mm(a.y):.3f}" x2="{X(mm(e.x)):.3f}" y2="{mm(e.y):.3f}" class="edge"/>')
# back copper
for t in b.GetTracks():
    if t.GetLayerName() == "B.Cu" and t.GetClass() == "PCB_TRACK":
        a, e = t.GetStart(), t.GetEnd()
        svg.append(f'<line x1="{X(mm(a.x)):.3f}" y1="{mm(a.y):.3f}" x2="{X(mm(e.x)):.3f}" y2="{mm(e.y):.3f}" class="trk" stroke-width="{mm(t.GetWidth()):.3f}"/>')
    elif t.GetClass() == "PCB_VIA":
        p = t.GetPosition()
        svg.append(f'<circle cx="{X(mm(p.x)):.3f}" cy="{mm(p.y):.3f}" r="{mm(t.GetWidth(pcbnew.B_Cu))/2:.3f}" class="via"/>')
# back pads + labels
refs = []
for f in b.GetFootprints():
    if not f.IsFlipped():
        continue
    ref = f.GetReference()
    pts = []
    for p in f.Pads():
        cs = corners(p)
        pts += cs
        thd = p.GetAttribute() != pcbnew.PAD_ATTRIB_SMD
        cls = "pad thd" if thd else ("pad tp" if ref.startswith("TP") else "pad")
        pl = " ".join(f"{x:.3f},{y:.3f}" for x, y in cs)
        svg.append(f'<polygon points="{pl}" class="{cls}"/>')
        cx, cy = X(mm(p.GetPosition().x)), mm(p.GetPosition().y)
        if p.GetNumber() == "1" and not ref.startswith("TP"):
            svg.append(f'<polygon points="{pl}" class="pin1"/>')
        if thd:
            svg.append(f'<circle cx="{cx:.3f}" cy="{cy:.3f}" r="{mm(p.GetDrillSize().x)/2:.3f}" class="hole"/>')
        net = p.GetNetname().lstrip("/")
        svg.append(f'<text x="{cx:.3f}" y="{cy+0.12:.3f}" class="net">{p.GetNumber()}:{net}</text>')
    refs.append((ref, f.GetValue(), sum(x for x, _ in pts)/len(pts), sum(y for _, y in pts)/len(pts), min(y for _, y in pts)))
for ref, val, cx, cy, ymin in refs:
    if ref.startswith("TP"):
        svg.append(f'<text x="{cx:.3f}" y="{cy+0.15:.3f}" class="tp">{ref}</text>')
    else:
        svg.append(f'<text x="{cx:.3f}" y="{ymin-0.5:.3f}" class="ref">{ref} <tspan class="val">{val}</tspan></text>')
body = "\n".join(svg)

CSS = """
body{font:14px system-ui;margin:20px;max-width:1100px}
svg{background:#fff;border:1px solid #888;width:100%}
.edge{stroke:#000;stroke-width:.15}.trk{stroke:#c9a14a;stroke-linecap:round;opacity:.55}
.via{fill:none;stroke:#888;stroke-width:.1}.pad{fill:#d4a017;stroke:#7a5c00;stroke-width:.05}
.pad.thd{fill:#c0c0c0}.pad.tp{fill:#9ec9e6}.hole{fill:#fff;stroke:#333;stroke-width:.04}
.pin1{fill:none;stroke:#d00;stroke-width:.14}
.net{font:.34px monospace;text-anchor:middle;fill:#000}.ref{font:bold .6px system-ui;text-anchor:middle;fill:#06c}
.val{font:.4px system-ui;fill:#555;font-weight:normal}.tp{font:bold .5px system-ui;text-anchor:middle}
table{border-collapse:collapse}td,th{border:1px solid #aaa;padding:3px 8px;text-align:left}
h2{margin-top:28px}.pb{page-break-before:always}
"""
views = [
    ("1. Whole board, seen from the back (mirrored left-right, as if you turned it over like a page)", "-170 82 43 46"),
    ("2. Converter cluster (U1, L1, D1, C1-C3, R1a/b, R2) - the dense part", "-162 83 22 15"),
    ("3. Q1 / R3 / R4 gate-drive corner", "-140 96 12 15"),
    ("4. J1 connector (pins come through to the FRONT)", "-155 114 14 10"),
]
html = [f"<!doctype html><meta charset=utf-8><title>IR ring - bottom solder guide</title><style>{CSS}</style>",
        "<h1>IR ring - bottom side hand-solder guide</h1>",
        "<p>Generated from <code>kicad/ir-ring.kicad_pcb</code>. Pad label = <b>pin:net</b>. "
        "<span style='color:#d00'>Red outline = pin 1.</span> Tan lines are B.Cu tracks (a large pour also covers the back; it is not drawn). "
        "Blue pads TP1-3 are bare test points: nothing to solder.</p>"]
for i, (t, vb) in enumerate(views):
    html.append(f"<h2{' class=pb' if i else ''}>{t}</h2><svg viewBox='{vb}'>{body}</svg>")
html.append("""<h2 class=pb>Order and notes</h2>
<table><tr><th>#</th><th>Part</th><th>Package</th><th>Orientation</th></tr>
<tr><td>1</td><td>U1 SY7200A</td><td>SOT-23-6</td><td>Pin 1 = SW (the pad red-outlined), pins 1-3 on one long side, 4-6 on the other. Tack one corner pad, align, then the rest. Most fiddly part - do it first while the board is bare.</td></tr>
<tr><td>2</td><td>Q1 AO3400A</td><td>SOT-23</td><td>1=gate (R3/R4 side), 2=source/GND, 3=drain (R1b). Single pad is the drain; two pads are the gate and source.</td></tr>
<tr><td>3</td><td>D1 B5819W</td><td>SOD-123</td><td><b>Polarised.</b> Pin 1 = cathode (VOUT), pin 2 = anode (SW). Cathode bar on the VOUT side.</td></tr>
<tr><td>4</td><td>R1a, R1b (4.02 R), R2 (1M), R3 (1k), R4 (100k)</td><td>0805</td><td>Not polarised. Check the value before placing: R1a/R1b are 4.02 R, R2 is 1M, R3 is 1k, R4 is 100k.</td></tr>
<tr><td>5</td><td>C3 (100n), C1 (10u 25V)</td><td>0805</td><td>Ceramic, not polarised.</td></tr>
<tr><td>6</td><td>C2 4.7u 50V</td><td>1206</td><td>Not polarised.</td></tr>
<tr><td>7</td><td>L1 33uH</td><td>4x4 mm</td><td>Not polarised. Hot, large pads - do last of the SMD parts, with more heat.</td></tr>
<tr><td>8</td><td>J1 PicoBlade 5-pin</td><td>THT R/A</td><td>Body on the back; pins exit on the <b>front</b>, so solder the five pins from the front side. Opening faces the board edge.</td></tr>
</table>
<p>R6, R7 and U2 are on the <b>front</b>, not here. The front-side emitters D2-D9 must be soldered separately.</p>""")
out = ROOT / "fab/bottom_solder_guide.html"
out.write_text("\n".join(html))
print(out)
