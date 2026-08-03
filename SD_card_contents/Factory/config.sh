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

if [ ! -x "$BIN" ]; then
  echo "anyka-init: missing or non-executable $BIN" >&2
  exit 1
fi

exec "$BIN"
