#!/usr/bin/env python3
"""Rewrite Cobertura XML paths to be relative to the git repository root.

llvm-cov (run from cross-compile/) emits filenames like onvif-rust/src/... with
<source>/workspace/cross-compile</source>. Sonar expects repo-root-relative
paths under sonar.sources=cross-compile.
"""

from __future__ import annotations

import argparse
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


def rewrite_cobertura(path: Path, prefix: str = "cross-compile/") -> None:
    """Normalize Cobertura XML so Sonar can resolve filenames under sonar.sources."""
    tree = ET.parse(path)
    root = tree.getroot()

    sources = root.find("sources")
    if sources is not None:
        for source in list(sources):
            sources.remove(source)
        source_el = ET.SubElement(sources, "source")
        source_el.text = "."

    for cls in root.iter("class"):
        filename = cls.get("filename")
        if filename and not filename.startswith(prefix):
            cls.set("filename", f"{prefix}{filename}")

    tree.write(path, encoding="utf-8", xml_declaration=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "cobertura_xml",
        type=Path,
        help="Path to cobertura.xml (e.g. coverage/cobertura.xml)",
    )
    parser.add_argument(
        "--prefix",
        default="cross-compile/",
        help="Path prefix to prepend to class filenames (default: cross-compile/)",
    )
    args = parser.parse_args()

    if not args.cobertura_xml.is_file():
        print(f"error: file not found: {args.cobertura_xml}", file=sys.stderr)
        return 1

    rewrite_cobertura(args.cobertura_xml, prefix=args.prefix)
    print(f"Rewrote {args.cobertura_xml} paths to repo-root-relative ({args.prefix})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
