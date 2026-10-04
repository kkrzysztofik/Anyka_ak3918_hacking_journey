# SNMP agent binary directory

`snmp-agent.bin` is installed here by `scripts/build_payload.sh`.
Runtime config lives at `/mnt/anyka_hack/snmp.toml` (sibling of this dir).

On/off is `[services.snmp] enabled` in `/mnt/anyka_hack/anyka.toml` (WebUI:
Diagnostics → Processes). `snmp.toml` holds only port, community and the
sys* strings; a leftover `enabled` key is ignored.
