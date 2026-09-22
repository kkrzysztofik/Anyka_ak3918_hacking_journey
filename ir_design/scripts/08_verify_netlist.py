"""Export the netlist KiCad actually computed and diff it against NETS.

ERC proves every pin is connected to *something*. This proves every pin is
connected to the *right* thing. Exits 1 on any difference.
"""
import json, os, subprocess, sys
here = os.path.dirname(os.path.abspath(__file__))
kd = os.path.join(here, "..", "kicad")
sys.path.insert(0, here); exec(open(os.path.join(here, "sexp.py")).read())
net_file = "/tmp/ir-ring.net"
subprocess.run(["kicad-cli", "sch", "export", "netlist", "--format", "kicadsexpr",
                "-o", net_file, os.path.join(kd, "ir-ring.kicad_sch")], check=True, capture_output=True)
tree = parse(open(net_file).read())[0]
nets_sec = [e for e in tree if isinstance(e, list) and e[0] == "nets"][0]
got = {}
for n in nets_sec[1:]:
    name = [e for e in n if isinstance(e, list) and e[0] == "name"][0][1].strip('"').lstrip("/")
    nodes = {(([e for e in nd if isinstance(e, list) and e[0]=="ref"][0][1].strip('"')),
              ([e for e in nd if isinstance(e, list) and e[0]=="pin"][0][1].strip('"')))
             for nd in n if isinstance(nd, list) and nd[0] == "node"}
    got[name] = nodes
want = {k: {tuple(v) for v in vs} for k, vs in json.load(open(os.path.join(kd, "nets.json"))).items()}
# footprints actually attached
comps = [e for e in tree if isinstance(e, list) and e[0] == "components"][0]
fps = {[x for x in c if isinstance(x, list) and x[0]=="ref"][0][1].strip('"'):
       [x for x in c if isinstance(x, list) and x[0]=="footprint"][0][1].strip('"')
       for c in comps[1:] if isinstance(c, list)}
bad = 0
for k in sorted(set(want) | set(got)):
    if want.get(k) != got.get(k):
        bad += 1
        print(f"MISMATCH {k}: want {sorted(want.get(k, []))} got {sorted(got.get(k, []))}")
nofp = [r for r, f in fps.items() if not f]
if nofp: bad += 1; print("no footprint:", nofp)
print(f"{len(got)} nets, {sum(len(v) for v in got.values())} connections, {len(fps)} components with footprints"
      + (" - IDENTICAL to NETS" if not bad else f" - {bad} PROBLEMS"))
sys.exit(1 if bad else 0)
