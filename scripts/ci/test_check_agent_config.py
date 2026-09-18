"""Tests for the agent-config guard."""

import tempfile
import unittest
from pathlib import Path

from check_agent_config import check_pi_settings, find_stub_symlinks, load_skills


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

    def test_load_skills_unterminated_frontmatter_raises(self):
        """`partition` returns the whole remainder when the closing --- is absent.

        Without an explicit check the entire body parses as frontmatter, so a
        malformed manifest passes here while every agent host rejects it.
        """
        with tempfile.TemporaryDirectory() as tmp:
            skill = Path(tmp) / "unclosed"
            skill.mkdir()
            (skill / "SKILL.md").write_text(
                "---\nname: unclosed\ndescription: Never closes.\n\nBody text.\n"
            )
            with self.assertRaises(ValueError):
                load_skills(Path(tmp))

    def test_load_skills_no_frontmatter_at_all_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            skill = Path(tmp) / "plain"
            skill.mkdir()
            (skill / "SKILL.md").write_text("# Just a heading\n\nNo frontmatter.\n")
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


class TestCheckPiSettings(unittest.TestCase):
    """The guard must read settings.json, not just probe a constant path.

    Every case here previously passed: the old check resolved a hard-coded
    path and never opened the file.
    """

    def _repo(self, tmp, settings_text=None):
        """Build a repo with .claude/skills/ present and optional .pi/settings.json."""
        root = Path(tmp)
        (root / ".claude" / "skills").mkdir(parents=True)
        if settings_text is not None:
            (root / ".pi").mkdir()
            (root / ".pi" / "settings.json").write_text(settings_text)
        return root

    def test_check_pi_settings_valid_returns_no_errors(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self._repo(tmp, '{"skills": ["../.claude/skills"]}')
            self.assertEqual(check_pi_settings(root), [])

    def test_check_pi_settings_missing_file_reports(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self._repo(tmp)
            self.assertIn("missing", " ".join(check_pi_settings(root)))

    def test_check_pi_settings_invalid_json_reports(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self._repo(tmp, '{"skills": [')
            self.assertIn("not valid JSON", " ".join(check_pi_settings(root)))

    def test_check_pi_settings_no_skills_key_reports(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self._repo(tmp, '{"theme": "dark"}')
            self.assertIn("no 'skills'", " ".join(check_pi_settings(root)))

    def test_check_pi_settings_typo_in_path_reports(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self._repo(tmp, '{"skills": ["../.calude/skills"]}')
            joined = " ".join(check_pi_settings(root))
            self.assertIn("does not resolve", joined)

    def test_check_pi_settings_points_elsewhere_reports(self):
        """Resolves to a real directory, but not the project skills dir."""
        with tempfile.TemporaryDirectory() as tmp:
            root = self._repo(tmp, '{"skills": ["/tmp"]}')
            self.assertIn("does not point at", " ".join(check_pi_settings(root)))

    def test_check_pi_settings_skills_not_a_list_reports(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self._repo(tmp, '{"skills": "../.claude/skills"}')
            self.assertIn("must be a list", " ".join(check_pi_settings(root)))


if __name__ == "__main__":
    unittest.main()
