# Firmware upgrade — reference

## On-disk layout (`/mnt/anyka_hack`)

```text
active                     # "a" or "b"
slots/{a,b}/
  manifest.sha256          # sha256sum -c format
  manifest.meta            # version=, requires_config_schema=
  anyka-init.bin
  vendor-daemon/
  onvif/{onvif-rust.bin, www/, config.template.toml}
anyka.toml                 # device-local — never in bundle
onvif/config.toml          # device-local — never in bundle
lib/                       # shared uClibc — outside slots/bundle
state/
  trial-<slot>             # exists ⇒ unconfirmed update; name = previous slot
spool/
  bundle.tar
  bundle.trigger           # written last; presence means apply
```

## Apply / trial (summary)

1. Trigger present → untar into inactive slot → `sha256sum -c` → schema check
2. Failure → clear spool, wipe staging, **no flip**
3. Success → `state/trial-<current>`, write `active=<inactive>`, reboot
4. Boot with trial → ports 80/554/8080 LISTEN for hold window → unlink trial (commit) or flip back + reboot (revert)

## Failure matrix

| Failure | Caught by | Result |
|---|---|---|
| Corrupt/truncated transfer | sha256 pre-flip | Both slots intact |
| Schema mismatch | `requires_config_schema` vs `anyka.toml` | Both slots intact |
| New build crashloops / port missing | trial | Revert to previous slot |
| New `anyka-init` will not exec | `config.sh` other-slot fallback | Previous slot |
| Both slots dead | `config.sh` deadman | Vendor boot path |

## HTTP status cheat sheet (`PUT /api/update`)

| Code | Meaning |
|---|---|
| 202 | Queued (`bundle.trigger` written) |
| 401 | Not Administrator / bad Basic auth |
| 409 | In-flight `.part` or trigger already present |
| 413 | Over `MAX_BUNDLE_BYTES` (64 MiB) |
| 500 | I/O / spool failure |

## Bundle contents

Produced by `scripts/build_bundle.sh` from `SD_card_contents/anyka_hack/`:

- `anyka-init.bin`, `vendor-daemon/`, `onvif/onvif-rust.bin`, `onvif/www/`, `onvif/config.template.toml`
- `manifest.sha256`, `manifest.meta` (`version` from `git describe`, `requires_config_schema`)

Not included: `lib/`, live `anyka.toml`, live `onvif/config.toml`.
