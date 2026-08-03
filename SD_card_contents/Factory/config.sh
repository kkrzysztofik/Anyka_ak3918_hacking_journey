#!/bin/sh
# P0: the only phase that must never fail.
#
# Called by the vendor's /usr/sbin/service.sh:91 when /mnt/Factory exists.
# Deliberately shell, not the ELF: anyka-init.bin needs the bundled uClibc
# loader at /mnt/anyka_hack/lib/ld-uClibc.so.1, and if that is missing the
# kernel cannot even start it. Shell has no such dependency, so telnet comes up
# regardless.
#
# service.sh:85 runs `killall telnetd` immediately before calling us, killing
# the telnetd that rcS:8 started. Restarting it here restores the only remote
# recovery channel.

telnetd -p 24 -l /bin/sh 2>/dev/null &

# The two paths below are overridable only so the host test in
# tests/p0_wrapper.rs can stub them. Production never sets these.
BIN=${ANYKA_INIT_BIN:-/mnt/anyka_hack/anyka-init.bin}
WIFI_MANAGE=${ANYKA_WIFI_MANAGE:-/usr/sbin/wifi_manage.sh}

# Deadman. P1 (main.rs:16) parks forever on a config error, and wifi bring-up
# is P2 (main.rs:62) — so a bad anyka.toml leaves telnet listening on an
# interface with no link. .121 has no serial console and no non-jumphost
# access, which would make that an on-site SD pull. Three minutes in, if
# nothing holds an address, hand wifi back to the vendor chain.
#
# Costs nothing when the supervisor is healthy: the grep matches and the
# subshell exits. Also covers a failed bring-up whose internal
# fallback_to_vendor did not take.
#
# Armed BEFORE the -x guard on purpose. A missing or corrupt binary exits
# non-zero, and service.sh's FACTORY_TEST branch then returns without ever
# starting wifi — the same stranding this exists to prevent.
( sleep 180
  ifconfig wlan0 | grep -q "inet addr" || "$WIFI_MANAGE" start ) &

if [ ! -x "$BIN" ]; then
  echo "anyka-init: missing or non-executable $BIN" >&2
  exit 1
fi

exec "$BIN"
