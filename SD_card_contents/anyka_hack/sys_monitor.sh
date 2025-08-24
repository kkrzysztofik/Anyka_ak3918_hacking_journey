#! /bin/sh
# Simple system monitor: logs CPU usage and free memory every minute

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

LOG_DIR=${LOG_DIR:-/mnt/logs}
LOG_FILE=${LOG_FILE:-sys_monitor.log}

echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] sys_monitor starting pid=$$" >> "$LOG_DIR/$LOG_FILE"

cleanup() {
  echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] sys_monitor stopping pid=$$" >> "$LOG_DIR/$LOG_FILE"
  rm -f /tmp/sys_monitor.pid /tmp/cpu1.$$ /tmp/cpu2.$$ 2>/dev/null
  exit 0
}

trap cleanup INT TERM EXIT

echo $$ > /tmp/sys_monitor.pid

while true; do
  # take two samples one second apart to compute recent CPU usage
  head -n1 /proc/stat > /tmp/cpu1.$$ 2>/dev/null
  sleep 1
  head -n1 /proc/stat > /tmp/cpu2.$$ 2>/dev/null

  cpu_pct=$(awk '
    FNR==1{for(i=2;i<=NF;i++) a[i]=$i; next}
    {for(i=2;i<=NF;i++) b[i]=$i;
      idle1=a[5]; idle2=b[5];
      iow1=(a[6] ? a[6] : 0); iow2=(b[6] ? b[6] : 0);
      idle_1=idle1+iow1; idle_2=idle2+iow2;
      non1=a[2]+a[3]+a[4]+(a[7]?a[7]:0)+(a[8]?a[8]:0)+(a[9]?a[9]:0);
      non2=b[2]+b[3]+b[4]+(b[7]?b[7]:0)+(b[8]?b[8]:0)+(b[9]?b[9]:0);
      total1=idle_1+non1; total2=idle_2+non2;
      totald=total2-total1; idled=idle_2-idle_1;
      if (totald>0) usage=(totald-idled)/totald*100; else usage=0;
      printf("%.1f",usage);
    }' /tmp/cpu1.$$ /tmp/cpu2.$$ 2>/dev/null)

  # get free memory in KB (prefer MemAvailable)
  mem_kb=$(awk '/MemAvailable/ {print $2; exit} /MemFree/ {f=$2} END{if(!f) print 0; else print f}' /proc/meminfo 2>/dev/null)
  if [ -z "$mem_kb" ]; then mem_kb=0; fi

  # load average (first field)
  loadavg=$(awk '{print $1}' /proc/loadavg 2>/dev/null || echo 0)

  # process count (count numeric entries in /proc)
  proc_count=$(ls -1 /proc 2>/dev/null | grep -E '^[0-9]+$' | wc -l 2>/dev/null || echo 0)

  # list all writable mounts and their free space in KB (mounted with rw)
  # /proc/mounts fields: device mountpoint fstype options ...
  writable_mounts_list=$(awk '$4 !~ /(^|,)ro(,|$)/ {print $2}' /proc/mounts 2>/dev/null | sort -u)
  mounts_free=""
  if [ -n "$writable_mounts_list" ]; then
    for mp in $writable_mounts_list; do
      # unescape octal escapes (e.g. \040 for space)
      unescaped_mp=$(printf '%b' "$mp")
      free_kb=$(df -k "$unescaped_mp" 2>/dev/null | awk 'NR==2 {print $4}' || echo 0)
      if [ -z "$free_kb" ]; then free_kb=0; fi
      if [ -z "$mounts_free" ]; then
        mounts_free="${unescaped_mp}=${free_kb}"
      else
        mounts_free="${mounts_free},${unescaped_mp}=${free_kb}"
      fi
    done
  fi

  ts="$(date '+%Y-%m-%d %H:%M:%S')"
  echo "$ts [INFO] CPU=${cpu_pct}% FREE_MEM_KB=${mem_kb} LOADAVG=${loadavg} PROC_COUNT=${proc_count} MOUNTS_FREE=${mounts_free}" >> "$LOG_DIR/$LOG_FILE"

  # sleep remaining time to make interval ~60s (we already slept 1s during sampling)
  sleep 59
done
