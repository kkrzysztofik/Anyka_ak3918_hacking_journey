use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Generates a minimal but valid AAC file in ADTS format for testing
/// This creates a file with valid ADTS headers and some synthetic AAC frames
pub fn generate_test_aac_file(path: &str, sample_rate: u32, num_frames: usize) -> std::io::Result<()> {
    let mut file = File::create(path)?;

    // Determine sampling frequency index
    let sampling_freq_index = match sample_rate {
        96000 => 0,
        88200 => 1,
        64000 => 2,
        48000 => 3,
        44100 => 4,
        32000 => 5,
        24000 => 6,
        22050 => 7,
        16000 => 8,
        12000 => 9,
        11025 => 10,
        8000 => 11,
        7350 => 12,
        _ => 3, // Default to 48kHz
    };

    // AAC-LC profile
    let profile = 2; // AAC-LC (1=Main, 2=LC, 3=SSR, 4=LTP)
    let channel_config = 2; // Stereo

    // Generate frames
    for frame_idx in 0..num_frames {
        // Create synthetic AAC payload (variable size for realism)
        let payload_size = 128 + (frame_idx % 64); // 128-192 bytes

        // ADTS frame length = header (7 bytes) + payload
        let frame_length = 7 + payload_size;

        // Construct ADTS header (7 bytes, no CRC)
        let mut adts_header = vec![0u8; 7];

        // Byte 0-1: Sync word (0xFFF) + ID + layer + protection_absent
        adts_header[0] = 0xFF;
        adts_header[1] = 0xF1; // 0xF0 (sync continuation) + 0x01 (no CRC)

        // Byte 2: Profile + sampling_freq_index + channel (partial)
        adts_header[2] = ((profile - 1) << 6) | (sampling_freq_index << 2) | ((channel_config >> 2) & 0x01);

        // Byte 3: Channel (partial) + frame_length (partial)
        adts_header[3] = ((channel_config & 0x03) << 6) | ((frame_length >> 11) & 0x03) as u8;

        // Byte 4: Frame length (middle 8 bits)
        adts_header[4] = ((frame_length >> 3) & 0xFF) as u8;

        // Byte 5: Frame length (low 3 bits) + buffer fullness (5 bits)
        adts_header[5] = (((frame_length & 0x07) << 5) | 0x1F) as u8;

        // Byte 6: Buffer fullness + num_raw_data_blocks
        adts_header[6] = 0xFC; // VBR + single raw data block

        // Write ADTS header
        file.write_all(&adts_header)?;

        // Write synthetic AAC payload
        let mut payload = vec![0u8; payload_size];
        for i in 0..payload_size {
            payload[i] = ((frame_idx * 17 + i) & 0xFF) as u8;
        }
        file.write_all(&payload)?;
    }

    Ok(())
}

fn main() {
    let output_path = "tests/fixtures/test_audio.aac";
    match generate_test_aac_file(output_path, 48000, 100) {
        Ok(_) => println!("Generated test AAC file at {} (100 frames @ 48kHz)", output_path),
        Err(e) => eprintln!("Failed to generate test AAC file: {}", e),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_generate_test_aac_file() {
        let test_file = "/tmp/test_generated.aac";

        // Clean up if it exists
        let _ = fs::remove_file(test_file);

        // Generate the file
        assert!(generate_test_aac_file(test_file, 48000, 10).is_ok());

        // Verify file was created
        assert!(Path::new(test_file).exists());

        // Verify file has content
        let metadata = fs::metadata(test_file).unwrap();
        assert!(metadata.len() > 0);

        // Verify file starts with ADTS sync word
        let data = fs::read(test_file).unwrap();
        assert!(data.len() >= 7);
        assert_eq!(data[0], 0xFF);
        assert_eq!(data[1] & 0xF0, 0xF0);

        // Clean up
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_generated_file_contains_valid_adts() {
        let test_file = "/tmp/test_aac_with_adts.aac";
        let _ = fs::remove_file(test_file);

        generate_test_aac_file(test_file, 48000, 5).unwrap();

        let data = fs::read(test_file).unwrap();

        // Find ADTS sync words (should have 5 frames)
        let sync_count = data.windows(2).filter(|w| {
            w[0] == 0xFF && (w[1] & 0xF0) == 0xF0
        }).count();

        assert!(sync_count >= 5, "Expected at least 5 ADTS frames, found {}", sync_count);

        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_different_sample_rates() {
        let test_file = "/tmp/test_aac_44k.aac";
        let _ = fs::remove_file(test_file);

        // Test with 44.1kHz
        generate_test_aac_file(test_file, 44100, 3).unwrap();

        let data = fs::read(test_file).unwrap();
        
        // Verify sampling frequency index in ADTS header
        // For 44.1kHz, index should be 4 (bits 2-5 of byte 2)
        assert_eq!((data[2] >> 2) & 0x0F, 4);

        let _ = fs::remove_file(test_file);
    }
}
