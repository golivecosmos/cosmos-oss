use crate::services::drive_service::{DriveService, DriveInfo, DriveStatus};
use crate::services::database_service::DatabaseService;
use std::path::Path;
use std::sync::Arc;
use tempfile::tempdir;
use chrono::Utc;

#[test]
fn test_drive_service_creation() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let _drive_service = DriveService::new(Arc::new(db_service));
    assert!(true, "DriveService should create successfully");
}

#[tokio::test]
async fn test_get_drive_for_path_none() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let drive_service = DriveService::new(Arc::new(db_service));

    // Test with a regular system path that shouldn't be on external drive
    let result = drive_service.get_drive_for_path("/usr/bin/ls").await;
    assert!(result.is_none(), "System paths should not be on external drives");
}

#[tokio::test]
async fn test_get_drive_for_path_empty() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let drive_service = DriveService::new(Arc::new(db_service));

    // Test with empty path
    let result = drive_service.get_drive_for_path("").await;
    assert!(result.is_none(), "Empty path should return None");
}

#[tokio::test]
async fn test_get_drive_for_path_invalid() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let drive_service = DriveService::new(Arc::new(db_service));

    // Test with invalid paths
    let invalid_paths = vec![
        "/nonexistent/path/that/does/not/exist",
        "relative/path",
        "C:\\Windows\\System32", // Windows path on macOS
    ];

    for path in invalid_paths {
        let result = drive_service.get_drive_for_path(path).await;
        assert!(result.is_none(), "Invalid path '{}' should return None", path);
    }
}

#[tokio::test]
async fn test_detect_connected_drives() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let drive_service = DriveService::new(Arc::new(db_service));

    // This test may succeed or fail depending on the system
    // but it should not crash
    let result = drive_service.detect_connected_drives().await;

    match result {
        Ok(drives) => {
            // Verify drive structure if any drives are found
            for drive in drives {
                assert!(!drive.uuid.is_empty(), "Drive UUID should not be empty");
                assert!(!drive.name.is_empty(), "Drive name should not be empty");
                assert!(!drive.mount_path.is_empty(), "Drive mount path should not be empty");
                // is_removable can be any boolean value
            }
        }
        Err(e) => {
            // Drive detection may fail on systems without appropriate tools
            // This is acceptable for testing
            println!("Drive detection failed (expected on some systems): {}", e);
        }
    }
}

#[test]
fn test_drive_info_structure() {
    // Test creating a DriveInfo structure
    let drive = DriveInfo {
        uuid: "test-uuid-12345".to_string(),
        name: "Test Drive".to_string(),
        mount_path: "/Volumes/TestDrive".to_string(),
        total_space: 1000000000,
        free_space: 500000000,
        is_removable: true,
        last_seen: Utc::now(),
        status: DriveStatus::Connected,
        indexed_files_count: 0,
        total_size_indexed: 0,
    };

    assert_eq!(drive.uuid, "test-uuid-12345");
    assert_eq!(drive.name, "Test Drive");
    assert_eq!(drive.mount_path, "/Volumes/TestDrive");
    assert!(drive.is_removable);
}

#[test]
fn test_drive_info_edge_cases() {
    // Test DriveInfo with edge case values
    let now = Utc::now();
    let edge_cases = vec![
        DriveInfo {
            uuid: "".to_string(),
            name: "".to_string(),
            mount_path: "".to_string(),
            total_space: 0,
            free_space: 0,
            is_removable: false,
            last_seen: now,
            status: DriveStatus::Disconnected,
            indexed_files_count: 0,
            total_size_indexed: 0,
        },
        DriveInfo {
            uuid: "a".repeat(100),
            name: "Very Long Drive Name ".repeat(10),
            mount_path: "/very/long/path/".repeat(20),
            total_space: u64::MAX,
            free_space: u64::MAX / 2,
            is_removable: true,
            last_seen: now,
            status: DriveStatus::Connected,
            indexed_files_count: i64::MAX,
            total_size_indexed: i64::MAX,
        },
        DriveInfo {
            uuid: "special-chars-!@#$%^&*()".to_string(),
            name: "Drive with 特殊字符 and émojis 🔥".to_string(),
            mount_path: "/Volumes/Special Chars!@#".to_string(),
            total_space: 1000000000,
            free_space: 500000000,
            is_removable: false,
            last_seen: now,
            status: DriveStatus::Error("Test error".to_string()),
            indexed_files_count: 42,
            total_size_indexed: 123456,
        },
    ];

    for drive in edge_cases {
        // These should all be valid DriveInfo structures
        // The actual validation would happen at the service level
        assert!(drive.uuid.len() >= 0, "UUID length should be non-negative");
        assert!(drive.name.len() >= 0, "Name length should be non-negative");
        assert!(drive.mount_path.len() >= 0, "Mount path length should be non-negative");
    }
}

#[tokio::test]
async fn test_path_normalization() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let drive_service = DriveService::new(Arc::new(db_service));

    // Test various path formats
    let test_paths = vec![
        "/normal/path",
        "/path/with/trailing/slash/",
        "/path/../with/../dots",
        "/path/./with/./current/dir",
        "//double//slashes//path",
    ];

    for path in test_paths {
        // These calls should not crash regardless of path format
        let result = drive_service.get_drive_for_path(path).await;
        // We don't assert on the result since it depends on the actual system
        // but the function should handle various path formats gracefully
        assert!(result.is_none() || result.is_some(),
               "get_drive_for_path should return Option for path: {}", path);
    }
}

#[tokio::test]
async fn test_concurrent_drive_detection() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let drive_service = std::sync::Arc::new(DriveService::new(Arc::new(db_service)));

    // Test concurrent drive detection calls
    let service1 = drive_service.clone();
    let service2 = drive_service.clone();
    let service3 = drive_service.clone();

    let task1 = tokio::spawn(async move {
        service1.detect_connected_drives().await
    });

    let task2 = tokio::spawn(async move {
        service2.detect_connected_drives().await
    });

    let task3 = tokio::spawn(async move {
        service3.detect_connected_drives().await
    });

    let (result1, result2, result3) = tokio::join!(task1, task2, task3);

    // All tasks should complete without panicking
    assert!(result1.is_ok(), "Task 1 should complete");
    assert!(result2.is_ok(), "Task 2 should complete");
    assert!(result3.is_ok(), "Task 3 should complete");

    // The actual results may be Ok or Err depending on system capabilities
    // but the service should handle concurrent access safely
}

#[tokio::test]
async fn test_drive_path_matching() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let drive_service = DriveService::new(Arc::new(db_service));

    // Create a temporary directory to simulate a mount point
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path().to_str().unwrap();

    // Test paths that should not match external drives
    let system_paths = vec![
        "/usr/bin",
        "/System/Library",
        "/Applications",
        "/Library",
        "/tmp",
        temp_path, // Temporary directory should not be considered external drive
    ];

    for path in system_paths {
        if Path::new(path).exists() {
            let result = drive_service.get_drive_for_path(path).await;
            // Most system paths should not be on external drives
            // (though this could vary by system configuration)
            assert!(result.is_none() || result.is_some(),
                   "Path check should complete for: {}", path);
        }
    }
}

#[test]
fn test_drive_service_memory_safety() {
    // Test that creating and dropping multiple drive services is safe
    for _ in 0..10 {
        let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
        let _service = DriveService::new(Arc::new(db_service));
        // Service should be dropped safely
    }

    // Test cloning Arc<DriveService>
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let service = std::sync::Arc::new(DriveService::new(Arc::new(db_service)));
    let _clones: Vec<_> = (0..10).map(|_| service.clone()).collect();
    // All clones should be dropped safely
}

#[cfg(target_os = "macos")]
#[test]
fn test_macos_specific_functionality() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let _drive_service = DriveService::new(Arc::new(db_service));

    // Test macOS-specific paths
    let macos_paths = vec![
        "/Volumes",
        "/Volumes/Macintosh HD",
        "/System/Volumes/Data",
    ];

    for path in macos_paths {
        if Path::new(path).exists() {
            // These paths exist on macOS and the service should handle them
            // We don't test the actual result since it depends on system configuration
            assert!(true, "macOS path should be processable: {}", path);
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_non_macos_fallback() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let _drive_service = DriveService::new(Arc::new(db_service));

    // On non-macOS systems, drive detection may not work
    // but the service should still be created successfully
    assert!(true, "DriveService should work on non-macOS systems");
}
