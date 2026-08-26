#!/usr/bin/env python3
"""Generate the shipped PCM clip set (s16le mono, 8 kHz).

8 kHz is the rate verified on hardware. Amplitude is deliberately low: the
speaker is loud enough to resonate the plastic casing at the vendor demo's
hardcoded max, so loudness is a property of the file as well as the DAC volume.
"""
import math
import struct
import pathlib

RATE = 8000
OUT = pathlib.Path(__file__).parent.parent / "SD_card_contents/anyka_hack/onvif/sounds"


def tone(freqs, amp=0.25):
    """Sequence of (freq, duration) notes with a short fade to avoid DAC clicks."""
    buf = bytearray()
    for freq, dur in freqs:
        n_total = int(RATE * dur)
        fade = min(400, n_total // 4)
        for n in range(n_total):
            env = min(1.0, n / fade, (n_total - n) / fade) if fade else 1.0
            v = int(32767 * amp * env * math.sin(2 * math.pi * freq * n / RATE))
            buf += struct.pack("<h", v)
    return bytes(buf)


CLIPS = {
    "boot.raw":  [(660, 0.12), (880, 0.18)],   # rising: up and running
    "ok.raw":    [(880, 0.10), (1170, 0.14)],  # short confirmation
    "alert.raw": [(520, 0.18), (420, 0.26)],   # falling: something is wrong
}

if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    for name, notes in CLIPS.items():
        data = tone(notes)
        (OUT / name).write_bytes(data)
        print(f"{name}: {len(data)} bytes ({len(data)/2/RATE:.2f}s)")
