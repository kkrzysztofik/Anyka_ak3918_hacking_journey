#!/usr/bin/env python3
"""Fix program header ordering for uClibc-ng compatibility.

uClibc-ng 1.0.54 ldso.c line 761 has a `break` inside the PT_TLS handler
that exits the program header iteration loop early. If PT_TLS appears before
PT_DYNAMIC in the program header table (as LLD orders them), the executable
is never registered in _dl_loaded_modules and the linker crashes with SIGSEGV
at _dl_get_ready_to_run+1232 (NULL deref: ldr r2, [r7, #60] where r7=0).

GNU ld places PT_DYNAMIC before PT_TLS, so the bug is harmless there.
LLD (used by Clang/Rust) places PT_TLS before PT_DYNAMIC, triggering the bug.

This script swaps the PT_TLS and PT_DYNAMIC entries in the ELF program header
table so PT_DYNAMIC is processed first by the uClibc-ng dynamic linker.

Safe to run on binaries that don't need fixing (it checks the order first).
"""
import struct
import sys


def fix_phdr_order(path, verbose=False):
    """Ensure PT_DYNAMIC appears before PT_TLS in program headers.

    Returns True if the file was modified, False if no change was needed.
    """
    PT_DYNAMIC = 2
    PT_TLS = 7

    with open(path, 'rb') as f:
        magic = f.read(4)
        if magic != b'\x7fELF':
            print(f"ERROR: {path} is not an ELF file", file=sys.stderr)
            return False

        ei_class = struct.unpack('B', f.read(1))[0]
        if ei_class != 1:
            print(f"ERROR: expected ELF32, got class {ei_class}", file=sys.stderr)
            return False

        f.seek(28)
        phoff = struct.unpack('<I', f.read(4))[0]
        f.seek(42)
        phentsize = struct.unpack('<H', f.read(2))[0]
        phnum = struct.unpack('<H', f.read(2))[0]

        tls_idx = -1
        dyn_idx = -1

        for i in range(phnum):
            f.seek(phoff + i * phentsize)
            p_type = struct.unpack('<I', f.read(4))[0]
            if p_type == PT_TLS:
                tls_idx = i
            elif p_type == PT_DYNAMIC:
                dyn_idx = i

        if tls_idx == -1 or dyn_idx == -1:
            if verbose:
                print(f"No PT_TLS or PT_DYNAMIC found — skipping")
            return False

        if dyn_idx < tls_idx:
            if verbose:
                print(f"PT_DYNAMIC (idx {dyn_idx}) already before PT_TLS (idx {tls_idx}) — OK")
            return False

        # Read both entries
        tls_off = phoff + tls_idx * phentsize
        dyn_off = phoff + dyn_idx * phentsize

        f.seek(tls_off)
        tls_data = f.read(phentsize)
        f.seek(dyn_off)
        dyn_data = f.read(phentsize)

    # Swap
    with open(path, 'r+b') as f:
        f.seek(tls_off)
        f.write(dyn_data)
        f.seek(dyn_off)
        f.write(tls_data)

    if verbose:
        print(f"Swapped PT_DYNAMIC (was idx {dyn_idx}) and PT_TLS (was idx {tls_idx})")
        print(f"PT_DYNAMIC now at idx {tls_idx}, PT_TLS now at idx {dyn_idx}")

    return True


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <elf-binary> [--verbose]")
        print()
        print("Swaps PT_TLS and PT_DYNAMIC program header entries if needed")
        print("to work around uClibc-ng 1.0.54 ldso.c break-on-TLS bug.")
        sys.exit(1)

    path = sys.argv[1]
    verbose = '--verbose' in sys.argv
    modified = fix_phdr_order(path, verbose=True)
    sys.exit(0 if not modified else 0)
