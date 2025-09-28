#!/bin/sh

LOG_FILE=factory_config.log
TELNET_PID_FILE="/mnt/tmp/telnetd.pid"

# Initialize log directories first
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities (which also ensures log directory)
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

log INFO "Starting factory config initialization"

# Create temporary directory for PID files
mkdir -p /mnt/tmp 2>/dev/null || true

# Start telnet (optional) on non-standard port if not already running
if ! pgrep -f 'telnetd.*-p 24' >/dev/null 2>&1; then
  telnetd -p 24 -l /bin/sh &
  echo $! > "$TELNET_PID_FILE" 2>/dev/null
  log INFO "Started telnetd on port 24 (pid=$!)"
else
  log DEBUG "telnetd already running"
fi

if [ ! -f /data/gergehack.sh ]; then
  cp /mnt/anyka_hack/gergehack.sh /data/gergehack.sh 2>/dev/null && log INFO "Installed gergehack.sh" || log WARN "Failed to copy gergehack.sh"
fi

if [ ! -f /data/gergesettings.txt ]; then
  cp /mnt/anyka_hack/gergesettings.txt /data/gergesettings.txt 2>/dev/null && log INFO "Installed gergesettings.txt" || log WARN "Failed to copy gergesettings.txt"
fi

log INFO "Launching gergehack"
/data/gergehack.sh >> /mnt/logs/gergehack.log 2>&1 &
GERGEHACK_PID=$!
echo $GERGEHACK_PID > /mnt/tmp/gergehack.pid 2>/dev/null
log INFO "Factory config complete (gergehack pid=$GERGEHACK_PID)"
