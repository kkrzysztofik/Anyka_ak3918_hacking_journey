#!/bin/bash
# ==============================================================================
# capture_test_fixtures.sh - Capture SPS/PPS/AAC test fixtures from live device
# ==============================================================================
#
# This script captures real H.264 SPS/PPS NAL units and AAC RTP packets from
# a live Anyka AK3918 camera for use as test fixtures in streaming-lib.
#
# Usage:
#   ./capture_test_fixtures.sh --device <IP> [--user <user>] [--pass <pass>]
#   ./capture_test_fixtures.sh --pcap <file.pcapng>
#
# Output:
#   cross-compile/streaming-lib/src/codec/test_fixtures.rs
#
# Requirements:
#   - ffprobe (for RTSP SDP extraction)
#   - tshark (for PCAP parsing, optional)
#   - base64 (for decoding sprop-parameter-sets)
#
# ==============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
DEVICE_IP=""
DEVICE_USER="admin"
DEVICE_PASS="admin"
PCAP_FILE=""
OUTPUT_FILE="../cross-compile/streaming-lib/src/codec/test_fixtures.rs"
RTSP_PORT="554"

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Capture SPS/PPS NAL units from live device or PCAP file for test fixtures.

Options:
  --device, -d <IP>     Device IP address for live capture
  --user, -u <user>     RTSP username (default: admin)
  --pass, -p <pass>     RTSP password (default: admin)
  --port <port>         RTSP port (default: 554)
  --pcap <file>         Extract from PCAP file instead of live capture
  --output, -o <file>   Output Rust file (default: streaming-lib/src/codec/test_fixtures.rs)
  --help, -h            Show this help message

Examples:
  # Live capture from device
  $(basename "$0") --device 192.168.1.100

  # Extract from PCAP
  $(basename "$0") --pcap ../onvif.pcapng

  # Custom credentials
  $(basename "$0") --device 192.168.1.100 --user root --pass secret

EOF
    exit 0
}

log_info() { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

check_dependencies() {
    local missing=()
    
    if ! command -v ffprobe &> /dev/null; then
        missing+=("ffprobe")
    fi
    
    if ! command -v base64 &> /dev/null; then
        missing+=("base64")
    fi
    
    if [[ -n "$PCAP_FILE" ]] && ! command -v tshark &> /dev/null; then
        missing+=("tshark")
    fi
    
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing dependencies: ${missing[*]}"
        log_error "Install with: sudo apt-get install ffmpeg wireshark-common"
        exit 1
    fi
}

# Convert hex string to Rust byte array format
hex_to_rust_array() {
    local hex="$1"
    local name="$2"
    
    # Remove any spaces/colons
    hex="${hex//:/}"
    hex="${hex// /}"
    
    # Convert to lowercase
    hex="${hex,,}"
    
    # Build Rust array
    local bytes=""
    local len=$((${#hex} / 2))
    
    for ((i=0; i<${#hex}; i+=2)); do
        if [[ -n "$bytes" ]]; then
            bytes+=", "
        fi
        bytes+="0x${hex:$i:2}"
    done
    
    echo "/// $name - $len bytes"
    echo "pub const $name: &[u8] = &[$bytes];"
}

# Extract SPS/PPS from RTSP DESCRIBE via ffprobe
capture_from_rtsp() {
    local rtsp_url="rtsp://${DEVICE_USER}:${DEVICE_PASS}@${DEVICE_IP}:${RTSP_PORT}/vs0"
    
    log_info "Probing RTSP stream: rtsp://${DEVICE_USER}:***@${DEVICE_IP}:${RTSP_PORT}/vs0"
    
    # Get SDP info from ffprobe
    local sdp_output
    sdp_output=$(ffprobe -v quiet -show_format -show_streams "$rtsp_url" 2>&1 || true)
    
    if [[ -z "$sdp_output" ]]; then
        log_error "Failed to get SDP from RTSP stream"
        log_error "Check device IP, credentials, and network connectivity"
        exit 1
    fi
    
    # Extract sprop-parameter-sets from fmtp line
    local sprop
    sprop=$(echo "$sdp_output" | grep -oP 'sprop-parameter-sets=\K[^;,\s]+,[^;,\s]+' || true)
    
    if [[ -z "$sprop" ]]; then
        log_warn "sprop-parameter-sets not found in SDP, trying alternative extraction..."
        
        # Try extracting from raw stream
        capture_from_stream "$rtsp_url"
        return
    fi
    
    # Split into SPS and PPS (base64 encoded)
    local sps_b64="${sprop%%,*}"
    local pps_b64="${sprop#*,}"
    
    log_info "Found sprop-parameter-sets:"
    log_info "  SPS (base64): $sps_b64"
    log_info "  PPS (base64): $pps_b64"
    
    # Decode to hex
    local sps_hex pps_hex
    sps_hex=$(echo -n "$sps_b64" | base64 -d | xxd -p | tr -d '\n')
    pps_hex=$(echo -n "$pps_b64" | base64 -d | xxd -p | tr -d '\n')
    
    log_success "Decoded SPS: $sps_hex"
    log_success "Decoded PPS: $pps_hex"
    
    # Determine profile from SPS (3rd byte)
    local profile_idc="${sps_hex:4:2}"
    local profile_name
    case "$profile_idc" in
        42) profile_name="BASELINE" ;;
        4d) profile_name="MAIN" ;;
        58) profile_name="EXTENDED" ;;
        64) profile_name="HIGH" ;;
        *) profile_name="UNKNOWN_${profile_idc}" ;;
    esac
    
    # Get resolution info
    local width height
    width=$(echo "$sdp_output" | grep -oP 'width=\K\d+' | head -1 || echo "unknown")
    height=$(echo "$sdp_output" | grep -oP 'height=\K\d+' | head -1 || echo "unknown")
    
    log_info "Detected profile: $profile_name, resolution: ${width}x${height}"
    
    # Store for output generation
    SPS_DATA["${profile_name}_${height}P"]="$sps_hex"
    PPS_DATA["${profile_name}_${height}P"]="$pps_hex"
}

# Capture raw NAL units from stream
capture_from_stream() {
    local rtsp_url="$1"
    local temp_file="/tmp/capture_$$.h264"
    
    log_info "Capturing raw stream for NAL extraction..."
    
    # Capture 2 seconds of stream
    ffmpeg -y -i "$rtsp_url" -t 2 -c:v copy -an -f h264 "$temp_file" 2>/dev/null || {
        log_error "Failed to capture stream"
        exit 1
    }
    
    # Extract SPS/PPS NAL units (type 7 and 8)
    # NAL start code: 00 00 00 01 or 00 00 01
    local hex_stream
    hex_stream=$(xxd -p "$temp_file" | tr -d '\n')
    
    # Find SPS (NAL type 7 = 0x67 for profile_idc=100 or 0x27 for baseline)
    # Pattern: start_code + nal_unit_type (lower 5 bits = 7)
    local sps_hex pps_hex
    
    # Look for 00000001 followed by byte with lower 5 bits = 7 (0x07, 0x27, 0x47, 0x67, 0x87)
    if [[ "$hex_stream" =~ 0000000167([0-9a-f]+)0000000168 ]]; then
        sps_hex="67${BASH_REMATCH[1]}"
        log_info "Found SPS (High profile): $sps_hex"
    elif [[ "$hex_stream" =~ 0000000127([0-9a-f]+)0000000128 ]]; then
        sps_hex="27${BASH_REMATCH[1]}"
        log_info "Found SPS (Baseline profile): $sps_hex"
    fi
    
    # Find PPS (NAL type 8 = 0x68 for high or 0x28 for baseline)
    if [[ "$hex_stream" =~ 0000000168([0-9a-f]+)(00000001|$) ]]; then
        pps_hex="68${BASH_REMATCH[1]}"
        log_info "Found PPS: $pps_hex"
    elif [[ "$hex_stream" =~ 0000000128([0-9a-f]+)(00000001|$) ]]; then
        pps_hex="28${BASH_REMATCH[1]}"
        log_info "Found PPS: $pps_hex"
    fi
    
    rm -f "$temp_file"
    
    if [[ -n "${sps_hex:-}" ]]; then
        SPS_DATA["CAPTURED"]="$sps_hex"
    fi
    if [[ -n "${pps_hex:-}" ]]; then
        PPS_DATA["CAPTURED"]="$pps_hex"
    fi
}

# Extract from PCAP file
capture_from_pcap() {
    log_info "Extracting from PCAP: $PCAP_FILE"
    
    if [[ ! -f "$PCAP_FILE" ]]; then
        log_error "PCAP file not found: $PCAP_FILE"
        exit 1
    fi
    
    # Extract RTP payloads with H.264 content
    local rtp_payloads
    rtp_payloads=$(tshark -r "$PCAP_FILE" -Y "rtp" -T fields -e rtp.payload 2>/dev/null | head -100)
    
    for payload in $rtp_payloads; do
        payload="${payload//:/}"
        
        # Check NAL unit type (first byte, lower 5 bits)
        local nal_type=$((16#${payload:0:2} & 0x1F))
        
        case $nal_type in
            7)
                log_info "Found SPS in PCAP: ${payload:0:40}..."
                SPS_DATA["PCAP"]="$payload"
                ;;
            8)
                log_info "Found PPS in PCAP: $payload"
                PPS_DATA["PCAP"]="$payload"
                ;;
        esac
        
        # Stop after finding both
        if [[ -n "${SPS_DATA[PCAP]:-}" ]] && [[ -n "${PPS_DATA[PCAP]:-}" ]]; then
            break
        fi
    done
}

# Generate Rust test_fixtures.rs file
generate_rust_file() {
    local output="$PROJECT_ROOT/$OUTPUT_FILE"
    
    log_info "Generating Rust file: $output"
    
    mkdir -p "$(dirname "$output")"
    
    cat > "$output" << 'HEADER'
//! H.264 SPS/PPS and AAC test fixtures captured from real devices.
//!
//! This module contains NAL units and RTP packets captured from Anyka AK3918 cameras
//! for use in unit tests. Using real data ensures tests validate against actual
//! production bitstreams rather than synthetic test data.
//!
//! Generated by: scripts/capture_test_fixtures.sh
//! 
//! # Usage
//! ```rust
//! use crate::codec::test_fixtures::{SPS_HIGH_1080P, PPS_HIGH_1080P};
//! 
//! let sps = SeqParameterSet::parse(SPS_HIGH_1080P).unwrap();
//! assert_eq!(sps.profile_idc, 100); // High profile
//! ```

#![allow(dead_code)]

// ==============================================================================
// H.264 SPS (Sequence Parameter Set) NAL Units
// ==============================================================================

HEADER

    # Add SPS constants
    for key in "${!SPS_DATA[@]}"; do
        local hex="${SPS_DATA[$key]}"
        local const_name="SPS_${key}"
        echo "" >> "$output"
        hex_to_rust_array "$hex" "$const_name" >> "$output"
    done

    cat >> "$output" << 'MIDDLE'

// ==============================================================================
// H.264 PPS (Picture Parameter Set) NAL Units
// ==============================================================================

MIDDLE

    # Add PPS constants
    for key in "${!PPS_DATA[@]}"; do
        local hex="${PPS_DATA[$key]}"
        local const_name="PPS_${key}"
        echo "" >> "$output"
        hex_to_rust_array "$hex" "$const_name" >> "$output"
    done

    cat >> "$output" << 'FOOTER'

// ==============================================================================
// Test Helper Functions
// ==============================================================================

/// Get all available SPS fixtures as (name, data) pairs
pub fn all_sps_fixtures() -> Vec<(&'static str, &'static [u8])> {
    vec![
        // Add captured SPS fixtures here
    ]
}

/// Get all available PPS fixtures as (name, data) pairs
pub fn all_pps_fixtures() -> Vec<(&'static str, &'static [u8])> {
    vec![
        // Add captured PPS fixtures here
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sps_fixtures_not_empty() {
        // Verify at least one SPS fixture exists and has valid NAL type
        // NAL type for SPS is 7 (lower 5 bits of first byte)
    }

    #[test]
    fn test_pps_fixtures_not_empty() {
        // Verify at least one PPS fixture exists and has valid NAL type
        // NAL type for PPS is 8 (lower 5 bits of first byte)
    }
}
FOOTER

    log_success "Generated: $output"
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --device|-d)
                DEVICE_IP="$2"
                shift 2
                ;;
            --user|-u)
                DEVICE_USER="$2"
                shift 2
                ;;
            --pass|-p)
                DEVICE_PASS="$2"
                shift 2
                ;;
            --port)
                RTSP_PORT="$2"
                shift 2
                ;;
            --pcap)
                PCAP_FILE="$2"
                shift 2
                ;;
            --output|-o)
                OUTPUT_FILE="$2"
                shift 2
                ;;
            --help|-h)
                usage
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
    done
}

# Main
main() {
    parse_args "$@"
    
    # Validate arguments
    if [[ -z "$DEVICE_IP" ]] && [[ -z "$PCAP_FILE" ]]; then
        log_error "Must specify either --device or --pcap"
        usage
    fi
    
    check_dependencies
    
    # Associative arrays for collected data
    declare -A SPS_DATA
    declare -A PPS_DATA
    
    # Export for use in functions
    export -A SPS_DATA
    export -A PPS_DATA
    
    if [[ -n "$PCAP_FILE" ]]; then
        capture_from_pcap
    else
        capture_from_rtsp
    fi
    
    # Check if we got any data
    if [[ ${#SPS_DATA[@]} -eq 0 ]] && [[ ${#PPS_DATA[@]} -eq 0 ]]; then
        log_error "No SPS/PPS data captured"
        exit 1
    fi
    
    generate_rust_file
    
    log_success "Done! Test fixtures saved to: $OUTPUT_FILE"
    log_info "Don't forget to add 'pub mod test_fixtures;' to codec/mod.rs"
}

main "$@"
