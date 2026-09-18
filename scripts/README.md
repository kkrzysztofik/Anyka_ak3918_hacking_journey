# scripts/

Names follow `<stage>_<unit>`. The stage tells you what the script *does*; the
unit tells you what it operates on.

| Stage | Meaning |
|---|---|
| `build_` | compiles sources. Touches no camera. |
| `package_` | tars what is already on disk. Compiles nothing. |
| `push_` | sends to a card or camera. Compiles and packages nothing. |
| `run_` | executes something already on the device. |

## Getting code onto a camera

```
build_bundle.sh  =  build_payload.sh  +  package_bundle.sh
                         (compile)        (tar + manifest)
```

| Script | Stage | Produces | Use for |
|---|---|---|---|
| **`build_bundle.sh`** | build + package | `bundle.tar` | **The default.** Any change you want on a camera. |
| **`push_bundle.sh`** | push | `PUT /api/update` | The normal upgrade: A/B slots, trial window, automatic rollback. |
| `build_payload.sh` | build | `SD_card_contents/anyka_hack/` | Fresh binaries, without packaging. |
| `package_bundle.sh` | package | `bundle.tar` | Re-tarring an *already fresh* payload. See the warning below. |
| `push_payload.sh` | push | card or `/mnt/...` over FTP | Fresh camera, or a change to `lib/` or `Factory/` that a bundle cannot carry. No versioning or rollback. |
| `push_config.py` | push | device-local config | Bringing one camera's config to the fleet standard. |
| `migrate_to_slots.sh` | — | slot layout | Once per camera: moves a flat install onto A/B slots. |

### The one trap worth knowing

`package_bundle.sh` compiles nothing. On a stale `SD_card_contents/` it still
succeeds — the version-embed check compares the manifest against the binary, and
an old `.build-version` agrees with the old binary it was built beside. You get a
valid bundle carrying last week's work under last week's label. Nothing fails.

**If you have touched any source, run `build_bundle.sh`.** It builds first.

### Dev-only

`push_binary_dev.sh` and `run_binary_dev.sh` push and launch a single binary over
FTP/telnet. No versioning, no slot, no trial, no rollback — for tightening a
debug loop, never for shipping. Both warn at startup.

## Other

| Script | Purpose |
|---|---|
| `common.sh` | shared logging and repo-root helpers; sourced, not run |
| `camera_ntp_sync.py` | clock sync, and the shared telnet client |
| `make_speech.py` | TTS audio generation |
| `cloc_our_code.sh` | line counts for our own code |
| `ci/check_agent_config.py` | guards skill discovery across agent hosts |
| `debugging/` | `cam_exec.py` (telnet shell), coredump collection, gdb analysis |

Full procedures live in the `anyka-firmware-upgrade` and `anyka-remote-debugging`
skills under `.claude/skills/`.
