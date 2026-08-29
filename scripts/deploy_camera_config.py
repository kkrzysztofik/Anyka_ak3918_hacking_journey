#!/usr/bin/env python3
"""Bring one camera's device-local config up to the fleet standard.

Idempotent: each edit is skipped when already satisfied, so running this against
an already-correct camera is a no-op that reports what it found.

Edits, all in files a bundle never overwrites:
  A. append [sound] + [sound.events]           (event audio playback)
  B. stream_profile_1.audio_enabled = true     (audio capture on /main)
  C. upload snmp.toml when absent              (gives WebUI edits a home)

Stdlib only: it has to run on the jumphost, which has python3 and nothing else.
"""

import argparse
import difflib
import hashlib
import io
import os
import sys
import tomllib
from ftplib import FTP, error_perm

CONFIG_PATH = "/mnt/anyka_hack/onvif/config.toml"
BACKUP_PATH = "/mnt/anyka_hack/onvif/config.toml.pre-6afa26f4"
SNMP_PATH = "/mnt/anyka_hack/snmp.toml"

SOUND_BLOCK = """
[sound]
enabled = true
clip_dir = "sounds"
volume = 3
debounce_secs = 30

[sound.events]
boot_ready = "boot.raw"
network_lost = "alert.raw"
network_up = "ok.raw"
upgrade_result = "upgrade.raw"
"""

EXPECTED_EVENTS = {"boot_ready", "network_lost", "network_up", "upgrade_result"}


def add_sound(text):
    """Append the [sound] tables unless a [sound] table already parses out.

    Appending a new top-level table at EOF is positionally unambiguous in TOML,
    so this needs no knowledge of what the file currently ends with.
    """
    if "sound" in tomllib.loads(text):
        return text, False
    if not text.endswith("\n"):
        text += "\n"
    return text + SOUND_BLOCK, True


def set_audio_enabled(text):
    """Set audio_enabled = true inside [stream_profile_1] only.

    Textual, not parse-and-reserialise: tomllib is read-only, and any writer
    would reflow the file and destroy its comments. Correctness comes from
    validate() re-parsing the result, not from this function being clever.
    """
    out, changed, in_section = [], False, False
    for line in text.splitlines(keepends=True):
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            in_section = stripped == "[stream_profile_1]"
        elif in_section and stripped.split("=", 1)[0].strip() == "audio_enabled":
            # Exact key match, not startswith: a prefix test would rewrite a
            # sibling like `audio_enabled_extra` into `audio_enabled = true`,
            # destroying its value -- and validate() could not tell, because the
            # key it asserts on would then be trivially satisfied.
            if stripped != "audio_enabled = true":
                line, changed = "audio_enabled = true\n", True
        out.append(line)
    return "".join(out), changed


def validate(text):
    """Parse the edited file and assert the edits landed where intended.

    Read-only parsing proves both that the file is still valid TOML and that the
    keys hold the intended values -- which is the entire risk, since a config
    onvif-rust cannot parse costs a reboot and an automatic revert.
    """
    cfg = tomllib.loads(text)
    if cfg["stream_profile_1"]["audio_enabled"] is not True:
        raise ValueError("stream_profile_1.audio_enabled is not true")
    if cfg["sound"]["enabled"] is not True:
        raise ValueError("sound.enabled is not true")
    if set(cfg["sound"]["events"]) != EXPECTED_EVENTS:
        raise ValueError(f"sound.events keys are {set(cfg['sound']['events'])}")
    return cfg


def self_test():
    """Runnable check for the two non-trivial edits. No network, no fixtures."""
    base = (
        '[stream_profile_1]\nname = "Profile1"\naudio_enabled = false\n'
        'audio_bitrate = 64\n\n[stream_profile_2]\naudio_enabled = false\n'
    )
    edited, changed = set_audio_enabled(base)
    assert changed
    cfg = tomllib.loads(edited)
    assert cfg["stream_profile_1"]["audio_enabled"] is True
    # The sibling profile must be untouched -- this is the whole point of
    # scoping the replace to a section rather than using a global substitution.
    assert cfg["stream_profile_2"]["audio_enabled"] is False
    assert "audio_bitrate = 64" in edited, "unrelated keys must survive"

    again, changed = set_audio_enabled(edited)
    assert not changed and again == edited, "must be idempotent"

    # A sibling key that merely starts with "audio_enabled" must survive
    # untouched. A startswith() test silently rewrites it to
    # `audio_enabled = true` and destroys its value, and validate() cannot
    # catch that -- the key it asserts on ends up trivially satisfied.
    sibling = '[stream_profile_1]\naudio_enabled_extra = 5\naudio_enabled = false\n'
    out, changed = set_audio_enabled(sibling)
    assert changed
    parsed = tomllib.loads(out)
    assert parsed["stream_profile_1"]["audio_enabled_extra"] == 5
    assert parsed["stream_profile_1"]["audio_enabled"] is True

    withsound, changed = add_sound(base)
    assert changed
    assert tomllib.loads(withsound)["sound"]["events"]["boot_ready"] == "boot.raw"
    again, changed = add_sound(withsound)
    assert not changed and again == withsound, "must be idempotent"

    validate(withsound.replace("audio_enabled = false", "audio_enabled = true", 1))

    try:
        validate(base)
    except (KeyError, ValueError):
        pass
    else:
        raise AssertionError("validate() must reject an unedited config")

    print("self-test OK")


def get(ftp, path):
    buf = io.BytesIO()
    ftp.retrbinary("RETR " + path, buf.write)
    return buf.getvalue()


def put(ftp, path, payload):
    """Upload, then read back and compare digests.

    Not paranoia: these cameras have silently written NUL bytes on exFAT before,
    and a truncated config.toml is indistinguishable from a good one until the
    next boot fails.
    """
    ftp.storbinary("STOR " + path, io.BytesIO(payload))
    back = get(ftp, path)
    if hashlib.sha256(back).hexdigest() != hashlib.sha256(payload).hexdigest():
        raise IOError(f"readback mismatch on {path}: wrote {len(payload)} B, "
                      f"read {len(back)} B")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("host", nargs="?", help="omit only with --self-test")
    ap.add_argument("--user", default="root")
    ap.add_argument("--snmp-toml", default=None,
                    help="local snmp.toml to upload when the camera lacks one")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()

    if args.self_test:
        self_test()
        return 0
    if not args.host:
        ap.error("host is required unless --self-test is given")

    password = os.environ.get("ANYKA_FTP_PASS")
    if not password:
        print("ANYKA_FTP_PASS is not set", file=sys.stderr)
        return 2

    # login() inside the try: FTP() has already opened the control connection,
    # so an auth failure or a dropped link mid-login would otherwise skip the
    # finally and leak the socket for the life of the process.
    ftp = FTP(args.host, timeout=30)
    try:
        ftp.login(args.user, password)
        original = get(ftp, CONFIG_PATH)
        text = original.decode()

        text, sound_changed = add_sound(text)
        text, audio_changed = set_audio_enabled(text)
        validate(text)

        print(f"{args.host}: sound={'ADD' if sound_changed else 'present'} "
              f"audio_in={'ENABLE' if audio_changed else 'already on'}")

        if sound_changed or audio_changed:
            diff = difflib.unified_diff(
                original.decode().splitlines(), text.splitlines(),
                "before", "after", lineterm="", n=1)
            print("\n".join(diff))

        if args.dry_run:
            print(f"{args.host}: dry run, nothing written")
            return 0

        if sound_changed or audio_changed:
            put(ftp, BACKUP_PATH, original)
            try:
                put(ftp, CONFIG_PATH, text.encode())
            except Exception:
                # The backup above is already written and readback-verified, so
                # recovery is always possible -- but only if whoever is watching
                # knows before something reboots the camera into a config that
                # may be half-written. A bare traceback does not say that.
                print(
                    f"\n!!! {args.host}: config.toml WRITE FAILED and may be "
                    f"truncated on the device.\n"
                    f"!!! DO NOT REBOOT THIS CAMERA.\n"
                    f"!!! Restore it by hand from {BACKUP_PATH}, then verify "
                    f"before any reboot or upgrade.\n",
                    file=sys.stderr)
                raise
            print(f"{args.host}: config.toml written and verified "
                  f"(backup at {BACKUP_PATH})")

        if args.snmp_toml:
            # Probe with RETR rather than NLST/SIZE: busybox ftpd's support for
            # the latter two is not guaranteed, and RETR is already proven to
            # work by the config fetch above.
            try:
                get(ftp, SNMP_PATH)
                present = True
            except error_perm:
                present = False
            if present:
                print(f"{args.host}: snmp.toml already present")
            else:
                with open(args.snmp_toml, "rb") as fh:
                    put(ftp, SNMP_PATH, fh.read())
                print(f"{args.host}: snmp.toml written and verified")
    finally:
        ftp.quit()
    return 0


if __name__ == "__main__":
    sys.exit(main())
