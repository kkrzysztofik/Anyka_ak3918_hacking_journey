---
name: protocol-debugging
description: |
  Protocol debugging specialist for ONVIF, RTSP, and network issues. Covers Wireshark analysis, packet inspection, and protocol troubleshooting.
  Triggers on: "Wireshark", "ONVIF debug", "RTSP issue", "packet analysis", "protocol error", "network capture".
version: 1.0.0
---

# Protocol Debugging and Analysis

Diagnose ONVIF, RTSP, and streaming issues using network analysis tools. Capture, filter, and analyze network traffic to identify protocol problems.

## Wireshark Capture Setup

### Capture ONVIF Traffic

```bash
# Capture only ONVIF SOAP over HTTP traffic
sudo tcpdump -i eth0 -w onvif_capture.pcap \
  "port 8080 and tcp" \
  "host 192.168.1.100"

# More targeted capture
sudo tcpdump -i eth0 -w onvif_debug.pcap \
  "tcp port 8080 or tcp port 80 or tcp port 443" \
  "and host 192.168.1.100"

# Capture RTSP streaming
sudo tcpdump -i eth0 -w rtsp_stream.pcap \
  "tcp port 554 or udp portrange 5000-65535" \
  "host 192.168.1.100"

# Capture everything to/from camera for full analysis
sudo tcpdump -i eth0 -w camera_full.pcap \
  "host 192.168.1.100"
```

### Transfer to Analysis Machine

```bash
# From camera (over SSH)
scp root@192.168.1.100:/tmp/onvif_capture.pcap ./

# On your machine
wireshark onvif_capture.pcap
```

## Wireshark Filters

### ONVIF-Specific Filters

```
# All SOAP requests/responses
http.request.method == "POST" and http.content_type contains "xml"

# Specific ONVIF action
tcp.dstport == 8080 and frame contains "GetDeviceInformation"

# GetCapabilities requests
frame contains "GetCapabilities"

# Authentication attempts
tcp.dstport == 8080 and frame contains "UsernameToken"

# SetHostname operations
frame contains "SetHostname"

# All faults/errors
frame contains "Fault" or frame contains "fault"

# Status code 401 (Unauthorized)
http.response.code == 401

# Status code 500 (Server error)
http.response.code == 500
```

### Streaming Protocol Filters

```
# RTSP traffic
tcp.port == 554

# RTP video packets
rtp.p_type == 96

# H.264 RTP
rtp.p_type == 96 and rtp.ssrc

# RTCP reports
rtcp

# Media playback
tcp.dstport == 554 or udp.dstport == 5004 or udp.dstport == 5005

# Fragmentation units (FU)
frame contains "0x1c"

# SPS/PPS sequences (NAL types 7/8)
frame contains "0x67" or frame contains "0x68"
```

### General Network Filters

```
# Retransmissions (indicates packet loss)
tcp.analysis.retransmission

# Duplicate ACKs
tcp.analysis.duplicate_ack

# Out-of-order packets
tcp.analysis.out_of_order

# Timeouts
tcp.analysis.ack_lost_segment

# Large packets
frame.len > 1500

# Slow requests (>1 second)
http.time > 1000
```

## Common ONVIF Issues

### Issue: "Unauthorized" Error (HTTP 401)

**Symptoms:**
- ONVIF requests return 401 Unauthorized
- Authentication headers missing or malformed

**Diagnosis:**

```bash
# Filter to see authentication attempts
http.request.method == "POST" and tcp.dstport == 8080
```

**Solutions:**

1. Verify credentials are correct
2. Check WS-Security header construction:
   ```bash
   # Look for these headers in the capture
   frame contains "UsernameToken"
   frame contains "PasswordDigest"
   ```

3. Check timestamp validity:
   ```bash
   # Timestamp should be current time ± small skew
   frame contains "Created"
   ```

### Issue: "Action Not Supported" (SOAP Fault)

**Symptoms:**
- Specific ONVIF operations fail
- Other operations work fine

**Diagnosis:**

```bash
# Find the fault
frame contains "ActionNotSupported"

# Check what action was attempted
tcp.stream == 0  # First connection
```

**Solutions:**

1. Verify the camera supports the operation
2. Check GetCapabilities for supported services
3. Verify correct service endpoint (device, media, ptz, etc.)

### Issue: Slow Response Times

**Symptoms:**
- WebUI feels sluggish
- API calls take >1-2 seconds

**Diagnosis:**

```bash
# Measure response times
http && http.response.code == 200

# Find slow requests
http && http.time > 1000  # Over 1 second
```

**Solutions:**

1. Check for packet loss:
   ```bash
   tcp.analysis.retransmission
   ```

2. Check for timeouts:
   ```bash
   tcp.analysis.ack_lost_segment
   ```

3. Monitor bandwidth usage

## RTSP/RTP Issues

### Issue: Video Stream Won't Start

**Symptoms:**
- RTSP connection established but no data flows
- "Media not available" or similar error

**Diagnosis:**

```bash
# RTSP handshake
tcp.port == 554

# RTP packets present?
rtp

# RTCP feedback
rtcp
```

**Analyze RTP sequence:**

```bash
# Get sequence numbers
rtp.seq

# Check for gaps in sequence
# Large jumps = dropped packets
```

### Issue: Choppy or Stuttering Video

**Symptoms:**
- Video stutters or freezes
- Audio drops frequently
- Out-of-sync audio/video

**Diagnosis:**

```bash
# Check packet loss
rtp.seq  # Look for gaps
tcp.analysis.retransmission

# Check timing
rtp.timestamp  # Should increase smoothly

# Check marker bit
rtp.marker == 1  # Marks end of frame
```

**Solutions:**

1. **Increase MTU if safe:**
   ```bash
   # Check current packet sizes
   frame.len

   # If consistently small, may be over-fragmented
   ```

2. **Check network stability:**
   ```bash
   # Packet loss indicators
   tcp.analysis.retransmission or tcp.analysis.ack_lost_segment
   ```

3. **Monitor bitrate:**
   ```bash
   # Calculate average bitrate
   # bytes per second based on frame sizes and timing
   ```

### NAL Unit Analysis

```bash
# Find SPS (Parameter Set) - NAL type 7 (0x67)
frame[0:1] == "67" and tcp.dstport == 5004

# Find PPS (Picture Parameter Set) - NAL type 8 (0x68)
frame[0:1] == "68" and tcp.dstport == 5004

# Find IDR Slice - NAL type 5 (0x65)
frame[0:1] == "65" and tcp.dstport == 5004

# Find Fragmentation Unit (FU-A) - NAL type 28
frame[0:1] == "1c" and tcp.dstport == 5004
```

## Protocol Inspection Techniques

### Extract SOAP Request/Response

1. In Wireshark: Right-click → Follow → HTTP Stream
2. Look for GetDeviceInformation response
3. Check for:
   - Correct namespace URLs
   - All required fields populated
   - No truncated responses

### Examine RTP Header

```
Expected RTP header format:
Bit:    0-1     2           3-7         8-9     10-13        14-15
Field:  V(2)    P(1)        X(1)        CC(4)   M(1)         PT(7)
Example: 10      0           0           0000    0            96

Then: Sequence number (16 bits), Timestamp (32 bits), SSRC (32 bits)
```

### Verify NAL Unit Structure

```
NAL Unit Header (first byte):
Bit:  0       1-2     3-7
      F       NRI     NAL_Type

Example for IDR slice (0x65):
- F: 0 (forbidden_zero_bit - should always be 0)
- NRI: 3 (priority - 3 = highest for IDR)
- Type: 5 (IDR_slice)
```

## Automated Debugging

### Create Custom Wireshark Dissector

```lua
-- minimal_onvif_dissector.lua
-- Run: wireshark -X lua_script:minimal_onvif_dissector.lua

local onvif_proto = Proto("ONVIF", "ONVIF Camera Protocol")

function onvif_proto.dissector(buffer, pinfo, tree)
  if buffer:len() == 0 then
    return
  end

  pinfo.cols.protocol = "ONVIF"

  local subtree = tree:add(onvif_proto, buffer(), "ONVIF Data")

  -- Look for SOAP elements
  local text = buffer:string("utf-8")

  if text:match("GetDeviceInformation") then
    subtree:add_expert_info(PI_SEQUENCE, PI_NOTE, "GetDeviceInformation call")
  elseif text:match("SetHostname") then
    subtree:add_expert_info(PI_SEQUENCE, PI_NOTE, "SetHostname call")
  end

  if text:match("Fault") then
    subtree:add_expert_info(PI_RESPONSE_CODE, PI_WARN, "SOAP Fault detected")
  end
end

register_protocol(onvif_proto)
local http_port = DissectorTable.get("tcp.port")
http_port:add(8080, onvif_proto)
```

## tcpdump Analysis Scripts

### Extract ONVIF Requests

```bash
#!/bin/bash
# extract_onvif_requests.sh

pcap_file="$1"

if [ -z "$pcap_file" ]; then
  echo "Usage: $0 <capture.pcap>"
  exit 1
fi

# Extract HTTP payloads containing ONVIF
tcpdump -r "$pcap_file" -A "tcp port 8080" | \
  grep -A 20 "^<" | \
  grep -E "GetDevice|SetHostname|GetProfiles|GetCapabilities" \
  > onvif_operations.txt

echo "ONVIF operations extracted to onvif_operations.txt"
```

### Analyze Packet Loss

```bash
#!/bin/bash
# analyze_packet_loss.sh

pcap_file="$1"

echo "=== Retransmission Analysis ==="
tcpdump -r "$pcap_file" -c 10000 | \
  grep "\[TCP.*retrans" | wc -l

echo ""
echo "=== Duplicate ACK Analysis ==="
tcpdump -r "$pcap_file" -c 10000 | \
  grep "dup ack" | wc -l

echo ""
echo "=== Out of Order Analysis ==="
tcpdump -r "$pcap_file" -c 10000 | \
  grep "out-of-order" | wc -l
```

## Real-Time Monitoring

### Live ONVIF Monitor

```bash
# Monitor ONVIF requests in real-time
sudo tcpdump -i eth0 -A -s 0 \
  'tcp port 8080' \
  | grep -E 'GET|POST|SetHostname|GetDevice|SetHostname|Fault' \
  | head -20
```

### Live Streaming Quality Monitor

```bash
# Monitor RTP bitrate in real-time
sudo tcpdump -i eth0 -s 0 'rtp' \
  | awk '{print $0; if (NR % 100 == 0) print "..." NR " packets"}' \
  | tail -20
```

## Reference

For detailed packet structure diagrams and advanced analysis techniques, see `references/wireshark-filters.md`.
