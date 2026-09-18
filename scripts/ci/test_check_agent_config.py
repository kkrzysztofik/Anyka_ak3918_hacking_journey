"""Tests for the agent-config guard."""

import tempfile
import unittest
from pathlib import Path

from check_agent_config import find_stub_symlinks, load_skills


class TestLoadSkills(unittest.TestCase):
    def test_load_skills_valid_dir_returns_name_and_description(self):
        with tempfile.TemporaryDirectory() as tmp:
            skill = Path(tmp) / "my-skill"
            skill.mkdir()
            (skill / "SKILL.md").write_text(
                "---\nname: my-skill\ndescription: Use when testing.\n---\n\nBody.\n"
            )
            self.assertEqual(load_skills(Path(tmp)), {"my-skill": "Use when testing."})

    def test_load_skills_missing_description_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            skill = Path(tmp) / "bad-skill"
            skill.mkdir()
            (skill / "SKILL.md").write_text("---\nname: bad-skill\n---\n\nBody.\n")
            with self.assertRaises(ValueError):
                load_skills(Path(tmp))

    def test_load_skills_name_mismatch_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            skill = Path(tmp) / "dir-name"
            skill.mkdir()
            (skill / "SKILL.md").write_text(
                "---\nname: other-name\ndescription: x\n---\n"
            )
            with self.assertRaises(ValueError):
                load_skills(Path(tmp))

    def test_load_skills_missing_manifest_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "empty-skill").mkdir()
            with self.assertRaises(ValueError):
                load_skills(Path(tmp))

    def test_load_skills_ignores_loose_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "README.md").write_text("# Not a skill\n")
            self.assertEqual(load_skills(Path(tmp)), {})


class TestFindStubSymlinks(unittest.TestCase):
    def test_find_stub_symlinks_detects_path_committed_as_text(self):
        with tempfile.TemporaryDirectory() as tmp:
            stub = Path(tmp) / "some-skill"
            stub.write_text("../../anyka-dev/.claude/skills/some-skill")
            self.assertEqual(find_stub_symlinks(Path(tmp)), [stub])

    def test_find_stub_symlinks_ignores_real_content(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "notes.md").write_text("# Real file\n\nWith prose.\n")
            self.assertEqual(find_stub_symlinks(Path(tmp)), [])

    def test_find_stub_symlinks_ignores_working_symlink(self):
        with tempfile.TemporaryDirectory() as tmp:
            target = Path(tmp) / "target"
            target.mkdir()
            link = Path(tmp) / "link"
            link.symlink_to(target)
            self.assertEqual(find_stub_symlinks(Path(tmp)), [])

    def test_find_stub_symlinks_ignores_node_modules(self):
        """Minified sourcemaps match the stub signature but are never tracked."""
        with tempfile.TemporaryDirectory() as tmp:
            vendored = Path(tmp) / "node_modules" / "effect" / "dist"
            vendored.mkdir(parents=True)
            (vendored / "HKT.js.map").write_text(
                '{"version":3,"sources":["../../src/HKT.ts"],"mappings":""}'
            )
            self.assertEqual(find_stub_symlinks(Path(tmp)), [])

    def test_find_stub_symlinks_ignores_single_word_no_slash(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "VERSION").write_text("1.2.3\n")
            self.assertEqual(find_stub_symlinks(Path(tmp)), [])


if __name__ == "__main__":
    unittest.main()
