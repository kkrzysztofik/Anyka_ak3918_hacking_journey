#!/bin/sh

# verify_logs.sh - Verify that the logging directory structure is set up correctly
# Usage: ./verify_logs.sh

echo "=== Anyka Hack Log Directory Verification ==="
echo "Date: $(date)"
echo

# Test log directory initialization
echo "Testing log directory initialization..."
probe="/mnt/logs/.verify_write_probe.$$"
if ! mkdir -p /mnt/logs || ! echo test > "$probe" || ! rm "$probe"; then
  echo "✗ Failed to create or access /mnt/logs"
  exit 1
fi

# Check main log directory
if [ -d "/mnt/logs" ]; then
  echo "✓ Main log directory exists: /mnt/logs"
  if [ -w "/mnt/logs" ]; then
    echo "✓ Main log directory is writable"
    # Test write access
    if echo "test" > /mnt/logs/verify_test.log 2>/dev/null; then
      echo "✓ Write test successful"
      rm /mnt/logs/verify_test.log 2>/dev/null
    else
      echo "✗ Write test failed"
    fi
  else
    echo "✗ Main log directory is not writable"
  fi
else
  echo "✗ Main log directory does not exist: /mnt/logs"
fi

# Check fallback directories
echo
echo "Checking fallback directories..."
for dir in "/mnt/tmp/logs" "/mnt/tmp"; do
  if [ -d "$dir" ]; then
    echo "✓ Fallback directory exists: $dir"
    [ -w "$dir" ] && echo "✓ Fallback directory is writable: $dir" || echo "✗ Fallback directory not writable: $dir"
  else
    echo "- Fallback directory does not exist: $dir"
  fi
done

# List existing log files
echo
echo "Existing log files in /mnt/logs:"
if [ -d "/mnt/logs" ]; then
  ls -la /mnt/logs/ 2>/dev/null | grep "\.log$" || echo "No .log files found"
else
  echo "Log directory not accessible"
fi

# Test inline log helper (replaces the old common.sh dependency)
echo
echo "Testing inline log helper..."
log() {
  level="$1"
  shift
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] [${level}] $*"
}
log INFO "Verification test message"

echo
echo "=== Verification Complete ==="
