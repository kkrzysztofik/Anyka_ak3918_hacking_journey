"""Build the JLCPCB fab package in ir_design/fab/ from the routed, filled board.

Refuses to run unless DRC with schematic parity is clean. Writes gerbers + drill
(zipped for upload), a JLCPCB BOM and CPL, and checks them against each other and
the board. Parts without an LCSC number are listed for hand assembly instead of
being silently dropped: JLCPCB does not place a part it cannot look up.
Run after 10_fill.py.
"""
import csv, glob, json, os, re, shutil, subprocess, sys, zipfile
HERE = os.path.dirname(os.path.abspath(__file__)); KD = os.path.join(HERE, "..", "kicad")
PCB = os.path.join(KD, "ir-ring.kicad_pcb"); FAB = os.path.join(HERE, "..", "fab")
parts = json.load(open(os.path.join(KD, "parts.json")))
def run(*a): subprocess.run(a, check=True, capture_output=True)

drc = subprocess.run(["kicad-cli", "pcb", "drc", PCB, "--schematic-parity", "--severity-error",
                      "--exit-code-violations", "-o", "/tmp/fab_drc.rpt"], capture_output=True)
if drc.returncode:
    sys.exit(f"DRC/parity not clean (exit {drc.returncode}), see /tmp/fab_drc.rpt - no fab package")

G = os.path.join(FAB, "gerbers"); shutil.rmtree(G, ignore_errors=True); os.makedirs(G)   # fab/ORDER.md is hand-written: keep it
LAYERS = "F.Cu,B.Cu,F.Mask,B.Mask,F.Paste,B.Paste,F.Silkscreen,B.Silkscreen,Edge.Cuts"
run("kicad-cli", "pcb", "export", "gerbers", PCB, "-o", G + "/", "-l", LAYERS, "--subtract-soldermask")
run("kicad-cli", "pcb", "export", "drill", PCB, "-o", G + "/", "--format", "excellon", "-u", "mm")
with zipfile.ZipFile(os.path.join(FAB, "ir-ring-gerbers.zip"), "w", zipfile.ZIP_DEFLATED) as z:
    for f in sorted(os.listdir(G)): z.write(os.path.join(G, f), f)

# BOM: JLCPCB wants one row per distinct part. Test pads have no part at all.
jlc = {r: p for r, p in parts.items() if p["lcsc"]}
hand = sorted(r for r, p in parts.items() if not p["lcsc"] and not p["footprint"].startswith("TestPoint:"))
groups = {}
for r, p in jlc.items(): groups.setdefault((p["value"], p["footprint"].split(":")[1], p["lcsc"]), []).append(r)
nat = lambda r: (re.sub(r"\d+.*", "", r), int(re.search(r"\d+", r).group()), r)
with open(os.path.join(FAB, "bom_jlcpcb.csv"), "w", newline="") as f:
    w = csv.writer(f); w.writerow(["Comment", "Designator", "Footprint", "LCSC Part #"])
    for (val, fp, lcsc), refs in sorted(groups.items(), key=lambda kv: nat(min(kv[1], key=nat))):
        w.writerow([val, ",".join(sorted(refs, key=nat)), fp, lcsc])

# CPL: KiCad's placement file, renamed to JLCPCB's columns, only for the parts JLCPCB places.
# Same absolute origin as the gerbers, so the two line up without any offset.
raw = "/tmp/fab_pos.csv"
run("kicad-cli", "pcb", "export", "pos", PCB, "-o", raw, "--format", "csv", "--units", "mm", "--side", "both")
rows = list(csv.DictReader(open(raw)))
with open(os.path.join(FAB, "cpl_jlcpcb.csv"), "w", newline="") as f:
    w = csv.writer(f); w.writerow(["Designator", "Mid X", "Mid Y", "Layer", "Rotation"])
    for r in sorted(rows, key=lambda r: nat(r["Ref"])):
        if r["Ref"] in jlc:
            w.writerow([r["Ref"], r["PosX"] + "mm", r["PosY"] + "mm", r["Side"].capitalize(), r["Rot"]])

# cross-checks: every placed part has a position, the back really is populated, holes match the board
bad = 0
def check(cond, msg):
    global bad
    print(("  ok   " if cond else "  FAIL ") + msg); bad += (not cond)
pos = {r["Ref"]: r["Side"] for r in rows}
check(set(jlc) <= set(pos), f"every BOM designator has a CPL row: missing {sorted(set(jlc) - set(pos)) or 'none'}")
back = sorted(r for r in jlc if pos.get(r) == "bottom")
check(len(back) >= 10, f"{len(back)} parts placed on the back (the converter lives there): {' '.join(sorted(back, key=nat))}")
brd = open(PCB).read()
n_via = len(re.findall(r"^\s*\(via\b", brd, re.M)); n_th = len(re.findall(r'\(pad "[^"]*" thru_hole', brd))
drl = "".join(open(p).read() for p in glob.glob(os.path.join(G, "*.drl")))
hits = len(re.findall(r"^X-?[\d.]+Y-?[\d.]+\s*$", drl, re.M))
check(hits == n_via + n_th, f"drill hits {hits} = {n_via} vias + {n_th} plated pads (thermal vias included)")
gbr = sorted(os.listdir(G))
check(len([g for g in gbr if g[-4:] not in (".drl", "rjob")]) == len(LAYERS.split(",")), f"one gerber per layer: {len(gbr)} files incl. drill + job")
with open(os.path.join(FAB, "hand_assembly.txt"), "w") as f:
    f.write("Not placed by JLCPCB (no LCSC number) - solder by hand:\n")
    for r in hand: f.write(f"  {r}: {parts[r]['value']}  ({parts[r]['footprint']})\n")
    if not hand: f.write("  none - JLCPCB places every part\n")
print(f"fab package -> {os.path.normpath(FAB)}: {len(groups)} BOM lines, {len(jlc)} placed parts, hand-solder: {', '.join(hand) or 'none'}")
sys.exit(1 if bad else 0)
