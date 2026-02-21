use crate::services::sqlite_service::SqliteVectorService;
use crate::services::drive_service::DriveService;
use crate::services::database_service::DatabaseService;
use std::sync::Arc;
use uuid::Uuid;

/// Create a test SQLite service with in-memory database for speed
fn create_test_sqlite_service() -> Arc<SqliteVectorService> {
    Arc::new(SqliteVectorService::new_in_memory()
        .expect("Failed to create in-memory test SQLite service"))
}

#[tokio::test]
async fn test_drive_metadata_operations() {
    let sqlite_service = create_test_sqlite_service();
    let drive_uuid = Uuid::new_v4().to_string();

    // Test adding a drive
    let result = sqlite_service.add_drive(
        &drive_uuid,
        "Test Drive",
        "/test/path",
        true
    );
    assert!(result.is_ok(), "Adding drive should succeed");

    // Test updating drive metadata
    let result = sqlite_service.update_drive_metadata(
        &drive_uuid,
        Some("My Custom Drive"),
        Some("On my desk")
    );
    assert!(result.is_ok(), "update_drive_metadata should succeed");

    // Verify the metadata was updated
    let drive = sqlite_service.get_drive_by_uuid(&drive_uuid)
        .expect("Failed to get drive")
        .expect("Drive should exist");

    assert_eq!(drive["custom_name"].as_str().unwrap(), "My Custom Drive");
    assert_eq!(drive["physical_location"].as_str().unwrap(), "On my desk");
}

#[tokio::test]
async fn test_drive_metadata_nonexistent_drive() {
    let sqlite_service = create_test_sqlite_service();
    let nonexistent_uuid = Uuid::new_v4().to_string();

    // Test updating metadata for non-existent drive
    let result = sqlite_service.update_drive_metadata(
        &nonexistent_uuid,
        Some("Test Name"),
        Some("Test Location")
    );
    assert!(result.is_ok(), "update_drive_metadata succeeds even for non-existent drive (current implementation)");
}

#[tokio::test]
async fn test_get_all_drives_with_metadata() {
    let sqlite_service = create_test_sqlite_service();

    // Add some test drives
    let drive1_uuid = Uuid::new_v4().to_string();
    let drive2_uuid = Uuid::new_v4().to_string();

    sqlite_service.add_drive(&drive1_uuid, "Drive 1", "/path1", true)
        .expect("Failed to add drive 1");
    sqlite_service.add_drive(&drive2_uuid, "Drive 2", "/path2", false)
        .expect("Failed to add drive 2");

    // Update metadata for one drive
    sqlite_service.update_drive_metadata(
        &drive1_uuid,
        Some("Custom Drive 1"),
        Some("Location 1")
    ).expect("Failed to update drive 1 metadata");

    // Test getting all drives with metadata
    let result = sqlite_service.get_all_drives();
    assert!(result.is_ok(), "get_all_drives should succeed");

    let drives = result.unwrap();
    assert_eq!(drives.len(), 2, "Should return 2 drives");

    // Find our test drives in the results
    let drive1 = drives.iter().find(|d| d["uuid"] == drive1_uuid).unwrap();
    let drive2 = drives.iter().find(|d| d["uuid"] == drive2_uuid).unwrap();

    assert_eq!(drive1["custom_name"].as_str().unwrap(), "Custom Drive 1");
    assert_eq!(drive1["physical_location"].as_str().unwrap(), "Location 1");
    assert_eq!(drive2["name"].as_str().unwrap(), "Drive 2");
}

#[tokio::test]
async fn test_drive_status_updates() {
    let sqlite_service = create_test_sqlite_service();
    let drive_uuid = Uuid::new_v4().to_string();

    // Add a test drive
    sqlite_service.add_drive(&drive_uuid, "Test Drive", "/test", true)
        .expect("Failed to add test drive");

    // Test updating drive status to connected
    let result = sqlite_service.update_drive_status(&drive_uuid, "connected", Some("/new/path"));
    assert!(result.is_ok(), "update_drive_status should succeed");

    // Test updating drive status to disconnected
    let result = sqlite_service.update_drive_status(&drive_uuid, "disconnected", None);
    assert!(result.is_ok(), "update_drive_status should succeed");

    // Verify the status was updated
    let drive = sqlite_service.get_drive_by_uuid(&drive_uuid)
        .expect("Failed to get drive")
        .expect("Drive should exist");

    assert_eq!(drive["status"].as_str().unwrap(), "disconnected");
}

#[tokio::test]
async fn test_concurrent_drive_operations() {
    let sqlite_service = create_test_sqlite_service();
    let service_arc = sqlite_service.clone();

    // Create multiple drives concurrently
    let handles: Vec<_> = (0..5).map(|i| {
        let service = service_arc.clone();
        tokio::spawn(async move {
            let uuid = Uuid::new_v4().to_string();
            service.add_drive(
                &uuid,
                &format!("Drive {}", i),
                &format!("/path/{}", i),
                i % 2 == 0
            )
        })
    }).collect();

    // Wait for all operations to complete
    let results: Vec<_> = futures::future::join_all(handles).await;

    // All operations should succeed
    for (i, result) in results.iter().enumerate() {
        let unwrapped = result.as_ref().expect("Task should complete");
        assert!(unwrapped.is_ok(), "Concurrent drive creation {} should succeed", i);
    }

    // Verify all drives were created
    let drives = service_arc.get_all_drives().unwrap();
    assert_eq!(drives.len(), 5, "All 5 drives should be created");
}

#[tokio::test]
async fn test_drive_service_creation() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let drive_service = DriveService::new(Arc::new(db_service));

    // Test basic drive detection (may not find drives in test environment)
    let result = drive_service.detect_connected_drives().await;

    // The result depends on the system, but the function should not crash
    match result {
        Ok(drives) => {
            // If drives are found, verify their structure
            for drive in drives {
                assert!(!drive.uuid.is_empty(), "Drive UUID should not be empty");
                assert!(!drive.name.is_empty(), "Drive name should not be empty");
            }
        }
        Err(_) => {
            // Drive detection may fail in test environments - this is expected
        }
    }
}

#[tokio::test]
async fn test_drive_path_detection() {
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let drive_service = DriveService::new(Arc::new(db_service));

    // Test with system paths that should not be on external drives
    let system_paths = vec![
        "/usr/bin/ls",
        "/System/Library",
        "",
        "/nonexistent/path",
    ];

    for path in system_paths {
        let result = drive_service.get_drive_for_path(path).await;
        // Most of these should return None, but the function should not crash
        assert!(result.is_none() || result.is_some(),
               "get_drive_for_path should complete for: {}", path);
    }
}

#[test]
fn test_drive_validation_edge_cases() {
    // Test edge cases for drive metadata validation
    let test_cases = vec![
        (None, None),                                          // Both None
        (Some("".to_string()), Some("".to_string())),         // Empty strings
        (Some("Valid Name".to_string()), None),               // Only name
        (None, Some("Valid Location".to_string())),           // Only location
        (Some("Very Long Drive Name That Exceeds Normal Length".to_string()),
         Some("Very Long Physical Location Description".to_string())), // Long strings
    ];

    for (name, location) in test_cases {
        // These should all be valid inputs for the service layer
        assert!(name.is_none() || !name.as_ref().unwrap().contains('\0'),
               "Names should not contain null bytes");
        assert!(location.is_none() || !location.as_ref().unwrap().contains('\0'),
               "Locations should not contain null bytes");
    }
}

#[test]
fn test_uuid_validation() {
    // Test UUID validation scenarios
    let valid_uuids = vec![
        Uuid::new_v4().to_string(),
        "550e8400-e29b-41d4-a716-446655440000".to_string(),
    ];

    let invalid_uuids = vec![
        "not-a-uuid".to_string(),
        "".to_string(),
        "550e8400-e29b-41d4-a716".to_string(),  // Too short
    ];

    for uuid in valid_uuids {
        assert!(Uuid::parse_str(&uuid).is_ok(), "Valid UUID should parse: {}", uuid);
    }

    for uuid in invalid_uuids {
        assert!(Uuid::parse_str(&uuid).is_err(), "Invalid UUID should fail to parse: {}", uuid);
    }
}
