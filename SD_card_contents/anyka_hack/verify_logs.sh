#! /bin/sh

# verify_logs.sh - Verify that the logging directory structure is set up correctly
# Usage: ./verify_logs.sh

echo "=== Anyka Hack Log Directory Verification ==="
echo "Date: $(date)"
echo

# Test log directory initialization
echo "Testing log directory initialization..."
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

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
for dir in "/tmp/logs" "/tmp"; do
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

# Test common.sh logging function
echo
echo "Testing common.sh logging function..."
if [ -f /mnt/anyka_hack/common.sh ]; then
  LOG_FILE=verify_test.log
  . /mnt/anyka_hack/common.sh
  log INFO "Verification test message"
  if [ -f "/mnt/logs/verify_test.log" ]; then
    echo "✓ Common.sh logging function works"
    echo "Last log entry:"
    tail -n 1 /mnt/logs/verify_test.log
    rm /mnt/logs/verify_test.log 2>/dev/null
  else
    echo "✗ Common.sh logging function failed"
  fi
else
  echo "✗ common.sh not found"
fi

echo
echo "=== Verification Complete ==="
