// Integration tests for FLV muxing and demuxing
// Tests end-to-end FLV container creation, tag writing, and parsing

use bytes::BytesMut;
use streaming_lib::container::define::tag_type;
use streaming_lib::container::demuxer::FlvDemuxer;
use streaming_lib::container::muxer::FlvMuxer;

/// Tests FLV header generation for audio/video flag combinations.
#[tokio::test]
async fn test_flv_header_generation() {
    let mut muxer = FlvMuxer::new();

    // Test header with audio and video
    muxer.write_flv_header(true, true).unwrap();
    let header = muxer.writer.get_current_bytes();
    assert_eq!(header.len(), 9);
    assert_eq!(header[0], b'F');
    assert_eq!(header[1], b'L');
    assert_eq!(header[2], b'V');
    assert_eq!(header[4] & 0x05, 0x05); // Both audio and video flags

    // Test header with audio only
    let mut muxer = FlvMuxer::new();
    muxer.write_flv_header(true, false).unwrap();
    let header_audio = muxer.writer.get_current_bytes();
    assert_eq!(header_audio[4] & 0x05, 0x04); // Audio flag only

    // Test header with video only
    let mut muxer = FlvMuxer::new();
    muxer.write_flv_header(false, true).unwrap();
    let header_video = muxer.writer.get_current_bytes();
    assert_eq!(header_video[4] & 0x05, 0x01); // Video flag only
}

/// Tests FLV tag header/body writing for audio and video tags.
#[tokio::test]
async fn test_flv_tag_writing() {
    let mut muxer = FlvMuxer::new();

    // Write video tag
    let video_data = BytesMut::from(
        &[
            0x17, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1e,
        ][..],
    );
    let body_size = video_data.len() as u32;
    muxer
        .write_flv_tag_header(tag_type::VIDEO, body_size, 0)
        .unwrap();
    muxer.write_flv_tag_body(video_data.clone()).unwrap();
    let tag = muxer.writer.get_current_bytes();

    assert!(tag.len() > 11); // Tag header (11 bytes) + data
    assert_eq!(tag[0], tag_type::VIDEO);

    // Write audio tag
    let mut muxer = FlvMuxer::new();
    let audio_data = BytesMut::from(&[0xaf, 0x00, 0x12, 0x10, 0x56, 0xe5][..]);
    let body_size = audio_data.len() as u32;
    muxer
        .write_flv_tag_header(tag_type::AUDIO, body_size, 0)
        .unwrap();
    muxer.write_flv_tag_body(audio_data.clone()).unwrap();
    let tag_audio = muxer.writer.get_current_bytes();

    assert!(tag_audio.len() > 11);
    assert_eq!(tag_audio[0], tag_type::AUDIO);
}

/// Tests previous tag size serialization is correct.
#[tokio::test]
async fn test_flv_previous_tag_size() {
    let mut muxer = FlvMuxer::new();

    let data = BytesMut::from(&[0x01, 0x02, 0x03, 0x04][..]);
    let body_size = data.len() as u32;
    muxer
        .write_flv_tag_header(tag_type::VIDEO, body_size, 0)
        .unwrap();
    muxer.write_flv_tag_body(data).unwrap();
    let tag = muxer.writer.get_current_bytes();

    let mut muxer = FlvMuxer::new();
    muxer.write_previous_tag_size(tag.len() as u32).unwrap();
    let previous_tag_size = muxer.writer.get_current_bytes();
    assert_eq!(previous_tag_size.len(), 4);

    // Verify size is correct (big-endian)
    let size = ((previous_tag_size[0] as u32) << 24)
        | ((previous_tag_size[1] as u32) << 16)
        | ((previous_tag_size[2] as u32) << 8)
        | (previous_tag_size[3] as u32);
    assert_eq!(size, tag.len() as u32);
}

/// Tests FLV demuxer header parsing for a minimal FLV stream.
#[tokio::test]
async fn test_flv_demuxing() {
    let mut muxer = FlvMuxer::new();

    // Create FLV file structure
    let mut flv_data = BytesMut::new();

    // Write FLV header
    muxer.write_flv_header(true, true).unwrap();
    let header = muxer.writer.get_current_bytes();
    flv_data.extend_from_slice(&header);

    // Write previous tag size (0 for first tag after header)
    let mut muxer = FlvMuxer::new();
    muxer.write_previous_tag_size(0).unwrap();
    let prev_size = muxer.writer.get_current_bytes();
    flv_data.extend_from_slice(&prev_size);

    // Write video tag
    let mut muxer = FlvMuxer::new();
    let video_data = BytesMut::from(
        &[
            0x17, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x67, 0x42,
        ][..],
    );
    let body_size = video_data.len() as u32;
    muxer
        .write_flv_tag_header(tag_type::VIDEO, body_size, 1000)
        .unwrap();
    muxer.write_flv_tag_body(video_data.clone()).unwrap();
    let video_tag = muxer.writer.get_current_bytes();
    flv_data.extend_from_slice(&video_tag);

    // Write previous tag size
    let mut muxer = FlvMuxer::new();
    muxer
        .write_previous_tag_size(video_tag.len() as u32)
        .unwrap();
    let prev_size2 = muxer.writer.get_current_bytes();
    flv_data.extend_from_slice(&prev_size2);

    // Demux the data
    let mut demuxer = FlvDemuxer::new(flv_data);
    let result = demuxer.read_flv_header();

    // Verify demuxing succeeded
    assert!(result.is_ok());
}

/// Tests FLV timestamp encoding differences across tags.
#[tokio::test]
async fn test_flv_timestamp_handling() {
    let mut muxer = FlvMuxer::new();

    let data = BytesMut::from(&[0x01, 0x02, 0x03][..]);
    let body_size = data.len() as u32;

    // Write tags with different timestamps
    muxer
        .write_flv_tag_header(tag_type::VIDEO, body_size, 0)
        .unwrap();
    muxer.write_flv_tag_body(data.clone()).unwrap();
    let tag1 = muxer.writer.get_current_bytes();

    let mut muxer = FlvMuxer::new();
    muxer
        .write_flv_tag_header(tag_type::VIDEO, body_size, 1000)
        .unwrap();
    muxer.write_flv_tag_body(data.clone()).unwrap();
    let tag2 = muxer.writer.get_current_bytes();

    let mut muxer = FlvMuxer::new();
    muxer
        .write_flv_tag_header(tag_type::VIDEO, body_size, 2000)
        .unwrap();
    muxer.write_flv_tag_body(data.clone()).unwrap();
    let tag3 = muxer.writer.get_current_bytes();

    // Verify tags are different (timestamps encoded in tag header)
    assert_ne!(tag1, tag2);
    assert_ne!(tag2, tag3);
}

/// Tests basic FLV muxing round-trip structure creation.
#[tokio::test]
async fn test_flv_round_trip() {
    let mut muxer = FlvMuxer::new();

    // Create original data
    let original_video = BytesMut::from(&[0x17, 0x01, 0x67, 0x42, 0x00, 0x1e][..]);
    let original_audio = BytesMut::from(&[0xaf, 0x00, 0x12, 0x10][..]);

    // Mux
    let mut flv_data = BytesMut::new();
    muxer.write_flv_header(true, true).unwrap();
    let header = muxer.writer.get_current_bytes();
    flv_data.extend_from_slice(&header);

    let mut muxer = FlvMuxer::new();
    muxer.write_previous_tag_size(0).unwrap();
    let prev_size = muxer.writer.get_current_bytes();
    flv_data.extend_from_slice(&prev_size);

    let mut muxer = FlvMuxer::new();
    let body_size = original_video.len() as u32;
    muxer
        .write_flv_tag_header(tag_type::VIDEO, body_size, 0)
        .unwrap();
    muxer.write_flv_tag_body(original_video.clone()).unwrap();
    let video_tag = muxer.writer.get_current_bytes();
    flv_data.extend_from_slice(&video_tag);

    let mut muxer = FlvMuxer::new();
    muxer
        .write_previous_tag_size(video_tag.len() as u32)
        .unwrap();
    let prev_size2 = muxer.writer.get_current_bytes();
    flv_data.extend_from_slice(&prev_size2);

    let mut muxer = FlvMuxer::new();
    let body_size = original_audio.len() as u32;
    muxer
        .write_flv_tag_header(tag_type::AUDIO, body_size, 0)
        .unwrap();
    muxer.write_flv_tag_body(original_audio.clone()).unwrap();
    let audio_tag = muxer.writer.get_current_bytes();
    flv_data.extend_from_slice(&audio_tag);

    // Verify FLV structure is valid
    assert!(flv_data.len() > 20); // Header + tags
    assert_eq!(&flv_data[0..3], b"FLV");
}
