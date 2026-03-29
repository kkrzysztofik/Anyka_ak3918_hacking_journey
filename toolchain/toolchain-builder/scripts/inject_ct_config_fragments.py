#!/usr/bin/env python3
"""Splice crosstool-NG .config fragments for pinned binutils / uClibc-ng versions."""

from __future__ import annotations

import pathlib
import re
import sys


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: inject_ct_config_fragments.py <path-to-.config>", file=sys.stderr)
        return 2
    cfg_path = pathlib.Path(sys.argv[1])
    script_dir = pathlib.Path(__file__).resolve().parent.parent
    text = cfg_path.read_text(encoding="utf-8")

    binutils_frag = (script_dir / "fragments" / "binutils-2.46.0.kconfig").read_text(
        encoding="utf-8"
    )
    uclibc_frag = (script_dir / "fragments" / "uClibc-ng-1.0.57.kconfig").read_text(
        encoding="utf-8"
    )

    patched, n_bin = re.subn(
        r"(CT_BINUTILS_PATCH_ORDER=.*\n)(?s:.*?)(\n#\n# GNU binutils\n)",
        r"\1" + binutils_frag + r"\2",
        text,
        count=1,
    )
    if n_bin != 1:
        print(
            "inject_ct_config_fragments: binutils block not found or ambiguous",
            file=sys.stderr,
        )
        return 1

    patched, n_lib = re.subn(
        r"(CT_UCLIBC_NG_PATCH_ORDER=.*\n)(?s:.*?)(\nCT_LIBC_UCLIBC_VERBOSITY)",
        r"\1" + uclibc_frag + r"\2",
        patched,
        count=1,
    )
    if n_lib != 1:
        print(
            "inject_ct_config_fragments: uClibc-ng block not found or ambiguous",
            file=sys.stderr,
        )
        return 1

    cfg_path.write_text(patched, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
