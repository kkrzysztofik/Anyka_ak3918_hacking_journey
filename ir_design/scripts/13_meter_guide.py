"""Multimeter verification guide: ir_design/fab/meter_check_guide.html.

Net -> pad lists come from kicad/nets.json (so they cannot drift from the schematic);
the expected values are worked out by hand from the schematic and written below.
Embeds the two board views from the bottom solder guide, so run 12_bottom_guide.py first.
Run: /usr/bin/python3 ir_design/scripts/13_meter_guide.py
"""
import json, pathlib, re

ROOT = pathlib.Path(__file__).resolve().parents[1]
nets = json.loads((ROOT / "kicad/nets.json").read_text())
FRONT = {"U2", "R6", "R7"} | {f"D{i}" for i in range(2, 10)}
side = lambda r: "front" if r in FRONT else ("front (pins)" if r == "J1" else "back")

# anchor = where to park the black probe; rows list every other pad that must beep to it
ANCHOR = {"GND": "TP3", "VOUT": "TP1", "FB": "TP2", "+5V": "J1 pin 5", "SW": "L1 pad 2",
          "IL_EN": "J1 pin 2", "WL_EN": "J1 pin 1", "LDR": "J1 pin 3", "Q1_G": "R4 pad 1",
          "R1B_Q": "R1b pad 2", "U2_C": "R7 pad 2"}
rows = []
for n, a in ANCHOR.items():
    pads = [f"{r}.{p}" + ("" if side(r) == "back" else f" ({side(r)})") for r, p in nets[n]]
    rows.append(f"<tr><td><b>{n}</b></td><td>{a}</td><td>{', '.join(pads)}</td></tr>")
strings = ", ".join(f"STR{i}" for i in range(1, 8))

grid = (ROOT / "fab/bottom_solder_guide.html").read_text()
views = re.findall(r"<svg.*?</svg>", grid, re.S)
# the board views and their styling come from the solder guide, so the two cannot drift
CSS = re.search(r"<style>(.*?)</style>", grid, re.S).group(1) + """
.ok{background:#e6f6e6}.bad{background:#fde8e8}td{vertical-align:top}table{margin:8px 0}"""

H = f"""<!doctype html><meta charset=utf-8><title>IR ring - multimeter check</title><style>{CSS}</style>
<h1>IR ring - multimeter check after soldering</h1>
<p><b>Unpowered, J1 unplugged.</b> Do these in order; stop at the first failure and fix it before going on.
Put the meter on the 200 &Omega; range, touch the probes together and note the lead resistance (or press REL): it is
subtracted from every low reading below. Pad label on the board = pin:net, red outline = pin 1 (views are the back side, mirrored).</p>
<p>Probe points: <b>TP1 = VOUT, TP2 = FB, TP3 = GND</b> (blue pads), and the five J1 pins (solder joints on the front).</p>
{views[0]}{views[1]}

<h2 class=pb>1. Short check (do this first, nothing may beep)</h2>
<p>Resistance mode. A hard short reads under ~10&Omega;, and so does a solder bridge on U1 or L1. Capacitors make readings
<i>climb</i> for a second; wait for them to settle.</p>
<table><tr><th>Red</th><th>Black</th><th>Expect</th><th>If wrong</th></tr>
<tr class=ok><td>J1 pin 5 (+5V)</td><td>J1 pin 4 (GND)</td><td>&ge; ~200 k&Omega; (R7+R6 floor; lower only if a bright light is on U2). Never a few &Omega;.</td><td>Bridge on U1 pins 2/6, C1/C3, L1 pad 1 to GND.</td></tr>
<tr class=ok><td>TP1 (VOUT)</td><td>TP3 (GND)</td><td>climbs to OL (C2 charging)</td><td>Bridge on D1, C2, U1 pin 5, or an emitter pad shorted to the pour.</td></tr>
<tr class=ok><td>TP2 (FB)</td><td>TP1 (VOUT)</td><td>OL (8 emitters in series)</td><td>Bridge on U1 pins 3/5.</td></tr>
<tr class=ok><td>L1 pad 2 (SW)</td><td>TP3 (GND)</td><td>OL / climbing</td><td>Bridge on U1 pins 1/2, D1.</td></tr>
<tr class=ok><td>J1 pin 2 (IL_EN)</td><td>J1 pin 5 (+5V)</td><td>not a few &Omega; (see R2 test, step 3)</td><td>Bridge on U1 pins 4/5/6.</td></tr>
</table>

<h2>2. Continuity: every net really is one net</h2>
<p>Black probe on the anchor, red on each listed pad. All must beep (&lt; ~1&Omega;). This is how a missed or cold joint shows up.</p>
<table><tr><th>Net</th><th>Anchor</th><th>Pads that must beep to it</th></tr>{''.join(rows)}</table>
<p>Emitter nodes {strings} have no test point. They are covered by the diode test in step 4.</p>

<h2 class=pb>3. Resistances (red on first, black on second; subtract lead resistance)</h2>
<table><tr><th>Between</th><th>Expect</th><th>It is</th></tr>
<tr><td>TP2 (FB) &rarr; TP3 (GND)</td><td><b>4.0&ndash;4.2 &Omega;</b></td><td>R1a alone (Q1 is off: R4 holds its gate low). Red on FB, so Q1's body diode is reverse-biased. Reads ~2 &Omega; if R1b is conducting, i.e. Q1 gate high or Q1 shorted.</td></tr>
<tr><td>J1 pin 5 &rarr; L1 pad 2 (SW)</td><td><b>0.4&ndash;0.6 &Omega;</b></td><td>L1 winding, 429 m&Omega; DCR</td></tr>
<tr><td>J1 pin 1 (WL_EN) &rarr; GND</td><td><b>~101 k&Omega;</b></td><td>R3 1k + R4 100k in series; Q1's gate is open at DC</td></tr>
<tr><td>R4 pad 1 (Q1_G) &rarr; GND</td><td><b>~100 k&Omega;</b></td><td>R4</td></tr>
<tr><td>J1 pin 2 (IL_EN) &rarr; GND</td><td><b>~1 M&Omega;</b> (use the 2 M&Omega; range)</td><td>R2. Lower than ~0.9 M&Omega; means U1's EN pin has a leak path or R2 is the wrong value.</td></tr>
<tr><td>J1 pin 3 (LDR) &rarr; GND</td><td><b>~100 k&Omega;</b></td><td>R6 (U2 is reverse-biased in this direction)</td></tr>
<tr><td>J1 pin 5 (+5V) &rarr; R7 pad 2 (U2_C)</td><td><b>~100 k&Omega;</b></td><td>R7</td></tr>
<tr><td>R1b pad 1 (FB) &rarr; R1b pad 2</td><td><b>4.0&ndash;4.2 &Omega;</b></td><td>R1b itself, read on the part; check the Q1 drain end next:</td></tr>
<tr><td>Q1 pad 3 (drain) &rarr; GND (red on GND)</td><td>diode drop, ~0.5&ndash;0.7 V in diode mode; OL the other way</td><td>Q1 body diode, also proves the source pad is on GND</td></tr>
</table>
<p>Optional, capacitance mode, in circuit: J1 pin 5 &harr; GND &asymp; <b>10 &micro;F</b> (C1 + C3); TP1 &harr; GND &asymp; <b>4.7 &micro;F</b> (C2).
Ceramics lose capacitance under bias but at meter voltage ~0&ndash;1 V the reading should be within 20&ndash;30% of nominal.</p>

<h2>4. Diode mode (the polarised parts)</h2>
<table><tr><th>Red</th><th>Black</th><th>Expect</th><th>Meaning</th></tr>
<tr><td>L1 pad 2 (SW) &mdash; D1 anode</td><td>TP1 (VOUT) &mdash; D1 cathode</td><td><b>0.2&ndash;0.4 V</b></td><td>D1 forward. Reversed: OL. <b>OL both ways = open or D1 turned round; 0 both ways = shorted.</b> (Schottky, so a low drop.)</td></tr>
<tr><td>U1 pin 2 (GND)</td><td>U1 pin 1 (SW)</td><td>typically 0.4&ndash;0.7 V, reversed OL</td><td>U1's internal switch body diode. Not guaranteed for this part, but any hard 0 V means pins 1/2 are bridged.</td></tr>
<tr><td colspan=4><b>Each emitter D2&hellip;D9, on the front:</b> red on the <b>large pad (anode, pin 2)</b>, black on the <b>small pad (cathode, pin 1)</b></td></tr>
<tr class=ok><td>anode</td><td>cathode</td><td><b>about 1.0&ndash;1.5 V</b>, and the emitter glows</td><td>The meter's diode current is enough to light an 850 nm LED: look at it with a phone camera (most show IR as faint purple). Check all eight.</td></tr>
<tr class=bad><td>cathode</td><td>anode</td><td>OL</td><td>Same pads swapped must read OL. If forward reads OL and the reverse reads 1&ndash;1.5 V, that emitter is soldered 180&deg; round &mdash; the failure the order notes warn about.</td></tr>
</table>

<h2>5. Gate drive and light sensor (with a 5 V supply, optional)</h2>
<p>Only if everything above passed. Feed 5 V to J1 pin 5 / pin 4 from a bench supply <b>current-limited to 100 mA</b> with IL_EN and WL_EN left open.</p>
<table><tr><th>Measure</th><th>Expect</th></tr>
<tr><td>J1 pin 5 vs pin 4</td><td>5.0 V, and supply current well under the limit (no hot parts, nothing buzzing)</td></tr>
<tr><td>J1 pin 3 (LDR) vs GND</td><td>a voltage that rises with light on U2 (R7 &rarr; U2 &rarr; R6 divider); in the dark close to 0 V</td></tr>
</table>
<p>Stop here: IL_EN high turns the boost converter on, which is outside what a multimeter can judge &mdash; check VOUT and the LED current on the bench with a limited supply.</p>
"""
out = ROOT / "fab/meter_check_guide.html"
out.write_text(H)
print(out)
