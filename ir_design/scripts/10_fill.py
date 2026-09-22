"""Fill every copper zone and save. A separate process on purpose: ZONE_FILLER
segfaults on a CreateEmptyBoard() board in KiCad 9.0.8 standalone scripting,
but works after LoadBoard() - the one call 09_place.py cannot use."""
import pcbnew, sys
b=pcbnew.LoadBoard(sys.argv[1])
zs=list(b.Zones()); print("zones:", len(zs), flush=True)
pcbnew.ZONE_FILLER(b).Fill(b.Zones())
filled=sum(1 for z in b.Zones() if z.IsFilled()); print("filled:", filled, "of", len(zs), flush=True)
pcbnew.SaveBoard(sys.argv[2] if len(sys.argv) > 2 else sys.argv[1], b); print("saved", flush=True)
