#! /bin/sh
# Simple system monitor: logs CPU usage and free memory every minute

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

LOG_DIR=${LOG_DIR:-/mnt/logs}
LOG_FILE=${LOG_FILE:-sys_monitor.log}

# Debug function to help troubleshoot
debug_log() {
  echo "$(date '+%Y-%m-%d %H:%M:%S') [DEBUG] $*" >> "$LOG_DIR/$LOG_FILE"
}

# Ensure preferred log dir exists (attempt creation if missing)
if [ ! -d "$LOG_DIR" ]; then
  mkdir -p "$LOG_DIR" 2>/dev/null || true
fi

# Primary write test; if it fails try one more time after mkdir
if ! touch "$LOG_DIR/$LOG_FILE" 2>/dev/null; then
  # Retry once after brief delay in case /mnt just mounted
  sleep 1
  touch "$LOG_DIR/$LOG_FILE" 2>/dev/null || {
    # Final fallback to /mnt/tmp (created earlier by caller / gergehack) to avoid ram-only /tmp
    LOG_DIR="/mnt/tmp"
    mkdir -p "$LOG_DIR" 2>/dev/null || true
    LOG_FILE="sys_monitor.log"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [WARN] Falling back to $LOG_DIR/$LOG_FILE (original log dir not writable)" >&2
  }
fi

echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] sys_monitor starting pid=$$, LOG_DIR=$LOG_DIR" >> "$LOG_DIR/$LOG_FILE"

# Check essential /proc files
debug_log "Checking /proc filesystem availability..."
for proc_file in /proc/stat /proc/meminfo /proc/loadavg /proc/mounts; do
  if [ -r "$proc_file" ]; then
    debug_log "$proc_file is readable"
  else
    debug_log "ERROR: $proc_file is not readable"
  fi
done

cleanup() {
  echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] sys_monitor stopping pid=$$" >> "$LOG_DIR/$LOG_FILE"
  rm -f /mnt/tmp/sys_monitor.pid /mnt/tmp/cpu1.$$ /mnt/tmp/cpu2.$$ 2>/dev/null
  exit 0
}

trap cleanup INT TERM EXIT

mkdir -p /mnt/tmp 2>/dev/null || true
echo $$ > /mnt/tmp/sys_monitor.pid
debug_log "PID file created: /mnt/tmp/sys_monitor.pid"

# Test basic functionality before entering main loop
debug_log "Testing basic functionality..."

# Test /proc/stat read
if head -n1 /proc/stat > /mnt/tmp/test_cpu.$$ 2>/dev/null; then
  debug_log "/proc/stat read test: OK"
  rm -f /mnt/tmp/test_cpu.$$
else
  debug_log "ERROR: Cannot read /proc/stat"
fi

# Test /proc/meminfo read  
if awk '/MemFree/ {print $2; exit}' /proc/meminfo > /mnt/tmp/test_mem.$$ 2>/dev/null; then
  debug_log "/proc/meminfo read test: OK"
  rm -f /mnt/tmp/test_mem.$$
else
  debug_log "ERROR: Cannot read /proc/meminfo"
fi

debug_log "Entering main monitoring loop..."

while true; do
  debug_log "Starting monitoring cycle..."
  
  # take two samples one second apart to compute recent CPU usage
  if ! head -n1 /proc/stat > /mnt/tmp/cpu1.$$ 2>/dev/null; then
    debug_log "ERROR: Failed to read /proc/stat (first sample)"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [ERROR] Failed to read /proc/stat" >> "$LOG_DIR/$LOG_FILE"
    sleep 60
    continue
  fi
  
  sleep 1
  
  if ! head -n1 /proc/stat > /mnt/tmp/cpu2.$$ 2>/dev/null; then
    debug_log "ERROR: Failed to read /proc/stat (second sample)"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [ERROR] Failed to read /proc/stat" >> "$LOG_DIR/$LOG_FILE"
    rm -f /mnt/tmp/cpu1.$$
    sleep 60
    continue
  fi

  # Simplified CPU calculation (busybox awk friendly)
  # Reads two snapshots of /proc/stat first line and computes non-idle percentage
  cpu_pct=$(awk 'NR==FNR { for(i=2;i<=NF;i++) t1+=$i; i1=$5; next } { for(i=2;i<=NF;i++) t2+=$i; i2=$5 } END { td=t2-t1; id=i2-i1; if (td>0) { printf("%.1f", (td-id)/td*100); } else { print "0.0" } }' /mnt/tmp/cpu1.$$ /mnt/tmp/cpu2.$$ 2>/dev/null)
  
  if [ -z "$cpu_pct" ]; then
    cpu_pct="0.0"
    debug_log "CPU calculation failed, using 0.0"
  fi

  # get free memory in KB (prefer MemAvailable, fallback to MemFree)
  # Memory free (prefer MemAvailable else MemFree) — explicit field match improves busybox reliability
  mem_kb=$(awk '($1=="MemAvailable:"){print $2; found=1; exit} ($1=="MemFree:" && !found){mf=$2} END{ if(!found){ if(mf) print mf; else print 0 } }' /proc/meminfo 2>/dev/null)
  if [ -z "$mem_kb" ]; then 
    mem_kb=0
    debug_log "Memory calculation failed, using 0"
  fi

  # load average (first field)
  loadavg=$(awk '{print $1; exit}' /proc/loadavg 2>/dev/null)
  if [ -z "$loadavg" ]; then 
    loadavg="0.00"
    debug_log "Load average read failed, using 0.00"
  fi

  # process count (count numeric entries in /proc) - simplified for busybox
  proc_count=$(ls -1 /proc 2>/dev/null | grep '^[0-9][0-9]*$' | wc -l 2>/dev/null)
  if [ -z "$proc_count" ]; then 
    proc_count=0
    debug_log "Process count failed, using 0"
  fi

  # Simplified mount space check - now includes /mnt /tmp /data /etc/jffs2 and root
  mounts_free=""
  for mp in /mnt /tmp /data /etc/jffs2 /; do
    if [ -d "$mp" ]; then
      free_kb=$(df -k "$mp" 2>/dev/null | awk 'NR==2 {print $4; exit}')
      if [ -n "$free_kb" ] && [ "$free_kb" -gt 0 ] 2>/dev/null; then
        if [ -z "$mounts_free" ]; then
          mounts_free="${mp}=${free_kb}"
        else
          mounts_free="${mounts_free},${mp}=${free_kb}"
        fi
      fi
    fi
  done
  
  if [ -z "$mounts_free" ]; then
    mounts_free="none=0"
    debug_log "Mount space check failed"
  fi

  ts="$(date '+%Y-%m-%d %H:%M:%S')"
  # Individual explicit free space metrics for key persistent areas (0 if missing)
  data_free_kb=$(echo "$mounts_free" | tr ',' '\n' | awk -F'=| ' '/\/data=/{print $2; found=1; exit} END{if(!found) print 0}')
  jffs2_free_kb=$(echo "$mounts_free" | tr ',' '\n' | awk -F'=| ' '/\/etc\/jffs2=/{print $2; found=1; exit} END{if(!found) print 0}')

  log_line="$ts [INFO] CPU=${cpu_pct}% FREE_MEM_KB=${mem_kb} LOADAVG=${loadavg} PROC_COUNT=${proc_count} DATA_FREE_KB=${data_free_kb} JFFS2_FREE_KB=${jffs2_free_kb} MOUNTS_FREE=${mounts_free}"
  
  echo "$log_line" >> "$LOG_DIR/$LOG_FILE"
  debug_log "Logged metrics successfully"
  
  # Clean up temp files
  rm -f /mnt/tmp/cpu1.$$ /mnt/tmp/cpu2.$$ 2>/dev/null

  # sleep remaining time to make interval ~60s (we already slept 1s during sampling)
  debug_log "Sleeping for 59 seconds..."
  sleep 59
done
