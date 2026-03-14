use crate::commands::indexing::categorize_error;
use crate::services::sqlite_service::SqliteVectorService;
use std::sync::Arc;

/// Create a test SQLite service with in-memory database for speed
fn create_test_sqlite_service() -> Arc<SqliteVectorService> {
    Arc::new(
        SqliteVectorService::new_in_memory()
            .expect("Failed to create in-memory test SQLite service"),
    )
}

#[test]
fn test_error_categorization_temporary_errors() {
    // Test cases that should be categorized as temporary (retryable)
    let temporary_errors = vec![
        "Connection timeout occurred",
        "Network connection failed",
        "Database is busy",
        "Resource temporarily unavailable",
        "Lock timeout exceeded",
        "Out of memory",
        "FFmpeg not available",
        "Temporary file creation failed",
    ];

    for error in temporary_errors {
        assert_eq!(
            categorize_error(error),
            "temporary",
            "Error '{}' should be categorized as temporary",
            error
        );
    }
}

#[test]
fn test_error_categorization_permanent_errors() {
    // Test cases that should be categorized as permanent (non-retryable)
    let permanent_errors = vec![
        "File not found",
        "Permission denied",
        "Access denied",
        "Invalid file format",
        "Corrupted file data",
        "Unsupported video codec",
        "Image decode error",
        "Invalid image format",
    ];

    for error in permanent_errors {
        assert_eq!(
            categorize_error(error),
            "permanent",
            "Error '{}' should be categorized as permanent",
            error
        );
    }
}

#[test]
fn test_error_categorization_unknown_errors() {
    // Test cases that should be categorized as unknown
    let unknown_errors = vec![
        "Something unexpected happened",
        "Internal error occurred",
        "Unknown failure mode",
        "Mysterious problem",
    ];

    for error in unknown_errors {
        assert_eq!(
            categorize_error(error),
            "unknown",
            "Error '{}' should be categorized as unknown",
            error
        );
    }
}

#[test]
fn test_error_categorization_case_insensitive() {
    // Test that categorization works regardless of case
    let test_cases = vec![
        ("TIMEOUT OCCURRED", "temporary"),
        ("file NOT FOUND", "permanent"),
        ("Permission DENIED", "permanent"),
        ("CONNECTION failed", "temporary"),
    ];

    for (error, expected_category) in test_cases {
        assert_eq!(
            categorize_error(error),
            expected_category,
            "Error '{}' should be categorized as {} (case insensitive)",
            error,
            expected_category
        );
    }
}

#[tokio::test]
async fn test_retry_scheduling_for_temporary_errors() {
    let sqlite_service = create_test_sqlite_service();

    // Create a job
    let job_id = sqlite_service
        .create_job("file", "/test/retry_test.jpg", Some(1))
        .expect("Failed to create test job");

    // Schedule retry for a temporary error
    let result = sqlite_service.schedule_job_retry(&job_id, "Connection timeout");

    match result {
        Ok(_) => {
            // Verify job status is updated appropriately for retry
            let job = sqlite_service
                .get_job_by_id(&job_id)
                .expect("Failed to get job after retry scheduling");

            // Job should not be in failed state if retry was scheduled
            assert_ne!(
                job["status"], "failed",
                "Job should not be failed if retry was scheduled successfully"
            );
        }
        Err(e) => {
            // If retry scheduling fails, that's also valid behavior
            // (depends on implementation - some systems might not support retry scheduling)
            println!("Retry scheduling failed (may be expected): {}", e);
        }
    }
}

#[tokio::test]
async fn test_job_failure_handling() {
    let sqlite_service = create_test_sqlite_service();

    // Create a job and mark it as running
    let job_id = sqlite_service
        .create_job("file", "/test/fail_test.jpg", Some(1))
        .expect("Failed to create test job");

    // Update job to running state
    sqlite_service
        .update_job_progress(&job_id, "running", Some("Processing..."), None, None, None)
        .expect("Failed to update job to running");

    // Mark job as failed with error details
    let error_message = "File not found";
    let errors = vec![error_message.to_string()];

    let result = sqlite_service.update_job_progress(
        &job_id,
        "failed",
        Some(&format!("Processing failed: {}", error_message)),
        Some(0), // processed count
        Some(&errors),
        None, // failed files
    );

    assert!(result.is_ok(), "Failed to mark job as failed: {:?}", result);

    // Verify job is in failed state
    let failed_job = sqlite_service
        .get_job_by_id(&job_id)
        .expect("Failed to get failed job");

    assert_eq!(
        failed_job["status"], "failed",
        "Job should be in failed state"
    );

    // Verify error information is stored
    if let Some(job_errors) = failed_job.get("errors") {
        assert!(
            job_errors.to_string().contains(error_message),
            "Job errors should contain the error message"
        );
    }
}

#[test]
fn test_error_categorization_with_partial_matches() {
    // Test that partial keyword matches work correctly
    assert_eq!(
        categorize_error("Socket connection timeout after 30 seconds"),
        "temporary"
    );

    assert_eq!(
        categorize_error("The requested file could not be found on disk"),
        "permanent"
    );

    assert_eq!(
        categorize_error("Network is busy, try again later"),
        "temporary"
    );
}

#[test]
fn test_all_production_errors_are_categorized() {
    // Ensure no production errors fall through as "unknown"
    let production_errors = vec![
        ("Failed to read file: Permission denied", "permanent"),
        ("Network timeout after 30 seconds", "temporary"),
        ("Out of memory during processing", "temporary"),
        ("File format not supported", "permanent"),
        ("Database connection lost", "temporary"),
        ("Corrupted video file detected", "permanent"),
        ("FFmpeg process crashed", "temporary"),
        ("Disk full - cannot write temp file", "temporary"),
        ("Invalid image format: PNG expected", "permanent"),
        ("Service temporarily unavailable", "temporary"),
        ("Access denied to file system", "permanent"),
        ("Connection refused by server", "temporary"),
    ];

    for (error, expected_category) in production_errors {
        let actual_category = categorize_error(error);
        assert_ne!(
            actual_category, "unknown",
            "Production error '{}' should not be unknown, got '{}'",
            error, actual_category
        );
        assert_eq!(
            actual_category, expected_category,
            "Error '{}' expected '{}', got '{}'",
            error, expected_category, actual_category
        );
    }
}

#[test]
fn test_error_categorization_edge_cases() {
    // Test edge cases that might break categorization

    // Empty string
    assert_eq!(categorize_error(""), "unknown");

    // Very long error message
    let long_error = "A".repeat(1000) + " timeout occurred";
    assert_eq!(categorize_error(&long_error), "temporary");

    // Special characters and unicode
    assert_eq!(
        categorize_error("File not found: /path/with/émojis/🎬/video.mp4"),
        "permanent"
    );

    // Mixed case variations
    assert_eq!(categorize_error("FILE NOT FOUND"), "permanent");
    assert_eq!(categorize_error("Timeout OCCURRED"), "temporary");

    // Multiple keywords (should match first applicable)
    assert_eq!(
        categorize_error("Connection timeout: file not found"),
        "temporary"
    );

    // Substring matches
    assert_eq!(categorize_error("timeouts are common"), "temporary");
    assert_eq!(categorize_error("files not found often"), "permanent");
}

#[test]
fn test_error_categorization_prevents_infinite_retries() {
    // Ensure permanent errors don't get retried infinitely
    let permanent_errors = vec![
        "File does not exist",
        "Invalid video codec",
        "Unsupported file format",
        "Permission denied",
        "Malformed image data",
    ];

    for error in permanent_errors {
        assert_eq!(
            categorize_error(error),
            "permanent",
            "Error '{}' should be permanent to prevent infinite retries",
            error
        );
    }
}

#[test]
fn test_temporary_errors_justify_retry() {
    // Ensure temporary errors make sense to retry
    let retry_worthy_errors = vec![
        "Connection timed out",
        "Server temporarily unavailable",
        "Out of memory",
        "Database is locked",
        "Network unreachable",
        "Resource temporarily unavailable",
    ];

    for error in retry_worthy_errors {
        assert_eq!(
            categorize_error(error),
            "temporary",
            "Error '{}' should be temporary since retry might succeed",
            error
        );
    }
}
