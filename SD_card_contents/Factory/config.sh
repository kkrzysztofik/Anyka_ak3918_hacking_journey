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

# The paths below are overridable only so the host test in
# tests/p0_wrapper.rs can stub them. Production never sets these.
#
# A/B slot selection. /mnt is vfat and has no symlinks, so the pointer is a
# one-byte file. Unreadable or unrecognized reads as slot a.
#
# Second line is the fallback: a slot whose supervisor will not exec is the
# exact failure an update must survive, and it costs one test to handle here
# rather than waiting 240s for the deadman below to restore the vendor path.
SLOT_ROOT=${ANYKA_SLOT_ROOT:-/mnt/anyka_hack}
SLOT=$(cat "$SLOT_ROOT/active" 2>/dev/null) || SLOT=a
[ "$SLOT" = b ] || SLOT=a
# An explicit ANYKA_INIT_BIN (host tests) must not fall through to the other
# slot — that would rewrite a deliberate missing-path probe into the
# production /mnt/anyka_hack layout.
if [ -n "${ANYKA_INIT_BIN:-}" ]; then
  BIN=$ANYKA_INIT_BIN
else
  BIN=$SLOT_ROOT/slots/$SLOT/anyka-init.bin
  if [ ! -x "$BIN" ]; then
    OTHER=$([ "$SLOT" = a ] && echo b || echo a)
    BIN=$SLOT_ROOT/slots/$OTHER/anyka-init.bin
  fi
fi
WIFI_MANAGE=${ANYKA_WIFI_MANAGE:-/usr/sbin/wifi_manage.sh}
SELF=${ANYKA_CONFIG_SELF:-/mnt/Factory/config.sh}
BAK=${ANYKA_CONFIG_BAK:-/mnt/Factory/config.sh.gerge.bak}

# Deadman. P1 (main.rs:16) parks forever on a config error, and wifi bring-up
# is P2 (main.rs:62) — so a bad anyka.toml leaves telnet listening on an
# interface with no link. .121 has no serial console and no non-jumphost
# access, which would make that an on-site SD pull.
#
# Costs nothing when the supervisor is healthy: the first grep matches and the
# subshell exits before doing anything.
#
# Two stages, because the 2026-08-03 dry run proved one is not enough. A failed
# bring-up can leave the radio wedged — module loaded, netdev destroyed, chip ID
# reading back as 0x57F7FFFF — and in that state `wifi_manage.sh start` fails
# exactly the way anyka-init's own fallback_to_vendor did. Only a reboot clears
# it. So: try the vendor chain, and if the link is still down a minute later,
# put the vendor boot path back and reboot into it. The camera returns on
# gergehack.sh in ~5 minutes instead of waiting for someone to drive out.
#
# Restoring is safe to do blind: config.sh.gerge.bak is written before the
# cutover and never changes afterwards.
#
# Armed BEFORE the -x guard on purpose. A missing or corrupt binary exits
# non-zero, and service.sh's FACTORY_TEST branch then returns without ever
# starting wifi — the same stranding this exists to prevent.
( sleep 180
  ifconfig wlan0 | grep -q "inet addr" && exit 0
  "$WIFI_MANAGE" start
  sleep 60
  ifconfig wlan0 | grep -q "inet addr" && exit 0
  # Restore atomically (temp + rename) and reboot only when every step
  # succeeds: a failed copy must not reboot into the unchanged broken wrapper.
  RESTORE="${SELF}.restore.$$"
  if [ -r "$BAK" ] && cp "$BAK" "$RESTORE" && sync && mv "$RESTORE" "$SELF" && sync; then
    reboot
  else
    echo "anyka-init: vendor boot-path restore failed" >&2
  fi ) &

if [ ! -x "$BIN" ]; then
  echo "anyka-init: missing or non-executable $BIN" >&2
  exit 1
fi

# service.sh has no respawn. If the supervisor dies, every service is
# orphaned and only a power cycle recovers. anyka-init's own reboot path
# uses libc::reboot and does not return, so this loop cannot fight it.
while :; do
  "$BIN"
  sleep 5
done &

