# H.264 Codec Patterns Reference

## NAL Unit Extraction from Bitstream

Extract NAL units from H.264 elementary stream (annex-B format):

```rust
pub struct NalUnitReader {
    data: Vec<u8>,
    position: usize,
}

impl NalUnitReader {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, position: 0 }
    }

    /// Find next start code (0x000001 or 0x00000001)
    pub fn next_nal_unit(&mut self) -> Option<Vec<u8>> {
        // Skip leading zeros and current start code
        while self.position < self.data.len() {
            if self.data[self.position] != 0 {
                break;
            }
            self.position += 1;
        }

        let start = self.position;

        // Find next start code
        while self.position < self.data.len() - 3 {
            if self.data[self.position] == 0
                && self.data[self.position + 1] == 0
                && (self.data[self.position + 2] == 1
                    || (self.position + 3 < self.data.len()
                        && self.data[self.position + 2] == 0
                        && self.data[self.position + 3] == 1))
            {
                break;
            }
            self.position += 1;
        }

        if start == self.position {
            None
        } else {
            Some(self.data[start..self.position].to_vec())
        }
    }
}
```

## SPS Parsing (Sequence Parameter Set)

Extract video dimensions from SPS:

```rust
pub struct SPSParser;

impl SPSParser {
    /// Parse SPS to extract video dimensions
    pub fn parse_dimensions(sps: &[u8]) -> Result<(u32, u32), ParseError> {
        if sps.len() < 4 {
            return Err(ParseError::TooShort);
        }

        let mut br = BitReader::new(&sps[1..]); // Skip NAL header

        // Skip profile, level indications
        br.read_bits(8)?; // profile_idc
        br.read_bits(1)?; // constraint_set0_flag
        br.read_bits(1)?; // constraint_set1_flag
        br.read_bits(1)?; // constraint_set2_flag
        br.read_bits(5)?; // reserved
        br.read_bits(8)?; // level_idc

        br.read_exp_golomb()?; // seq_parameter_set_id

        // profile_idc dependent parsing
        let profile_idc = sps[1];
        if profile_idc == 100 || profile_idc == 110 || profile_idc == 122 || profile_idc == 244 {
            br.read_exp_golomb()?; // chroma_format_idc
            br.read_exp_golomb()?; // bit_depth_luma_minus8
            br.read_exp_golomb()?; // bit_depth_chroma_minus8
            br.read_bits(1)?;      // qpprime_y_zero_transform_bypass_flag
        }

        br.read_exp_golomb()?; // log2_max_frame_num_minus4

        let pic_order_cnt_type = br.read_exp_golomb()?;
        if pic_order_cnt_type == 0 {
            br.read_exp_golomb()?; // log2_max_pic_order_cnt_lsb_minus4
        }

        br.read_exp_golomb()?; // max_num_ref_frames
        br.read_bits(1)?;      // gaps_in_frame_num_value_allowed_flag

        // pic_width_in_mbs_minus1 and pic_height_in_map_units_minus1
        let pic_width_in_mbs_minus1 = br.read_exp_golomb()?;
        let pic_height_in_map_units_minus1 = br.read_exp_golomb()?;

        let frame_mbs_only_flag = br.read_bits(1)?;

        let width = ((pic_width_in_mbs_minus1 + 1) * 16) as u32;
        let height = ((pic_height_in_map_units_minus1 + 1) * 16 * (2 - frame_mbs_only_flag)) as u32;

        Ok((width, height))
    }
}
```

## STAP-A Unpacking

Handle Single-Time Aggregation packets:

```rust
pub fn unpack_stap_a(payload: &[u8]) -> Result<Vec<Vec<u8>>, UnpackError> {
    if payload.len() < 3 {
        return Err(UnpackError::TooShort);
    }

    // Skip STAP-A header (1 byte)
    let mut offset = 1;
    let mut nalus = Vec::new();

    while offset < payload.len() {
        // Read 2-byte size
        if offset + 2 > payload.len() {
            break;
        }

        let size = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
        offset += 2;

        // Extract NAL unit
        if offset + size > payload.len() {
            return Err(UnpackError::InvalidSize);
        }

        nalus.push(payload[offset..offset + size].to_vec());
        offset += size;
    }

    Ok(nalus)
}
```

## FU-A Fragmentation

Fragmentation algorithm for large NAL units:

```rust
pub struct FragmentationContext {
    original_nal_type: u8,
    fragments: Vec<Vec<u8>>,
    sequence_numbers: Vec<u32>,
    timestamps: Vec<u32>,
}

impl FragmentationContext {
    pub fn new(nal_type: u8) -> Self {
        Self {
            original_nal_type: nal_type,
            fragments: Vec::new(),
            sequence_numbers: Vec::new(),
            timestamps: Vec::new(),
        }
    }

    pub fn add_fragment(
        &mut self,
        payload: &[u8],
        seq: u32,
        ts: u32,
        is_end: bool,
    ) -> Result<Option<Vec<u8>>, ReassemblyError> {
        // FU Indicator at payload[1]
        let s_bit = (payload[1] & 0x80) != 0;
        let e_bit = (payload[1] & 0x40) != 0;
        let nal_type = payload[1] & 0x1F;

        if nal_type != self.original_nal_type {
            return Err(ReassemblyError::TypeMismatch);
        }

        // Extract fragment data (skip FU header and indicator)
        self.fragments.push(payload[2..].to_vec());
        self.sequence_numbers.push(seq);
        self.timestamps.push(ts);

        if !e_bit && !is_end {
            return Ok(None); // More fragments coming
        }

        // Reassemble complete NAL unit
        let mut nalu = vec![0x60 | self.original_nal_type]; // NAL header
        for fragment in &self.fragments {
            nalu.extend_from_slice(fragment);
        }

        Ok(Some(nalu))
    }
}
```

## RTCP Feedback Integration

Handle RTCP packets for stream synchronization:

```rust
pub trait TRtpReceiverForRtcp {
    fn on_packet_for_rtcp_handler(&self, packet: &RtpPacket) -> RtcpInfo;
}

impl TRtpReceiverForRtcp for RtpH264Packer {
    fn on_packet_for_rtcp_handler(&self, packet: &RtpPacket) -> RtcpInfo {
        RtcpInfo {
            sequence_number: packet.header.sequence_number,
            timestamp: packet.header.timestamp,
            ssrc: packet.header.ssrc,
            marker: packet.marker,
            payload_type: packet.header.payload_type,
        }
    }
}
```

## MTU-Aware Packing

Calculate optimal packet sizes based on MTU:

```rust
pub fn calculate_max_payload_size(mtu: usize, has_rtp_ext: bool) -> usize {
    const RTP_HEADER: usize = 12;
    const FU_HEADER: usize = 2; // For fragmented packets
    const IP_HEADER: usize = 20;
    const UDP_HEADER: usize = 8;
    const RTP_EXT: usize = 4; // If present

    let overhead = RTP_HEADER + IP_HEADER + UDP_HEADER;
    let ext = if has_rtp_ext { RTP_EXT } else { 0 };

    mtu.saturating_sub(overhead + ext + FU_HEADER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_payload_ethernet() {
        // Standard Ethernet MTU 1500
        let size = calculate_max_payload_size(1500, false);
        assert!(size > 1400); // Leaves room for headers
    }

    #[test]
    fn test_max_payload_wlan() {
        // Typical WiFi MTU 1200
        let size = calculate_max_payload_size(1200, false);
        assert!(size > 1100);
    }
}
```

## Timestamp Management

Maintain consistent RTP timestamps across packets:

```rust
pub struct TimestampGenerator {
    base_timestamp: u32,
    clock_rate: u32, // Typically 90000 for video
    last_frame_time: std::time::Instant,
}

impl TimestampGenerator {
    pub fn new(clock_rate: u32) -> Self {
        Self {
            base_timestamp: rand::random(),
            clock_rate,
            last_frame_time: std::time::Instant::now(),
        }
    }

    pub fn get_timestamp_for_frame(&mut self) -> u32 {
        let elapsed = self.last_frame_time.elapsed().as_secs_f64();
        let increment = (elapsed * self.clock_rate as f64) as u32;

        self.base_timestamp.wrapping_add(increment)
    }

    pub fn mark_frame_sent(&mut self) {
        self.last_frame_time = std::time::Instant::now();
    }
}
```

## Error Detection and Handling

Validate packet structure and detect transmission errors:

```rust
pub fn validate_rtp_packet(packet: &[u8]) -> Result<(), ValidationError> {
    if packet.len() < 12 {
        return Err(ValidationError::TooShort);
    }

    let version = (packet[0] >> 6) & 0x03;
    if version != 2 {
        return Err(ValidationError::UnsupportedVersion(version));
    }

    let cc = packet[0] & 0x0F;
    let csrc_len = cc as usize * 4;
    let min_len = 12 + csrc_len;

    if packet.len() < min_len {
        return Err(ValidationError::InvalidCsrcLength);
    }

    Ok(())
}
```
