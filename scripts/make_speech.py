#!/usr/bin/env python3
"""Generate Polish TTS PCM clips (s16le mono, 8 kHz) for event audio.

Host-side only — requires espeak-ng and ffmpeg on the build machine. Output
matches the vendor-daemon sound worker format verified on hardware.

Usage:
    python3 scripts/make_speech.py
    python3 scripts/make_speech.py --list-voices
"""
from __future__ import annotations

import argparse
import math
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

RATE = 8000
OUT = Path(__file__).parent.parent / "SD_card_contents/anyka_hack/onvif/sounds"

# Short phrases for a small speaker; filenames match [sound.events] in config.toml.
CLIPS: dict[str, str] = {
    "boot.raw": "Kamera gotowa.",
    "alert.raw": "Utracono połączenie z siecią.",
    "ok.raw": "Połączenie sieciowe przywrócone.",
    "upgrade.raw": "Aktualizacja zakończona pomyślnie.",
}

VOICE = "pl"
SPEAK_RATE = 130  # words per minute; slow enough for clarity on the DAC speaker
SPEAK_AMP = 120  # espeak-ng amplitude (0–200); keep below demo's deafening max


def require_tool(name: str) -> str:
    path = shutil.which(name)
    if path is None:
        sys.exit(
            f"{name} not found. Install on the build host, e.g.\n"
            f"  sudo apt-get install -y espeak-ng ffmpeg"
        )
    return path


def list_polish_voices(espeak: str) -> None:
    result = subprocess.run(
        [espeak, "--voices=pl"],
        check=True,
        capture_output=True,
        text=True,
    )
    print(result.stdout)


def apply_fade(pcm: bytes, rate: int = RATE, fade_ms: int = 25) -> bytes:
    """Short linear fade in/out to avoid DAC clicks at clip boundaries."""
    samples = len(pcm) // 2
    if samples == 0:
        return pcm
    fade = min(int(rate * fade_ms / 1000), samples // 4)
    if fade == 0:
        return pcm

    out = bytearray(pcm)
    for i in range(fade):
        env_in = i / fade
        env_out = (fade - i) / fade
        for idx, env in ((i, env_in), (samples - 1 - i, env_out)):
            offset = idx * 2
            sample = struct.unpack_from("<h", out, offset)[0]
            struct.pack_into("<h", out, offset, int(sample * env))
    return bytes(out)


def synthesize(
    text: str,
    *,
    espeak: str,
    ffmpeg: str,
    voice: str,
    rate_wpm: int,
    amplitude: int,
) -> bytes:
    with tempfile.TemporaryDirectory(prefix="anyka-speech-") as tmp:
        wav = Path(tmp) / "clip.wav"
        raw = Path(tmp) / "clip.raw"

        subprocess.run(
            [
                espeak,
                "-v",
                voice,
                "-s",
                str(rate_wpm),
                "-a",
                str(amplitude),
                "-w",
                str(wav),
                text,
            ],
            check=True,
        )

        subprocess.run(
            [
                ffmpeg,
                "-y",
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                str(wav),
                "-af",
                "aresample=8000:resampler=soxr",
                "-ac",
                "1",
                "-f",
                "s16le",
                str(raw),
            ],
            check=True,
        )

        return apply_fade(raw.read_bytes())


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate Polish TTS sound clips.")
    parser.add_argument(
        "--voice",
        default=VOICE,
        help=f"espeak-ng voice (default: {VOICE})",
    )
    parser.add_argument(
        "--rate",
        type=int,
        default=SPEAK_RATE,
        help=f"espeak-ng speed in WPM (default: {SPEAK_RATE})",
    )
    parser.add_argument(
        "--list-voices",
        action="store_true",
        help="List installed Polish espeak-ng voices and exit",
    )
    args = parser.parse_args()

    espeak = require_tool("espeak-ng")
    if args.list_voices:
        list_polish_voices(espeak)
        return 0
    ffmpeg = require_tool("ffmpeg")

    OUT.mkdir(parents=True, exist_ok=True)
    for name, text in CLIPS.items():
        data = synthesize(
            text,
            espeak=espeak,
            ffmpeg=ffmpeg,
            voice=args.voice,
            rate_wpm=args.rate,
            amplitude=SPEAK_AMP,
        )
        (OUT / name).write_bytes(data)
        duration_s = len(data) / 2 / RATE
        print(f"{name}: {len(data)} bytes ({duration_s:.2f}s)  «{text}»")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
