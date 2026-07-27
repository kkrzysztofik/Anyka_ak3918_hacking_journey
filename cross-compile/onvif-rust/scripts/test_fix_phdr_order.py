#!/usr/bin/env python3
"""Tests for fix_phdr_order.py.

Builds minimal synthetic "ELF32" files containing just the bytes
fix_phdr_order() actually reads (e_ident, e_phoff, e_phentsize, e_phnum,
and p_type per program header), rather than full valid ELF binaries.
"""
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from fix_phdr_order import fix_phdr_order  # noqa: E402

PT_NULL = 0
PT_DYNAMIC = 2
PT_TLS = 7
PHENTSIZE = 8
PHOFF = 64


def build_fake_elf32(p_types):
    """Build a minimal file with a program header table listing `p_types` in order."""
    buf = bytearray(PHOFF)
    buf[0:4] = b'\x7fELF'
    buf[4] = 1  # EI_CLASS = ELFCLASS32
    struct.pack_into('<I', buf, 28, PHOFF)
    struct.pack_into('<H', buf, 42, PHENTSIZE)
    struct.pack_into('<H', buf, 44, len(p_types))
    for i, p_type in enumerate(p_types):
        entry = struct.pack('<I', p_type) + b'\x00' * (PHENTSIZE - 4)
        buf.extend(entry)
    return bytes(buf)


def write_temp_file(data):
    f = tempfile.NamedTemporaryFile(delete=False)
    f.write(data)
    f.close()
    return f.name


class FixPhdrOrderTests(unittest.TestCase):
    def test_rejects_non_elf_file(self):
        path = write_temp_file(b'not an elf file at all')
        self.assertIsNone(fix_phdr_order(path))

    def test_rejects_non_elf32_class(self):
        buf = bytearray(b'\x7fELF')
        buf.append(2)  # ELFCLASS64
        path = write_temp_file(bytes(buf))
        self.assertIsNone(fix_phdr_order(path))

    def test_no_phdrs_of_interest_is_noop(self):
        path = write_temp_file(build_fake_elf32([PT_NULL, PT_NULL]))
        self.assertFalse(fix_phdr_order(path, verbose=True))

    def test_dynamic_already_before_tls_is_noop(self):
        data = build_fake_elf32([PT_DYNAMIC, PT_TLS])
        path = write_temp_file(data)
        self.assertFalse(fix_phdr_order(path, verbose=True))
        # File must be unchanged.
        self.assertEqual(Path(path).read_bytes(), data)

    def test_tls_before_dynamic_gets_swapped(self):
        path = write_temp_file(build_fake_elf32([PT_TLS, PT_DYNAMIC]))
        self.assertTrue(fix_phdr_order(path, verbose=True))

        with open(path, 'rb') as f:
            f.seek(PHOFF)
            first_type = struct.unpack('<I', f.read(4))[0]
            f.seek(PHOFF + PHENTSIZE)
            second_type = struct.unpack('<I', f.read(4))[0]
        self.assertEqual(first_type, PT_DYNAMIC)
        self.assertEqual(second_type, PT_TLS)

    def test_main_rejects_invalid_elf_with_nonzero_exit(self):
        path = write_temp_file(b'garbage')
        result = subprocess.run(
            [sys.executable, str(Path(__file__).parent / 'fix_phdr_order.py'), path],
            capture_output=True,
        )
        self.assertEqual(result.returncode, 1)

    def test_main_succeeds_on_already_correct_binary(self):
        path = write_temp_file(build_fake_elf32([PT_DYNAMIC, PT_TLS]))
        result = subprocess.run(
            [sys.executable, str(Path(__file__).parent / 'fix_phdr_order.py'), path, '--verbose'],
            capture_output=True,
        )
        self.assertEqual(result.returncode, 0)

    def test_main_missing_argument_exits_nonzero(self):
        result = subprocess.run(
            [sys.executable, str(Path(__file__).parent / 'fix_phdr_order.py')],
            capture_output=True,
        )
        self.assertEqual(result.returncode, 1)


if __name__ == '__main__':
    unittest.main()
