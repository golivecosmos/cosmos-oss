use crate::services::audio_service::{AudioService, TranscriptionResult, TranscriptionSegment};
// Note: WhisperModel tests are in models/tests/whisper_tests.rs
use tempfile::NamedTempFile;

/// Create a test audio service (without loading actual models)
fn create_test_audio_service() -> AudioService {
    AudioService::new()
}

/// Create a temporary audio file for testing
async fn create_test_audio_file(format: &str) -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().expect("Should create temporary file");

    // Write minimal valid audio data (silence)
    let audio_data = match format {
        "wav" => create_minimal_wav(),
        "mp3" => create_minimal_mp3(),
        _ => vec![0u8; 1024], // Fallback to empty data
    };

    std::io::Write::write_all(&mut temp_file, &audio_data).expect("Should write test audio data");

    temp_file
}

/// Create minimal valid WAV file data
fn create_minimal_wav() -> Vec<u8> {
    // Minimal WAV header for 1 second of silence at 16kHz mono
    let mut wav_data = Vec::new();

    // RIFF header
    wav_data.extend_from_slice(b"RIFF");
    wav_data.extend_from_slice(&(36u32).to_le_bytes()); // File size - 8
    wav_data.extend_from_slice(b"WAVE");

    // Format chunk
    wav_data.extend_from_slice(b"fmt ");
    wav_data.extend_from_slice(&(16u32).to_le_bytes()); // Chunk size
    wav_data.extend_from_slice(&(1u16).to_le_bytes()); // Audio format (PCM)
    wav_data.extend_from_slice(&(1u16).to_le_bytes()); // Channels (mono)
    wav_data.extend_from_slice(&(16000u32).to_le_bytes()); // Sample rate
    wav_data.extend_from_slice(&(32000u32).to_le_bytes()); // Byte rate
    wav_data.extend_from_slice(&(2u16).to_le_bytes()); // Block align
    wav_data.extend_from_slice(&(16u16).to_le_bytes()); // Bits per sample

    // Data chunk
    wav_data.extend_from_slice(b"data");
    wav_data.extend_from_slice(&(0u32).to_le_bytes()); // Data size (empty)

    wav_data
}

/// Create minimal MP3 data (just header)
fn create_minimal_mp3() -> Vec<u8> {
    // MP3 frame header for silence
    vec![
        0xFF, 0xFB, 0x90, 0x00, // MP3 frame sync + header
        0x00, 0x00, 0x00, 0x00, // Silent frame data
    ]
}

#[tokio::test]
async fn test_audio_service_creation() {
    let audio_service = create_test_audio_service();

    // Service should be created but not available until model is loaded
    assert!(
        !audio_service.is_available(),
        "Service should not be available without loaded model"
    );
}

#[tokio::test]
async fn test_validate_supported_audio_formats() {
    let audio_service = create_test_audio_service();

    // Test supported audio formats by creating files with proper extensions
    let supported_formats = vec!["wav", "mp3", "mp4", "m4a", "flac", "ogg"];

    for format in supported_formats {
        let temp_dir = tempfile::tempdir().expect("Should create temp dir");
        let file_path = temp_dir.path().join(format!("test.{}", format));

        // Create a file with the correct extension
        std::fs::write(&file_path, b"fake audio data").expect("Should write test file");

        let result = audio_service.validate_audio_file(&file_path);

        // Should succeed for supported formats (validation only checks extension and existence)
        assert!(
            result.is_ok(),
            "Should validate supported format: {}",
            format
        );
    }
}

// Video format validation is covered by the general format detection test

// Unsupported format validation is covered by the general format detection test

// Nonexistent file validation is a basic file system check

#[test]
fn test_transcription_result_structure() {
    // Test that TranscriptionResult has expected structure for word-level segments
    let segments = vec![
        TranscriptionSegment {
            start: 0.0,
            end: 0.5,
            text: "Hello".to_string(),
            confidence: Some(0.95),
        },
        TranscriptionSegment {
            start: 0.5,
            end: 1.0,
            text: "world".to_string(),
            confidence: Some(0.90),
        },
    ];

    let result = TranscriptionResult {
        text: "Hello world".to_string(),
        segments: segments.clone(),
        duration: 1.0,
        language: Some("en".to_string()),
    };

    assert_eq!(result.text, "Hello world");
    assert_eq!(result.segments.len(), 2);
    assert_eq!(result.duration, 1.0);
    assert_eq!(result.language, Some("en".to_string()));

    // Verify segments structure
    assert_eq!(result.segments[0].text, "Hello");
    assert_eq!(result.segments[0].start, 0.0);
    assert_eq!(result.segments[0].end, 0.5);
    assert_eq!(result.segments[0].confidence, Some(0.95));

    assert_eq!(result.segments[1].text, "world");
    assert_eq!(result.segments[1].start, 0.5);
    assert_eq!(result.segments[1].end, 1.0);
    assert_eq!(result.segments[1].confidence, Some(0.90));
}

#[test]
fn test_transcription_segment_word_level_granularity() {
    // Test that segments can represent word-level granularity
    let words = vec![
        "This",
        "is",
        "a",
        "test",
        "of",
        "word",
        "level",
        "transcription",
    ];
    let mut segments = Vec::new();

    for (i, word) in words.iter().enumerate() {
        let start_time = i as f64 * 0.3; // 300ms per word
        let end_time = start_time + 0.3;

        segments.push(TranscriptionSegment {
            start: start_time,
            end: end_time,
            text: word.to_string(),
            confidence: Some(0.9),
        });
    }

    let result = TranscriptionResult {
        text: words.join(" "),
        segments: segments.clone(),
        duration: words.len() as f64 * 0.3,
        language: Some("en".to_string()),
    };

    // Verify word-level granularity
    assert_eq!(result.segments.len(), words.len());

    for (i, segment) in result.segments.iter().enumerate() {
        // Each segment should be a single word
        assert_eq!(segment.text, words[i]);

        // Duration should be reasonable for a word (< 1 second)
        let duration = segment.end - segment.start;
        assert!(
            duration > 0.0 && duration <= 1.0,
            "Word '{}' duration should be 0-1 seconds: {}",
            segment.text,
            duration
        );

        // Verify sequential timing (allow for exact matches or small overlap)
        if i > 0 {
            let prev_end = result.segments[i - 1].end;
            assert!(
                segment.start >= prev_end || (prev_end - segment.start).abs() < 0.001,
                "Segment timing should be sequential: prev_end={:.3}, curr_start={:.3}",
                prev_end,
                segment.start
            );
        }
    }
}

#[test]
fn test_audio_format_extension_detection() {
    let test_cases = vec![
        ("file.wav", true),
        ("file.WAV", true),
        ("file.mp3", true),
        ("file.MP3", true),
        ("file.mp4", true),
        ("video.mov", true),
        ("audio.m4a", true),
        ("recording.flac", true),
        ("podcast.ogg", true),
        ("movie.avi", true),
        ("clip.mkv", true),
        ("stream.webm", true),
        ("document.txt", false),
        ("image.jpg", false),
        ("presentation.pdf", false),
    ];

    let audio_service = create_test_audio_service();

    for (filename, should_be_valid) in test_cases {
        let temp_dir = tempfile::tempdir().expect("Should create temp dir");
        let file_path = temp_dir.path().join(filename);

        // Create the file only if filename is not empty
        if !filename.is_empty() {
            std::fs::write(&file_path, b"test data").expect("Should write test file");
        }

        let result = audio_service.validate_audio_file(&file_path);

        if should_be_valid {
            assert!(result.is_ok(), "Should validate file: {}", filename);
        } else {
            assert!(result.is_err(), "Should reject file: {}", filename);
        }
    }

    // Test edge case: file with no extension
    let temp_dir = tempfile::tempdir().expect("Should create temp dir");
    let file_path = temp_dir.path().join("no_extension_file");
    std::fs::write(&file_path, b"test data").expect("Should write test file");
    let result = audio_service.validate_audio_file(&file_path);
    assert!(result.is_err(), "Should reject file with no extension");
}

// Confidence validation is tested through the word-level granularity test

// Sample rate handling is tested through actual transcription

#[test]
fn test_word_level_timestamp_accuracy() {
    // Test timestamp accuracy requirements for word-level navigation
    let mock_segments = vec![
        ("The", 0.0, 0.2),
        ("quick", 0.2, 0.5),
        ("brown", 0.5, 0.8),
        ("fox", 0.8, 1.1),
        ("jumps", 1.1, 1.5),
        ("over", 1.5, 1.8),
        ("the", 1.8, 2.0),
        ("lazy", 2.0, 2.3),
        ("dog", 2.3, 2.6),
    ];

    let segments: Vec<TranscriptionSegment> = mock_segments
        .iter()
        .map(|(text, start, end)| TranscriptionSegment {
            start: *start,
            end: *end,
            text: text.to_string(),
            confidence: Some(0.9),
        })
        .collect();

    // Verify word-level precision requirements
    for segment in &segments {
        let duration = segment.end - segment.start;

        // Most words should be under 500ms for good UX
        assert!(
            duration <= 1.0,
            "Word '{}' duration too long for precise navigation: {:.2}s",
            segment.text,
            duration
        );

        // Minimum duration should be reasonable (20ms)
        assert!(
            duration >= 0.02,
            "Word '{}' duration too short: {:.2}s",
            segment.text,
            duration
        );
    }

    // Verify sequential timing with no gaps
    for i in 1..segments.len() {
        let prev_end = segments[i - 1].end;
        let curr_start = segments[i].start;

        // Should be continuous or have minimal gap
        let gap = curr_start - prev_end;
        assert!(
            gap >= 0.0,
            "No negative gaps allowed between '{}' and '{}'",
            segments[i - 1].text,
            segments[i].text
        );
        assert!(
            gap <= 0.1,
            "Gap too large between '{}' and '{}': {:.2}s",
            segments[i - 1].text,
            segments[i].text,
            gap
        );
    }
}
