"""Fill every copper zone and save. A separate process on purpose: ZONE_FILLER
segfaults on a CreateEmptyBoard() board in KiCad 9.0.8 standalone scripting,
but works after LoadBoard() - the one call 09_place.py cannot use."""
import pcbnew, sys
b=pcbnew.LoadBoard(sys.argv[1])
cu=[z for z in b.Zones() if not z.GetIsRuleArea()]      # keepouts are rule areas: never filled, not an error
print("zones:", len(cu), "copper,", len(list(b.Zones())) - len(cu), "keepout", flush=True)
ok=pcbnew.ZONE_FILLER(b).Fill(b.Zones())
empty=[z.GetZoneName() or z.GetNetname() for z in cu if not z.IsFilled()]
if not ok or empty: sys.exit(f"zone fill failed (Fill returned {ok}); unfilled copper zones: {empty or 'none'} - board NOT saved")
print("filled:", len(cu), "of", len(cu), "copper zones", flush=True)
pcbnew.SaveBoard(sys.argv[2] if len(sys.argv) > 2 else sys.argv[1], b); print("saved", flush=True)
