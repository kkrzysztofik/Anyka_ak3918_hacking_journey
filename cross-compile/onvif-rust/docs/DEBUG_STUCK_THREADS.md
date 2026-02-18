# Debugging Stuck Threads on Embedded Device

This guide provides practical commands to identify which thread is stuck during shutdown or runtime on the Anyka AK3918 embedded Linux system.

## Quick Check: Find Stuck Threads

### Method 1: Check Thread States (Recommended)

```bash
# Get process ID of onvif-rust
ONVIF_PID=$(pgrep -f onvif-rust || pidof onvif-rust)

# Check all thread states - look for 'D' (uninterruptible sleep)
echo "=== Thread States ==="
for tid in /proc/$ONVIF_PID/task/*; do
    tid_num=$(basename $tid)
    name=$(cat $tid/comm 2>/dev/null || echo "unknown")
    state=$(awk '{print $3}' $tid/stat 2>/dev/null || echo "unknown")
    wchan=$(cat $tid/wchan 2>/dev/null || echo "unknown")
    
    # Highlight stuck threads (D state = uninterruptible sleep)
    if [ "$state" = "D" ]; then
        echo "⚠️  STUCK: TID=$tid_num, Name=$name, State=$state, Wait=$wchan"
    else
        echo "   OK:    TID=$tid_num, Name=$name, State=$state"
    fi
done
```

**Expected Output:**

```bash
   OK:    TID=1234, Name=onvif-rust, State=S
   OK:    TID=1235, Name=main-read, State=S
⚠️  STUCK: TID=1236, Name=venc_capture, State=D, Wait=__down_interruptible
   OK:    TID=1237, Name=venc_encode, State=S
```

### Method 2: One-Liner for Stuck Threads Only

```bash
ONVIF_PID=$(pgrep -f onvif-rust || pidof onvif-rust)
echo "=== Stuck Threads (D state) ==="
for tid in /proc/$ONVIF_PID/task/*; do
    state=$(awk '{print $3}' $tid/stat 2>/dev/null)
    if [ "$state" = "D" ]; then
        tid_num=$(basename $tid)
        name=$(cat $tid/comm 2>/dev/null)
        wchan=$(cat $tid/wchan 2>/dev/null)
        echo "TID: $tid_num | Name: $name | Wait: $wchan"
        echo "Stack trace:"
        cat $tid/stack 2>/dev/null | head -10
        echo "---"
    fi
done
```

## Detailed Thread Analysis

### Complete Thread Report

```bash
#!/bin/sh
# save as: check_threads.sh

ONVIF_PID=$(pgrep -f onvif-rust || pidof onvif-rust)
if [ -z "$ONVIF_PID" ]; then
    echo "Error: onvif-rust process not found"
    exit 1
fi

echo "=== Thread Analysis for PID $ONVIF_PID ==="
echo ""

for tid in /proc/$ONVIF_PID/task/*; do
    tid_num=$(basename $tid)
    name=$(cat $tid/comm 2>/dev/null || echo "unknown")
    state=$(awk '{print $3}' $tid/stat 2>/dev/null || echo "unknown")
    utime=$(awk '{print $14}' $tid/stat 2>/dev/null || echo "0")
    stime=$(awk '{print $15}' $tid/stat 2>/dev/null || echo "0")
    wchan=$(cat $tid/wchan 2>/dev/null || echo "unknown")
    
    echo "TID: $tid_num"
    echo "  Name: $name"
    echo "  State: $state"
    echo "  CPU Time: user=${utime}ms, sys=${stime}ms"
    echo "  Wait Channel: $wchan"
    
    if [ "$state" = "D" ]; then
        echo "  ⚠️  STATUS: UNINTERRUPTIBLE SLEEP (STUCK)"
        echo "  Stack trace:"
        cat $tid/stack 2>/dev/null | head -15 | sed 's/^/    /'
    elif [ "$state" = "S" ]; then
        echo "  ✓ STATUS: Interruptible sleep (normal)"
    elif [ "$state" = "R" ]; then
        echo "  ✓ STATUS: Running"
    fi
    echo ""
done
```

### Check Specific Thread by Name

```bash
# Find capture_thread (tid 731 from logs)
ONVIF_PID=$(pgrep -f onvif-rust || pidof onvif-rust)
for tid in /proc/$ONVIF_PID/task/*; do
    name=$(cat $tid/comm 2>/dev/null)
    if echo "$name" | grep -q "venc_capture\|capture"; then
        tid_num=$(basename $tid)
        state=$(awk '{print $3}' $tid/stat 2>/dev/null)
        wchan=$(cat $tid/wchan 2>/dev/null)
        echo "Found capture thread: TID=$tid_num, State=$state, Wait=$wchan"
        echo "Stack:"
        cat $tid/stack 2>/dev/null
    fi
done
```

## Understanding Thread States

| State | Meaning | Action |
| ----- | ------ | ----- |
| **R** | Running | Normal - thread is executing |
| **S** | Interruptible sleep | Normal - waiting for I/O or signal |
| **D** | **Uninterruptible sleep** | **⚠️ STUCK** - blocked in kernel I/O, cannot be killed |
| **Z** | Zombie | Dead thread waiting for parent to reap |
| **T** | Stopped | Suspended by signal (SIGSTOP) |

## Common Wait Channels (wchan) for Stuck Threads

| Wait Channel | Meaning | Likely Cause |
| ------------ | ------- | ------------ |
| `__down_interruptible` | Waiting on semaphore/mutex | Mutex contention, deadlock |
| `do_sys_poll` | Waiting in poll/select | Blocking I/O operation |
| `futex_wait_queue_me` | Waiting on futex | Mutex/semaphore wait |
| `schedule` | General sleep | Normal sleep, not stuck |
| `io_schedule` | I/O wait | Blocked on device I/O |
| `__mutex_lock` | Mutex lock wait | Mutex contention |

## Expected Threads During Normal Operation

After initialization, you should see:

```text
TID: <main>     Name: onvif-rust          State: S (main thread)
TID: <tid1>     Name: main-read           State: S (Rust reader thread)
TID: <tid2>     Name: sub-read            State: S (Rust reader thread)
TID: <tid3>     Name: venc_capture        State: S (SDK capture thread, tid 731)
TID: <tid4>     Name: venc_encode         State: S (SDK encode thread, tid 732)
TID: <tid5>     Name: change_fps_pthread  State: S (SDK FPS thread, tid 729)
```

## During Shutdown - What to Look For

### Normal Shutdown (All threads exit)

```text
All threads should transition: S → Z (zombie) → disappear
```

### Stuck Shutdown (Thread blocked)

```text
⚠️  STUCK: TID=731, Name=venc_capture, State=D, Wait=__down_interruptible
```

This indicates `capture_thread` is stuck waiting on a mutex/semaphore, likely in `ak_vi_get_frame()` holding `pdev->frame_lock`.

## Real-Time Monitoring

### Watch Thread States Over Time

```bash
#!/bin/sh
# Monitor thread states every second
ONVIF_PID=$(pgrep -f onvif-rust || pidof onvif-rust)

while true; do
    clear
    echo "=== $(date) ==="
    echo "Thread States for PID $ONVIF_PID:"
    for tid in /proc/$ONVIF_PID/task/*; do
        tid_num=$(basename $tid)
        name=$(cat $tid/comm 2>/dev/null || echo "unknown")
        state=$(awk '{print $3}' $tid/stat 2>/dev/null || echo "unknown")
        
        if [ "$state" = "D" ]; then
            echo "⚠️  $tid_num: $name - STUCK (D)"
        else
            echo "   $tid_num: $name - $state"
        fi
    done
    sleep 1
done
```

### Monitor During Shutdown

```bash
# Run this in one terminal, then trigger shutdown in another
ONVIF_PID=$(pgrep -f onvif-rust || pidof onvif-rust)
echo "Monitoring shutdown for PID $ONVIF_PID..."
echo "Press Ctrl+C to stop"
echo ""

while [ -d "/proc/$ONVIF_PID" ]; do
    echo "=== $(date +%H:%M:%S) ==="
    stuck_count=0
    for tid in /proc/$ONVIF_PID/task/*; do
        state=$(awk '{print $3}' $tid/stat 2>/dev/null)
        if [ "$state" = "D" ]; then
            tid_num=$(basename $tid)
            name=$(cat $tid/comm 2>/dev/null)
            wchan=$(cat $tid/wchan 2>/dev/null)
            echo "⚠️  STUCK: TID=$tid_num ($name) waiting on $wchan"
            stuck_count=$((stuck_count + 1))
        fi
    done
    if [ $stuck_count -eq 0 ]; then
        echo "✓ No stuck threads"
    fi
    echo "Total threads: $(ls -1 /proc/$ONVIF_PID/task/ | wc -l)"
    echo ""
    sleep 0.5
done
echo "Process exited"
```

## Stack Trace Analysis

### Get Full Stack Trace for Stuck Thread

```bash
ONVIF_PID=$(pgrep -f onvif-rust || pidof onvif-rust)

# Find stuck thread
for tid in /proc/$ONVIF_PID/task/*; do
    state=$(awk '{print $3}' $tid/stat 2>/dev/null)
    if [ "$state" = "D" ]; then
        tid_num=$(basename $tid)
        name=$(cat $tid/comm 2>/dev/null)
        echo "=== Stack Trace for Stuck Thread: TID=$tid_num ($name) ==="
        cat $tid/stack 2>/dev/null
        echo ""
        echo "=== Registers ==="
        cat $tid/syscall 2>/dev/null || echo "Syscall info not available"
    fi
done
```

### Using GDB (if available on device)

```bash
# Attach to process and get all thread stacks
ONVIF_PID=$(pgrep -f onvif-rust || pidof onvif-rust)
gdb -p $ONVIF_PID -batch -ex "thread apply all bt" -ex "quit" 2>/dev/null
```

## Identifying the Specific Stuck Thread from Your Logs

Based on your logs, the stuck thread is likely:

1. **capture_thread (tid 731)** - Stuck in `ak_vi_get_frame()` with backpressure
2. **main-read thread** - Stuck waiting for `ak_venc_cancel_stream()` to complete

### Check Script for Your Specific Case

```bash
#!/bin/sh
# check_stuck_shutdown.sh - Check threads during shutdown

ONVIF_PID=$(pgrep -f onvif-rust || pidof onvif-rust)

echo "=== Checking for stuck threads ==="
echo ""

# Check for capture_thread (SDK thread)
echo "1. SDK capture_thread (should be tid 731):"
for tid in /proc/$ONVIF_PID/task/*; do
    name=$(cat $tid/comm 2>/dev/null)
    if echo "$name" | grep -q "venc_capture\|capture"; then
        tid_num=$(basename $tid)
        state=$(awk '{print $3}' $tid/stat 2>/dev/null)
        wchan=$(cat $tid/wchan 2>/dev/null)
        echo "   Found: TID=$tid_num, State=$state, Wait=$wchan"
        if [ "$state" = "D" ]; then
            echo "   ⚠️  STUCK in kernel I/O"
            echo "   Stack:"
            cat $tid/stack 2>/dev/null | head -10 | sed 's/^/      /'
        fi
    fi
done

echo ""
echo "2. Rust reader threads:"
for tid in /proc/$ONVIF_PID/task/*; do
    name=$(cat $tid/comm 2>/dev/null)
    if echo "$name" | grep -q "main-read\|sub-read"; then
        tid_num=$(basename $tid)
        state=$(awk '{print $3}' $tid/stat 2>/dev/null)
        wchan=$(cat $tid/wchan 2>/dev/null)
        echo "   Found: TID=$tid_num ($name), State=$state, Wait=$wchan"
        if [ "$state" = "D" ]; then
            echo "   ⚠️  STUCK"
            cat $tid/stack 2>/dev/null | head -10 | sed 's/^/      /'
        fi
    fi
done

echo ""
echo "3. All threads in D state (uninterruptible sleep):"
stuck_found=0
for tid in /proc/$ONVIF_PID/task/*; do
    state=$(awk '{print $3}' $tid/stat 2>/dev/null)
    if [ "$state" = "D" ]; then
        stuck_found=1
        tid_num=$(basename $tid)
        name=$(cat $tid/comm 2>/dev/null)
        wchan=$(cat $tid/wchan 2>/dev/null)
        echo "   ⚠️  TID=$tid_num ($name) waiting on $wchan"
    fi
done

if [ $stuck_found -eq 0 ]; then
    echo "   ✓ No stuck threads found"
fi
```

## What to Do When You Find a Stuck Thread

### If capture_thread (tid 731) is stuck

**Symptom**: State=D, Wait=`__down_interruptible` or `do_sys_poll`

**Cause**: Thread is blocked in `ak_vi_get_frame()` holding `pdev->frame_lock` due to frame queue backpressure

**Solution**:

1. Call `ak_vi_clear_buffer()` BEFORE `ak_venc_cancel_stream()`
2. This unblocks the capture thread so it can exit cooperatively

### If main-read/sub-read threads are stuck

**Symptom**: State=D or State=S, waiting for `ak_venc_get_stream()` to return

**Cause**: `ak_venc_cancel_stream()` is blocking, preventing reader threads from exiting

**Solution**:

1. Ensure `ak_vi_clear_buffer()` is called first
2. Reader threads check `stop_signal` and should exit after cancel completes

## Quick Diagnostic Command

```bash
# One command to rule them all
ONVIF_PID=$(pgrep -f onvif-rust || pidof onvif-rust) && \
echo "=== Thread Status ===" && \
for tid in /proc/$ONVIF_PID/task/*; do \
    tid_num=$(basename $tid); \
    name=$(cat $tid/comm 2>/dev/null); \
    state=$(awk '{print $3}' $tid/stat 2>/dev/null); \
    wchan=$(cat $tid/wchan 2>/dev/null); \
    [ "$state" = "D" ] && echo "⚠️  STUCK: $tid_num ($name) -> $wchan" || \
    echo "   OK:    $tid_num ($name) -> $state"; \
done
```

## References

- Thread state codes: `/proc/<pid>/task/<tid>/stat` field 3
- Wait channel: `/proc/<pid>/task/<tid>/wchan`
- Stack trace: `/proc/<pid>/task/<tid>/stack` (if kernel supports it)
- Thread name: `/proc/<pid>/task/<tid>/comm`
