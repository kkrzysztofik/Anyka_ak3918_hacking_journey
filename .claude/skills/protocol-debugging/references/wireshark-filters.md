# Wireshark Filters and Network Analysis Reference

## Quick Filter Cheatsheet

```
# Basic Filters
ip.src == 192.168.1.100              # Source IP
ip.dst == 192.168.1.100              # Destination IP
tcp.port == 8080                     # Any port
tcp.srcport == 8080                  # Source port
tcp.dstport == 8080                  # Destination port

# Logical Operators
frame && tcp.port == 8080            # AND
frame || tcp.port == 554             # OR
!tcp.port == 22                      # NOT (NOT ssh)

# Combining
(ip.src == 192.168.1.100 && tcp.port == 8080) || tcp.port == 554
```

## ONVIF Analysis Filters

### Device Service Filters

```
# GetDeviceInformation
tcp.dstport == 8080 && frame contains "GetDeviceInformation"

# GetCapabilities
frame contains "GetCapabilities"

# GetHostname
frame contains "GetHostname"

# SetHostname
frame contains "SetHostname"

# GetUsers
frame contains "GetUsers"

# CreateUser
frame contains "CreateUser"

# DeleteUser
frame contains "DeleteUser"

# System Reboot
frame contains "SystemReboot"
```

### Media Service Filters

```
# GetProfiles
tcp.dstport == 8080 && frame contains "GetProfiles"

# CreateProfile
frame contains "CreateProfile"

# DeleteProfile
frame contains "DeleteProfile"

# GetStreamUri
frame contains "GetStreamUri"

# GetVideoEncoderConfiguration
frame contains "GetVideoEncoderConfiguration"
```

### PTZ Service Filters

```
# ContinuousMove
frame contains "ContinuousMove"

# AbsoluteMove
frame contains "AbsoluteMove"

# Stop
frame contains "Stop"

# GotoHomePosition
frame contains "GotoHomePosition"
```

### Error Analysis Filters

```
# Any SOAP Fault
frame contains "Fault" or frame contains "<s:Fault"

# Authentication Failures
tcp.dstport == 8080 && frame contains "NotAuthorized"

# Invalid Arguments
frame contains "InvalidArgVal"

# Action Not Supported
frame contains "ActionNotSupported"

# Server Errors
http.response.code == 500

# Timeout Errors
tcp.analysis.ack_lost_segment
```

## RTSP/RTP Filters

### Streaming Protocol Filters

```
# RTSP Connection
tcp.port == 554

# RTSP METHOD (OPTIONS, DESCRIBE, SETUP, PLAY, etc)
frame contains "RTSP"

# RTSP OPTIONS
tcp.port == 554 && frame contains "OPTIONS"

# RTSP SETUP (stream initialization)
frame contains "SETUP"

# RTSP PLAY (start streaming)
frame contains "PLAY"

# RTSP TEARDOWN (stop streaming)
frame contains "TEARDOWN"
```

### RTP Payload Filters

```
# All RTP packets
rtp

# RTP with H.264 payload
rtp.pt == 96

# RTP with dynamic payload
rtp.pt > 95

# RTP packets with marker bit set (end of frame)
rtp.marker == 1

# RTP packets without marker (mid-frame fragments)
rtp.marker == 0
```

### RTCP Filters

```
# RTCP Sender Report
rtcp.type == 200

# RTCP Receiver Report
rtcp.type == 201

# RTCP Source Description
rtcp.type == 202

# RTCP Goodbye
rtcp.type == 203

# All RTCP
rtcp
```

## Performance Analysis Filters

### Network Quality Filters

```
# Retransmitted packets (packet loss)
tcp.analysis.retransmission

# Duplicate ACKs (network congestion)
tcp.analysis.duplicate_ack

# Out-of-order packets
tcp.analysis.out_of_order

# Ack lost segment
tcp.analysis.ack_lost_segment

# Window update
tcp.analysis.window_update

# Zero window
tcp.analysis.zero_window
```

### Response Time Filters

```
# HTTP responses taking >1 second
http && http.time > 1000

# HTTP responses taking >2 seconds
http && http.time > 2000

# TCP conversation time
tcp.time_delta > 1  # Time between packets > 1 second
```

### Packet Size Analysis

```
# Large packets (>1500 bytes - potential fragmentation)
frame.len > 1500

# Small packets (<100 bytes - might indicate fragmentation)
frame.len < 100

# Packets in specific range
frame.len >= 1000 && frame.len <= 1200
```

## HTTP Status Filters

```
# Successful responses
http.response.code == 200

# Unauthorized
http.response.code == 401

# Forbidden
http.response.code == 403

# Not Found
http.response.code == 404

# Server Errors
http.response.code >= 500

# All errors
http.response.code >= 400
```

## Advanced Analysis Filters

### NAL Unit Detection (H.264 Video)

```
# SPS (Sequence Parameter Set) - NAL type 7
# Raw packet: 0x67 (binary 01100111)
frame[0:1] == "67" || frame[1:1] == "67"

# PPS (Picture Parameter Set) - NAL type 8
# Raw packet: 0x68 (binary 01101000)
frame[0:1] == "68" || frame[1:1] == "68"

# IDR Slice (Keyframe) - NAL type 5
# Raw packet: 0x65 (binary 01100101)
frame[0:1] == "65" || frame[1:1] == "65"

# Fragmentation Unit (FU-A) - NAL type 28
# Raw packet: 0x1c in FU header
frame[0:1] == "1c"
```

### RTP Stream Analysis

```
# Same SSRC (Synchronization Source)
rtp.ssrc == 0x12345678

# Specific RTP Sequence Range
rtp.seq > 1000 && rtp.seq < 2000

# RTP Timestamp progression
# Should increase smoothly for real-time
rtp.timestamp > 90000

# Increasing sequence numbers
frame previous rtp.seq && rtp.seq == previous rtp.seq + 1
```

### Authentication Filters

```
# WS-Security Headers
frame contains "UsernameToken" && tcp.dstport == 8080

# Password Digest
frame contains "PasswordDigest"

# Nonce (random value)
frame contains "Nonce"

# Timestamp (WS-Security)
frame contains "<wsu:Created"
```

## Conversation Analysis

### Single Connection Streams

```
# First conversation
tcp.stream == 0

# Show only HTTP conversations
http && tcp.stream == 0

# Multiple conversations (debugging load)
tcp.stream
```

### Connection Patterns

```
# New connections (SYN flag)
tcp.flags.syn == 1

# Connection termination (FIN flag)
tcp.flags.fin == 1

# Reset connections (RST flag)
tcp.flags.reset == 1

# ACK flood
tcp.flags.ack == 1 && frame.len < 100
```

## Composite Filters

### Complete ONVIF Session Analysis

```
(tcp.dstport == 8080 &&
 ((frame contains "GetDevice") ||
  (frame contains "SetHostname") ||
  (frame contains "GetCapabilities")))
```

### Streaming Quality Monitor

```
((rtp && rtp.pt == 96) ||  # H.264 RTP
 (rtcp) ||                 # RTCP feedback
 (tcp.analysis.retransmission))  # Loss detection
```

### Authentication Troubleshooting

```
(tcp.dstport == 8080 &&
 ((frame contains "UsernameToken") ||  # Auth attempt
  (frame contains "NotAuthorized") ||  # Failure
  (http.response.code == 401)))  # HTTP 401
```

### Network Quality Summary

```
(tcp.analysis.retransmission ||
 tcp.analysis.duplicate_ack ||
 tcp.analysis.out_of_order ||
 tcp.analysis.window_update)
```

## Filter Syntax Tips

### Hex Matching

```
# Look for specific byte patterns
frame[0:1] == "ff"           # First byte is 0xFF
frame[10:2] == "dead"        # Bytes 10-11 are 0xDEAD
frame contains "67 68 65"    # Contains these hex bytes
```

### String Matching

```
# Case-insensitive
ip.addr == 192.168.1.100

# Partial string
frame contains "Device"
frame contains "Stream"
```

### Range Expressions

```
# Port ranges
tcp.port >= 5000 && tcp.port <= 65535

# Packet counts
tcp.len > 100

# Time ranges
frame.time >= "2024-01-15 10:00:00" &&
frame.time <= "2024-01-15 10:05:00"
```

## Performance Optimization

### Capture Filter (More Efficient)

```
# Capture only relevant traffic
tcpdump 'tcp port 8080 or tcp port 554 or udp port 5000-65535'

# Save to file for later analysis
tcpdump -i eth0 -w capture.pcap 'host 192.168.1.100'
```

### Display Filter (After Capture)

```
# Applied after capture - slower but powerful
# Use for detailed analysis in Wireshark
tcp.dstport == 8080 && frame contains "Fault"
```

## Saving and Reusing Filters

### In Wireshark

1. Enter filter in the filter bar
2. Click bookmark icon to save
3. Access from "Saved Filters" dropdown

### Command Line Preset

```
# Create filter file
echo "tcp.dstport == 8080" > ~/.wireshark/profiles/Default/filter_expressions.txt

# Or use -F flag
wireshark -F 'tcp.dstport == 8080' capture.pcap
```
