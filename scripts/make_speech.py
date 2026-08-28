#!/usr/bin/env python3
"""Generate Polish TTS PCM clips (s16le mono, 16 kHz) for event audio.

Host-side only. Synthesis happens here and the resulting .raw files are
committed, so the camera never needs network access and a normal build never
needs an API key -- only regeneration does.

Requires ffmpeg on the build host and an ElevenLabs API key:

    export ELEVENLABS_API_KEY=...
    python3 scripts/make_speech.py

    python3 scripts/make_speech.py --list-voices   # discover voice IDs
    python3 scripts/make_speech.py --voice <id>    # pick a specific voice

Why 16 kHz: the clips are speech, and Polish sibilants (s, sz, cz, s') carry
most of their energy above 4 kHz. At the previous 8 kHz the anti-alias filter
discarded exactly the band that makes them distinguishable. The DA supports
8/16/32/48 kHz; SOUND_SAMPLE_RATE in platform/anyka/sound.rs must match.

Why loudnorm: the previous espeak-based version peaked at 0.96-1.00 full scale
while sitting at only 0.105 RMS -- simultaneously clipping and quiet. EBU R128
normalisation with a true-peak ceiling fixes both, and ffmpeg does it properly
rather than us hand-rolling a limiter.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from pathlib import Path

RATE = 16000
OUT = Path(__file__).parent.parent / "SD_card_contents/anyka_hack/onvif/sounds"

API_ROOT = "https://api.elevenlabs.io/v1"

# eleven_multilingual_v2 handles Polish on any voice; quality still varies by
# voice, so run --list-voices and pick one that suits before settling.
DEFAULT_VOICE = "21m00Tcm4TlvDq8ikWAM"
DEFAULT_MODEL = "eleven_multilingual_v2"

# Integrated loudness target in LUFS, and true-peak ceiling in dBFS. -14 is a
# common speech target; the camera speaker is small and often in a noisy room,
# so we sit louder than broadcast but leave 2 dB of true-peak headroom.
DEFAULT_LUFS = -14.0
DEFAULT_PEAK_DB = -2.0

# Gentle compression BEFORE loudnorm, because on speech the true-peak ceiling
# binds long before the loudness target does: measured on the espeak clips,
# loudnorm alone could only reach 0.083 RMS on the peakiest clip no matter what
# LUFS target it was given, since raising it further would breach TP. Reducing
# the crest factor first lets the same peak ceiling carry ~3.5 dB more RMS
# (0.083 -> 0.126). 4:1 rather than 6:1: the extra ratio buys only 0.5 dB and
# starts to sound squashed.
DEFAULT_COMPRESS = "acompressor=threshold=-18dB:ratio=4:attack=5:release=50"

# Short phrases for a small speaker; filenames match [sound.events] in config.toml.
CLIPS: dict[str, str] = {
    "boot.raw": "Kamera gotowa.",
    "alert.raw": "Utracono połączenie z siecią.",
    "ok.raw": "Połączenie sieciowe przywrócone.",
    "upgrade.raw": "Aktualizacja zakończona pomyślnie.",
}


def require_tool(name: str) -> str:
    path = shutil.which(name)
    if path is None:
        sys.exit(f"{name} not found. Install it, e.g.\n  sudo apt-get install -y {name}")
    return path


def require_key() -> str:
    key = os.environ.get("ELEVENLABS_API_KEY")
    if not key:
        sys.exit(
            "ELEVENLABS_API_KEY is not set.\n"
            "Get a key at https://elevenlabs.io/ then:\n"
            "  export ELEVENLABS_API_KEY=...\n"
            "The committed .raw clips are only regenerated when this script runs,\n"
            "so a normal build does not need a key."
        )
    return key


def api_request(path: str, key: str, payload: dict | None = None) -> bytes:
    """GET when payload is None, else POST. Returns the raw response body."""
    headers = {"xi-api-key": key, "Accept": "*/*"}
    data = None
    if payload is not None:
        data = json.dumps(payload).encode()
        headers["Content-Type"] = "application/json"

    req = urllib.request.Request(f"{API_ROOT}{path}", data=data, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            return resp.read()
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", "replace")[:400]
        hint = ""
        if "library voices" in body:
            hint = ("\n  Free accounts cannot use Voice Library voices via the API."
                    "\n  Run --list-voices and pick one marked 'ok' (category=premade).")
        elif e.code == 401:
            hint = "  (bad or expired ELEVENLABS_API_KEY)"
        elif e.code == 422:
            hint = "  (voice id or model not accepted -- try --list-voices)"
        sys.exit(f"ElevenLabs HTTP {e.code} on {path}{hint}\n{body}")
    except urllib.error.URLError as e:
        sys.exit(f"Cannot reach ElevenLabs: {e.reason}")


def list_voices(key: str) -> None:
    """List voices, flagging which ones a free account may actually use.

    Free accounts can synthesise with `premade` voices but not with `library`
    ones added from the Voice Library -- those return "Free users cannot use
    library voices via the API". The category is the only way to tell them
    apart before spending a request, so show it and sort usable ones first.
    """
    voices = json.loads(api_request("/voices", key))["voices"]
    usable = {"premade", "cloned", "generated", "professional"}

    print(f"{'':3} {'voice_id':24} {'name':22} {'category':13} labels")
    for v in sorted(voices, key=lambda x: x.get("category") != "premade"):
        cat = v.get("category", "?")
        mark = "ok " if cat in usable else "PAID"
        labels = ", ".join(f"{k}={x}" for k, x in (v.get("labels") or {}).items())
        print(f"{mark:3} {v['voice_id']:24} {v.get('name', '?'):22} {cat:13} {labels}")

    print("\n'ok' = usable on a free account. 'PAID' = library voice, needs a "
          "paid plan.\neleven_multilingual_v2 speaks Polish on any voice; "
          "prosody varies, so try a few.")


def synthesize_mp3(text: str, *, key: str, voice: str, model: str) -> bytes:
    """Request mp3 rather than a pcm_* output_format.

    mp3_44100_128 is available on every account tier, whereas the pcm_* formats
    are gated on paid plans. ffmpeg has to run anyway for resampling and
    loudness, so decoding mp3 costs us nothing and avoids a tier dependency.
    """
    return api_request(
        f"/text-to-speech/{voice}?output_format=mp3_44100_128",
        key,
        {"text": text, "model_id": model},
    )


def to_pcm(
    mp3: bytes, *, ffmpeg: str, lufs: float, peak_db: float, compress: str
) -> bytes:
    """Decode, compress, normalise loudness, resample to 16 kHz mono s16le."""
    chain = f"{compress},loudnorm=I={lufs}:TP={peak_db}:LRA=11" if compress \
        else f"loudnorm=I={lufs}:TP={peak_db}:LRA=11"

    with tempfile.TemporaryDirectory(prefix="anyka-speech-") as tmp:
        src = Path(tmp) / "clip.mp3"
        raw = Path(tmp) / "clip.raw"
        src.write_bytes(mp3)

        subprocess.run(
            [
                ffmpeg, "-y", "-hide_banner", "-loglevel", "error",
                "-i", str(src),
                # Compress then loudnorm, both before the rate change: they
                # work on the decoded stream and ffmpeg resamples internally
                # for the loudness measurement.
                "-af", chain,
                "-ar", str(RATE),
                "-ac", "1",
                "-f", "s16le",
                str(raw),
            ],
            check=True,
        )
        return raw.read_bytes()


def apply_fade(pcm: bytes, rate: int = RATE, fade_ms: int = 15) -> bytes:
    """Short linear fade in/out so the DAC does not click at clip boundaries."""
    samples = len(pcm) // 2
    if samples == 0:
        return pcm
    fade = min(int(rate * fade_ms / 1000), samples // 4)
    if fade == 0:
        return pcm

    out = bytearray(pcm)
    for i in range(fade):
        for idx, env in ((i, i / fade), (samples - 1 - i, (fade - i) / fade)):
            offset = idx * 2
            sample = struct.unpack_from("<h", out, offset)[0]
            struct.pack_into("<h", out, offset, int(sample * env))
    return bytes(out)


def measure(pcm: bytes) -> tuple[float, float]:
    """Peak and RMS as a fraction of full scale, for the summary line."""
    n = len(pcm) // 2
    if n == 0:
        return 0.0, 0.0
    s = struct.unpack(f"<{n}h", pcm[: n * 2])
    peak = max(abs(x) for x in s) / 32768
    rms = (sum(x * x for x in s) / n) ** 0.5 / 32768
    return peak, rms


def main() -> int:
    p = argparse.ArgumentParser(description="Generate Polish TTS clips via ElevenLabs.")
    p.add_argument("--voice", default=os.environ.get("ELEVENLABS_VOICE_ID", DEFAULT_VOICE))
    p.add_argument("--model", default=DEFAULT_MODEL)
    p.add_argument("--lufs", type=float, default=DEFAULT_LUFS,
                   help=f"integrated loudness target (default {DEFAULT_LUFS})")
    p.add_argument("--peak-db", type=float, default=DEFAULT_PEAK_DB,
                   help=f"true-peak ceiling in dBFS (default {DEFAULT_PEAK_DB})")
    p.add_argument("--compress", default=DEFAULT_COMPRESS,
                   help="ffmpeg filter applied before loudnorm; empty string to disable")
    p.add_argument("--list-voices", action="store_true", help="List voices and exit")
    args = p.parse_args()

    key = require_key()
    if args.list_voices:
        list_voices(key)
        return 0

    ffmpeg = require_tool("ffmpeg")
    OUT.mkdir(parents=True, exist_ok=True)

    for name, text in CLIPS.items():
        mp3 = synthesize_mp3(text, key=key, voice=args.voice, model=args.model)
        pcm = apply_fade(
            to_pcm(mp3, ffmpeg=ffmpeg, lufs=args.lufs, peak_db=args.peak_db,
                   compress=args.compress)
        )
        (OUT / name).write_bytes(pcm)

        peak, rms = measure(pcm)
        print(
            f"{name}: {len(pcm)} bytes  {len(pcm) / 2 / RATE:.2f}s  "
            f"peak={peak:.3f} rms={rms:.4f}  «{text}»"
        )

    print(f"\nWrote {len(CLIPS)} clips at {RATE} Hz to {OUT}")
    print("SOUND_SAMPLE_RATE in platform/anyka/sound.rs must match this rate.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
