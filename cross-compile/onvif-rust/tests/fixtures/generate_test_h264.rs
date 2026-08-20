use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Generates a minimal but valid H264 file in Annex-B format for testing
/// This creates a file with SPS, PPS, and a few I-frames and P-frames
pub fn generate_test_h264_file(path: &str) -> std::io::Result<()> {
    let mut file = File::create(path)?;

    // Annex-B start code (4 bytes)
    let start_code_4: &[u8] = &[0x00, 0x00, 0x00, 0x01];

    // Minimal valid SPS (Sequence Parameter Set)
    // This is a simplified SPS that represents a baseline H264 profile
    // Profile: 42 (Baseline), Level: c0 (Level 3.0), Constraints: 10
    let sps_data: &[u8] = &[
        0x27, 0x42, 0xc0, 0x28, 0xd9, 0x05, 0x05, 0x14, 0x11, 0x00,
    ];

    // Minimal valid PPS (Picture Parameter Set)
    let pps_data: &[u8] = &[0x28, 0xce, 0x06, 0xe2];

    // Write SPS NAL unit
    file.write_all(start_code_4)?;
    file.write_all(sps_data)?;

    // Write PPS NAL unit
    file.write_all(start_code_4)?;
    file.write_all(pps_data)?;

    // Generate synthetic I-frames and P-frames
    // For testing, we create minimal IDR frames with synthetic data
    for frame_idx in 0..10 {
        let is_key_frame = frame_idx == 0 || frame_idx == 5;
        let nal_unit_type = if is_key_frame { 0x05 } else { 0x01 };

        // Minimal frame payload
        let mut frame_data = vec![nal_unit_type];

        // Add some synthetic payload data based on frame index
        for i in 0..64 {
            frame_data.push(((frame_idx * 17 + i) & 0xff) as u8);
        }

        // Write frame NAL unit with start code
        file.write_all(start_code_4)?;
        file.write_all(&frame_data)?;
    }

    Ok(())
}

fn main() {
    let output_path = "tests/fixtures/test_video.h264";
    match generate_test_h264_file(output_path) {
        Ok(_) => println!("Generated test H264 file at {}", output_path),
        Err(e) => eprintln!("Failed to generate test H264 file: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_generate_test_h264_file() {
        let test_file = "/tmp/test_generated.h264";

        // Clean up if it exists
        let _ = fs::remove_file(test_file);

        // Generate the file
        assert!(generate_test_h264_file(test_file).is_ok());

        // Verify file was created
        assert!(Path::new(test_file).exists());

        // Verify file has content
        let metadata = fs::metadata(test_file).unwrap();
        assert!(metadata.len() > 0);

        // Verify file starts with start code
        let data = fs::read(test_file).unwrap();
        assert!(data.len() >= 4);
        assert_eq!(&data[0..4], &[0x00, 0x00, 0x00, 0x01]);

        // Clean up
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_generated_file_contains_sps_pps() {
        let test_file = "/tmp/test_h264_with_sps_pps.h264";
        let _ = fs::remove_file(test_file);

        generate_test_h264_file(test_file).unwrap();

        let data = fs::read(test_file).unwrap();

        // Find SPS (NAL type 7) - looks for pattern: start_code + 0x27
        let sps_present = data.windows(5).any(|w| {
            w[0..4] == [0x00, 0x00, 0x00, 0x01] && (w[4] & 0x1f) == 0x07
        });

        // Find PPS (NAL type 8) - looks for pattern: start_code + 0x28
        let pps_present = data.windows(5).any(|w| {
            w[0..4] == [0x00, 0x00, 0x00, 0x01] && (w[4] & 0x1f) == 0x08
        });

        assert!(sps_present, "SPS not found in generated file");
        assert!(pps_present, "PPS not found in generated file");

        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_generated_file_contains_idr_frames() {
        let test_file = "/tmp/test_h264_with_frames.h264";
        let _ = fs::remove_file(test_file);

        generate_test_h264_file(test_file).unwrap();

        let data = fs::read(test_file).unwrap();

        // Find IDR frames (NAL type 5)
        let idr_frames = data.windows(5)
            .filter(|w| {
                w[0..4] == [0x00, 0x00, 0x00, 0x01] && (w[4] & 0x1f) == 0x05
            })
            .count();

        assert!(idr_frames > 0, "No IDR frames found in generated file");

        let _ = fs::remove_file(test_file);
    }
}

