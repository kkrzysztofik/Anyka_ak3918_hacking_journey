#!/usr/bin/env python3
"""Guard the shared agent configuration.

`.claude/skills/` is the single source of truth for every agent host used on
this repo: Claude Code and opencode read it natively, pi is pointed at it by
`.pi/settings.json`.

Two failure modes this catches, both silent in every host:

1. Skill discovery dropping to zero. A host that finds no skills prints no
   error, it just stops offering them. Assert a count, never an error string.
2. Symlinks committed as regular files. Git stores a symlink as mode 120000; a
   checkout without symlink support writes mode 100644 with the target as the
   file's content. That is what killed `.opencode/skills/` for two months.
"""

import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SKILLS_DIR = REPO_ROOT / ".claude" / "skills"
PI_SETTINGS = REPO_ROOT / ".pi" / "settings.json"

MIN_SKILLS = 11

# A stub is short, single-line, and looks like a path. Real prose is not.
STUB_MAX_BYTES = 256

# Minified sourcemaps match the stub signature exactly: one line, slashes, no
# spaces. They are also untracked, so they can never carry a committed stub.
EXCLUDE_DIRS = frozenset({"node_modules", ".git"})


def _frontmatter(text):
    """Return the frontmatter block as a flat dict, or None if malformed.

    The closing delimiter has to be checked explicitly: `partition` returns the
    entire remaining text when its separator is absent, so an unterminated
    block would parse the whole document body as frontmatter and pass
    validation that every agent host would fail.
    """
    if not text.startswith("---"):
        return None
    _, _, rest = text.partition("---")
    block, delimiter, _ = rest.partition("---")
    if not delimiter:
        return None
    fields = {}
    for line in block.splitlines():
        key, sep, value = line.partition(":")
        if sep:
            fields[key.strip()] = value.strip()
    return fields


def load_skills(skills_dir):
    """Map skill name -> description, validating each SKILL.md."""
    skills = {}
    for entry in sorted(skills_dir.iterdir()):
        if not entry.is_dir():
            continue
        manifest = entry / "SKILL.md"
        if not manifest.is_file():
            raise ValueError(f"{entry} has no SKILL.md")
        fields = _frontmatter(manifest.read_text(encoding="utf-8"))
        if fields is None:
            raise ValueError(f"{manifest} has no '---' delimited frontmatter block")
        name = fields.get("name")
        description = fields.get("description")
        if not description:
            raise ValueError(f"{manifest} has no description; hosts filter it out")
        if name != entry.name:
            raise ValueError(f"{manifest} name '{name}' != directory '{entry.name}'")
        skills[name] = description
    return skills


def find_stub_symlinks(root):
    """Find regular files whose entire content is a filesystem path."""
    stubs = []
    for path in sorted(root.rglob("*")):
        if EXCLUDE_DIRS.intersection(path.parts):
            continue
        if path.is_symlink() or not path.is_file():
            continue
        if path.stat().st_size > STUB_MAX_BYTES:
            continue
        try:
            text = path.read_text(encoding="utf-8").strip()
        except (UnicodeDecodeError, OSError):
            continue
        if not text or "\n" in text:
            continue
        if "/" in text and " " not in text:
            stubs.append(path)
    return stubs


def check_pi_settings(repo_root):
    """Verify .pi/settings.json actually points pi at the project skills.

    Reads the file rather than probing a constant path: invalid JSON, a missing
    `skills` key and a typo'd path all leave pi discovering nothing, and all
    three look identical from the outside.
    """
    settings = repo_root / ".pi" / "settings.json"
    skills_dir = (repo_root / ".claude" / "skills").resolve()

    if not settings.is_file():
        return [f"{settings} is missing; pi will not see project skills"]

    try:
        data = json.loads(settings.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        return [f"{settings} is not valid JSON: {exc}"]

    if not isinstance(data, dict) or "skills" not in data:
        return [f"{settings} has no 'skills' key; pi will not see project skills"]

    entries = data["skills"]
    if not isinstance(entries, list):
        return [f"{settings} 'skills' must be a list, got {type(entries).__name__}"]

    # Paths are relative to the .pi/ directory, per pi's docs.
    for entry in entries:
        resolved = (settings.parent / str(entry)).resolve()
        if not resolved.is_dir():
            continue
        if resolved == skills_dir:
            return []

    listed = ", ".join(repr(e) for e in entries) or "nothing"
    if any((settings.parent / str(e)).resolve().is_dir() for e in entries):
        return [f"{settings} does not point at {skills_dir}; it lists {listed}"]
    return [f"{settings} path does not resolve: it lists {listed}"]


def main():
    errors = []

    try:
        skills = load_skills(SKILLS_DIR)
    except ValueError as exc:
        print(f"FAIL: {exc}")
        return 1

    if len(skills) < MIN_SKILLS:
        errors.append(
            f"expected >= {MIN_SKILLS} skills in {SKILLS_DIR}, found {len(skills)}"
        )

    for name in (".claude", ".opencode", ".pi"):
        directory = REPO_ROOT / name
        if not directory.is_dir():
            continue
        for stub in find_stub_symlinks(directory):
            errors.append(f"{stub} looks like a symlink committed as text")

    errors.extend(check_pi_settings(REPO_ROOT))

    for error in errors:
        print(f"FAIL: {error}")
    if errors:
        return 1

    print(f"OK: {len(skills)} skills, no stub symlinks, pi wired up")
    return 0


if __name__ == "__main__":
    sys.exit(main())
