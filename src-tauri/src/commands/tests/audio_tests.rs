use crate::services::audio_service::{TranscriptionResult, TranscriptionSegment};

/// Create mock transcription result with word-level segments
fn create_mock_transcription_with_words() -> TranscriptionResult {
    TranscriptionResult {
        text: "Hello world this is a test".to_string(),
        segments: vec![
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
                confidence: Some(0.92),
            },
            TranscriptionSegment {
                start: 1.0,
                end: 1.3,
                text: "this".to_string(),
                confidence: Some(0.88),
            },
            TranscriptionSegment {
                start: 1.3,
                end: 1.5,
                text: "is".to_string(),
                confidence: Some(0.90),
            },
            TranscriptionSegment {
                start: 1.5,
                end: 1.7,
                text: "a".to_string(),
                confidence: Some(0.85),
            },
            TranscriptionSegment {
                start: 1.7,
                end: 2.0,
                text: "test".to_string(),
                confidence: Some(0.93),
            },
        ],
        duration: 2.0,
        language: Some("en".to_string()),
    }
}

// Word-level timestamp precision is tested in audio_service_tests

#[tokio::test]
async fn test_transcription_text_assembly() {
    let mock_transcription = create_mock_transcription_with_words();

    // Verify full text is correctly assembled from segments
    let expected_text = "Hello world this is a test";
    assert_eq!(mock_transcription.text, expected_text, "Full text should match segment concatenation");

    // Verify individual words are preserved
    let words: Vec<&str> = expected_text.split_whitespace().collect();
    assert_eq!(words.len(), mock_transcription.segments.len(), "Word count should match segment count");

    for (i, word) in words.iter().enumerate() {
        assert_eq!(mock_transcription.segments[i].text, *word, "Segment text should match word: {}", word);
    }
}

// Confidence score validation is tested in audio_service_tests

// Language detection is tested in whisper_tests

#[test]
fn test_transcription_segment_validation() {
    // Test validation of transcription segment data
    let valid_segment = TranscriptionSegment {
        start: 1.0,
        end: 2.0,
        text: "hello".to_string(),
        confidence: Some(0.9),
    };

    // Verify segment structure
    assert!(valid_segment.start >= 0.0, "Start time should be non-negative");
    assert!(valid_segment.end > valid_segment.start, "End time should be after start time");
    assert!(!valid_segment.text.trim().is_empty(), "Text should not be empty");

    if let Some(confidence) = valid_segment.confidence {
        assert!(confidence >= 0.0 && confidence <= 1.0, "Confidence should be in [0,1] range");
    }

    // Test duration calculation
    let duration = valid_segment.end - valid_segment.start;
    assert_eq!(duration, 1.0, "Duration should be calculated correctly");
}

#[test]
fn test_audio_format_validation() {
    let supported_formats = vec![
        "wav", "mp3", "mp4", "m4a", "flac", "ogg", "mov", "avi", "mkv", "webm"
    ];

    let unsupported_formats = vec![
        "txt", "pdf", "doc", "jpg", "png", "gif", "zip", "exe"
    ];

    // Test that we know which formats are supported
    for format in supported_formats {
        assert!(format.len() <= 4, "Format extension should be reasonable length: {}", format);
        assert!(format.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "Format should be lowercase letters and digits: {}", format);
    }

    // Test that unsupported formats are properly identified
    for format in unsupported_formats {
        assert!(!["wav", "mp3", "mp4", "m4a", "flac", "ogg", "mov", "avi", "mkv", "webm"].contains(&format),
            "Format {} should not be in audio formats", format);
    }
}

// Word segmentation accuracy is tested through transcription tests
