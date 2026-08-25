# SNMP Integration — Design

Date: 2026-08-25
Status: approved (design)
Branch / worktree: `feat/snmp` @ `.worktrees/snmp` (from `main`)

## Problem

The camera has no SNMP agent. NMS tools cannot poll identity, uptime, or
interfaces. ONVIF `Get/SetNetworkProtocols` today cover HTTP / HTTPS / RTSP only.
Memory is tight (~24MB), so a full SNMP stack or in-process agent sharing fate
with ONVIF/streaming is the wrong default.

## Goals

1. **Read-only SNMPv2c agent** for NMS GET/GETNEXT (MIB-II `system` +
   basic `interfaces`).
2. **Separate binary** supervised by `anyka-init` (crash isolation from ONVIF
   and from the supervisor itself).
3. **Product surface in v1**: `snmp.toml` + ONVIF NetworkProtocols + WebUI.
4. **Defaults**: enabled, port 161, RO community `public` (documented as weak).
5. **Room for SNMPv3 later** without rewriting process layout or config ownership.

## Non-goals (v1)

- Traps / informs
- SET / write community
- SNMPv3 USM
- Private enterprise MIB beyond a fixed `sysObjectID`
- AgentX, BusyBox/`net-snmp` embedding
- Management VRF / bind-to-interface policy beyond “all interfaces”

## Decisions

| Topic | Choice |
| --- | --- |
| Role | Read-only agent (NMS polls) |
| Version | SNMPv2c now; design seam for v3 later |
| MIB | Minimal MIB-II `system` + basic `ifTable` |
| Process | Separate `snmp-agent` binary |
| Supervision | `anyka-init` `[services.snmp]` |
| Config UX | `snmp.toml` + ONVIF + WebUI |
| Default | `enabled=true`, `port=161`, `community="public"` |
| Implementation | Hand-rolled minimal responder (Approach A) |

## Architecture

```text
┌─────────────┐  write snmp.toml   ┌──────────────────┐
│  WebUI /    │ ─────────────────► │  snmp.toml       │
│  REST API   │  /api/snmp         │  enabled, port,  │
│  (onvif-    │                    │  community,      │
│   rust)     │                    │  sysName/...     │
└─────────────┘                    └────────┬─────────┘
                                            │ SIGHUP reload
┌─────────────┐  supervise         ┌────────▼─────────┐
│  anyka-init │ ─────────────────► │  snmp-agent.bin  │
│  [services. │  restart on death  │  UDP :161        │
│   snmp]     │                    │  SNMPv2c RO      │
└─────────────┘                    │  MIB-II system + │
                                   │  interfaces      │
                                   └────────┬─────────┘
                                            │ read
                                   ┌────────▼─────────┐
                                   │ /proc, sysfs,    │
                                   │ hostname/config  │
                                   └──────────────────┘
```

### Ownership

| Component | Owns |
| --- | --- |
| `snmp-agent` | UDP/161 bind; GET/GETNEXT; reject SET; `/proc`/`sysfs` reads |
| `anyka-init` | Start/stop/restart of the binary; no SNMP parsing |
| `onvif-rust` | Persist settings; ONVIF protocol list + auth; signal reload |
| WebUI | Enable / port / RO community (+ contact/location if not elsewhere) |
| `snmp.toml` | Single machine-written source of truth (overlay style, like `network.toml`) |

Operator baseline `anyka.toml` may list `[services.snmp]` exec paths; runtime
enable/community/port live in `snmp.toml`, not hand-mixed into operator wifi
keys.

## Components

### `snmp-agent` (new workspace binary)

- UDP loop on configured port (default 161).
- SNMPv2c community check → GET/GETNEXT → response; SET → `notWritable`.
- Fixed OID map (no MIB compiler):
  - **system**: `sysDescr`, `sysObjectID`, `sysUpTime`, `sysContact`, `sysName`,
    `sysLocation`, `sysServices`
  - **interfaces**: `ifNumber` + `ifTable` basics (`ifIndex`, `ifDescr`, `ifType`,
    `ifMtu`, `ifSpeed`, `ifPhysAddress`, `ifAdminStatus`, `ifOperStatus`,
    `ifInOctets`, `ifOutOctets`) — trim further only if flash forces it
- Values from `/proc/uptime`, `/proc/net/dev`, sysfs for MAC/oper state; identity
  strings from `snmp.toml` (`sys_name` falls back to hostname).
- `sysDescr` / `sysObjectID`: build-time or device constants (not user-editable
  in v1).
- Hot reload via `SIGHUP` (re-read `snmp.toml`). Port change rebinds; on bind
  failure keep previous socket and surface failure to the writer (ONVIF fault).

**Lifecycle (v1):** when `[services.snmp]` is enabled in init config, the process
stays running. `snmp.toml.enabled=false` unbinds / ignores packets without
exiting so WebUI/ONVIF toggles only need SIGHUP. Operators who want no binary at
all disable the init service entry.

**v3 later:** new auth/PDU module in the same binary; v1 community path stays
isolated so USM is not bolted onto the GET handler.

### `snmp.toml` schema

```toml
enabled = true
port = 161
community = "public"
sys_contact = ""
sys_name = ""      # fallback: hostname
sys_location = ""
```

Missing file → built-in defaults above.

### `anyka-init`

Add `[services.snmp]` using the existing `ServiceCfg` map (exec under SD
payload). Init does not interpret SNMP PDUs.

### `onvif-rust`

- Extend `GetNetworkProtocols` / `SetNetworkProtocols` with SNMP name, port,
  enabled.
- `SetNetworkProtocols` remains Administrator-only; persist atomically to
  `snmp.toml`; `SIGHUP` the agent (pidfile or init-known PID).
- Prefer reusing Identification/hostname for `sysName` where that pane already
  owns the value — avoid duplicate editors.

### WebUI

- Network / ports: SNMP enable, port, RO community.
- Show default `public` with a short security note.
- Optional contact/location only if not already on Identification.

## Data flow

**Boot:** init brings up network → starts supervised services → agent loads
`snmp.toml` (or defaults) → bind if enabled.

**Config change:** ONVIF/WebUI → validate → atomic write `snmp.toml` → SIGHUP →
reload community/port/enabled/sys* → rebind if port changed.

**GET:** parse SNMPv2c → community match → OID lookup / GETNEXT in fixed order →
cheap `/proc` snapshot (optional ~1s ifTable cache).

**SET:** always `notWritable`.

## Errors & security

| Case | Behavior |
| --- | --- |
| Bad PDU / unsupported version | Drop |
| Wrong community | Silent drop (no scanner oracle) |
| SET | `notWritable` |
| Unknown OID | `noSuchObject` / `noSuchInstance` |
| `/proc` failure | Skip iface or `genErr` for that varbind; never panic |
| Bind failure at start | Log + backoff retry (avoid init restart storms) |
| Corrupt `snmp.toml` | Keep last-good in memory; log (quarantine if we mirror network-overlay policy) |

Security honesty: SNMPv2c + default `public` is weak; document in WebUI/wiki.
RO only; redact community in logs; bind all interfaces in v1.

## Testing

- **Unit:** BER/PDU encode-decode; community check; OID walk order; ifTable from
  fixture `/proc/net/dev`.
- **Agent integration (host):** ephemeral port + walk harness (CLI or tiny Rust
  client in CI).
- **onvif-rust:** Get/SetNetworkProtocols ↔ `snmp.toml`; auth levels.
- **WebUI:** validation + mocked save path.
- **Init:** service stanza parse / non-empty exec.
- **Device:** laptop `snmpwalk` of `system` + `interfaces` after SD deploy.

## Approach rejected (record)

- **In `onvif-rust`:** simpler wiring; SNMP dies with the app.
- **In `anyka-init`:** best availability; SNMP bugs share fate with the
  supervisor — rejected.
- **Full Rust SNMP framework / stock snmpd:** heavier or unavailable on rootfs;
  overkill for fixed OID set.

## Success criteria

1. NMS can `snmpwalk` MIB-II `system` and basic `interfaces` with community
   `public` on a deployed device.
2. Disabling via WebUI/ONVIF stops answering without requiring a full device
   reboot (SIGHUP path).
3. SET requests never change device state.
4. Host quality gates green for new agent + onvif-rust + WebUI changes;
   init config tests cover the new service stanza.
