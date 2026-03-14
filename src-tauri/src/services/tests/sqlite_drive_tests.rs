use crate::services::sqlite_service::SqliteVectorService;
use std::sync::Arc;
use uuid::Uuid;

/// Create a test SQLite service with in-memory database for speed
fn create_test_sqlite_service() -> Arc<SqliteVectorService> {
    Arc::new(
        SqliteVectorService::new_in_memory()
            .expect("Failed to create in-memory test SQLite service"),
    )
}

#[test]
fn test_add_drive_success() {
    let service = create_test_sqlite_service();
    let drive_uuid = Uuid::new_v4().to_string();

    let result = service.add_drive(&drive_uuid, "Test Drive", "/test/mount/path", true);

    assert!(result.is_ok(), "Adding drive should succeed");

    // Verify the drive was added
    let drive = service
        .get_drive_by_uuid(&drive_uuid)
        .expect("Failed to get drive")
        .expect("Drive should exist");

    assert_eq!(drive["uuid"].as_str().unwrap(), drive_uuid);
    assert_eq!(drive["name"].as_str().unwrap(), "Test Drive");
    assert_eq!(drive["last_mount_path"].as_str(), Some("/test/mount/path"));
    assert_eq!(drive["is_removable"].as_bool().unwrap(), true);
}

#[test]
fn test_add_duplicate_drive() {
    let service = create_test_sqlite_service();
    let drive_uuid = Uuid::new_v4().to_string();

    // Add drive first time
    let result1 = service.add_drive(&drive_uuid, "First Drive", "/path1", true);
    assert!(result1.is_ok(), "First add should succeed");

    // Try to add same UUID again
    let result2 = service.add_drive(&drive_uuid, "Second Drive", "/path2", false);
    assert!(result2.is_err(), "Duplicate drive UUID should fail");
}

#[test]
fn test_get_drive_by_uuid_existing() {
    let service = create_test_sqlite_service();
    let drive_uuid = Uuid::new_v4().to_string();

    // Add a test drive
    service
        .add_drive(&drive_uuid, "Test Drive", "/test/path", true)
        .expect("Failed to add drive");

    // Get the drive back
    let result = service.get_drive_by_uuid(&drive_uuid);
    assert!(result.is_ok(), "get_drive_by_uuid should succeed");

    let drive = result.unwrap().expect("Drive should exist");
    assert_eq!(drive["uuid"].as_str().unwrap(), drive_uuid);
    assert_eq!(drive["name"].as_str().unwrap(), "Test Drive");
}

#[test]
fn test_get_drive_by_uuid_nonexistent() {
    let service = create_test_sqlite_service();
    let nonexistent_uuid = Uuid::new_v4().to_string();

    let result = service.get_drive_by_uuid(&nonexistent_uuid);
    assert!(
        result.is_ok(),
        "get_drive_by_uuid should succeed even for non-existent drive"
    );
    assert!(
        result.unwrap().is_none(),
        "Non-existent drive should return None"
    );
}

#[test]
fn test_update_drive_metadata_success() {
    let service = create_test_sqlite_service();
    let drive_uuid = Uuid::new_v4().to_string();

    // Add a test drive
    service
        .add_drive(&drive_uuid, "Original Drive", "/original/path", true)
        .expect("Failed to add drive");

    // Update metadata
    let result =
        service.update_drive_metadata(&drive_uuid, Some("Custom Name"), Some("Physical Location"));
    assert!(result.is_ok(), "update_drive_metadata should succeed");

    // Verify the metadata was updated
    let drive = service
        .get_drive_by_uuid(&drive_uuid)
        .expect("Failed to get drive")
        .expect("Drive should exist");

    assert_eq!(drive["custom_name"].as_str(), Some("Custom Name"));
    assert_eq!(
        drive["physical_location"].as_str(),
        Some("Physical Location")
    );
    // Original fields should remain unchanged
    assert_eq!(drive["name"].as_str().unwrap(), "Original Drive");
}

#[test]
fn test_update_drive_metadata_partial() {
    let service = create_test_sqlite_service();
    let drive_uuid = Uuid::new_v4().to_string();

    // Add a test drive
    service
        .add_drive(&drive_uuid, "Test Drive", "/test/path", true)
        .expect("Failed to add drive");

    // Update only custom name (None for location will overwrite any existing location)
    let result1 = service.update_drive_metadata(&drive_uuid, Some("New Name"), None);
    assert!(result1.is_ok(), "Partial update (name only) should succeed");

    // Verify the first update
    let drive1 = service
        .get_drive_by_uuid(&drive_uuid)
        .expect("Failed to get drive")
        .expect("Drive should exist");

    assert_eq!(drive1["custom_name"].as_str(), Some("New Name"));
    assert_eq!(drive1["physical_location"].as_str(), None);

    // Update only physical location (None for name will overwrite the custom name)
    let result2 = service.update_drive_metadata(&drive_uuid, None, Some("New Location"));
    assert!(
        result2.is_ok(),
        "Partial update (location only) should succeed"
    );

    // Verify the second update (custom_name will be None now)
    let drive2 = service
        .get_drive_by_uuid(&drive_uuid)
        .expect("Failed to get drive")
        .expect("Drive should exist");

    assert_eq!(drive2["custom_name"].as_str(), None);
    assert_eq!(drive2["physical_location"].as_str(), Some("New Location"));
}

#[test]
fn test_update_drive_metadata_nonexistent() {
    let service = create_test_sqlite_service();
    let nonexistent_uuid = Uuid::new_v4().to_string();

    let result =
        service.update_drive_metadata(&nonexistent_uuid, Some("Test Name"), Some("Test Location"));
    assert!(
        result.is_ok(),
        "update_drive_metadata succeeds even for non-existent drive (current implementation)"
    );
}

#[test]
fn test_update_drive_status_success() {
    let service = create_test_sqlite_service();
    let drive_uuid = Uuid::new_v4().to_string();

    // Add a test drive
    service
        .add_drive(&drive_uuid, "Test Drive", "/original/path", true)
        .expect("Failed to add drive");

    // Update status to connected with new mount path
    let result = service.update_drive_status(&drive_uuid, "connected", Some("/new/mount/path"));
    assert!(result.is_ok(), "update_drive_status should succeed");

    // Verify the status was updated
    let drive = service
        .get_drive_by_uuid(&drive_uuid)
        .expect("Failed to get drive")
        .expect("Drive should exist");

    assert_eq!(drive["status"].as_str().unwrap(), "connected");
    assert_eq!(drive["last_mount_path"].as_str(), Some("/new/mount/path"));
}

#[test]
fn test_update_drive_status_disconnected() {
    let service = create_test_sqlite_service();
    let drive_uuid = Uuid::new_v4().to_string();

    // Add a test drive
    service
        .add_drive(&drive_uuid, "Test Drive", "/mount/path", true)
        .expect("Failed to add drive");

    // Update status to disconnected (no mount path)
    let result = service.update_drive_status(&drive_uuid, "disconnected", None);
    assert!(result.is_ok(), "update_drive_status should succeed");

    // Verify the status was updated
    let drive = service
        .get_drive_by_uuid(&drive_uuid)
        .expect("Failed to get drive")
        .expect("Drive should exist");

    assert_eq!(drive["status"].as_str().unwrap(), "disconnected");
    // Mount path should be None when disconnected (as per current implementation)
    assert_eq!(drive["last_mount_path"].as_str(), None);
}

#[test]
fn test_get_all_drives() {
    let service = create_test_sqlite_service();

    // Add multiple test drives
    let drive1_uuid = Uuid::new_v4().to_string();
    let drive2_uuid = Uuid::new_v4().to_string();
    let drive3_uuid = Uuid::new_v4().to_string();

    service
        .add_drive(&drive1_uuid, "Drive 1", "/path1", true)
        .expect("Failed to add drive 1");
    service
        .add_drive(&drive2_uuid, "Drive 2", "/path2", false)
        .expect("Failed to add drive 2");
    service
        .add_drive(&drive3_uuid, "Drive 3", "/path3", true)
        .expect("Failed to add drive 3");

    // Update metadata for some drives
    service
        .update_drive_metadata(&drive1_uuid, Some("Custom Drive 1"), Some("Location 1"))
        .expect("Failed to update drive 1");
    service
        .update_drive_status(&drive2_uuid, "disconnected", None)
        .expect("Failed to update drive 2 status");

    // Get all drives
    let result = service.get_all_drives();
    assert!(result.is_ok(), "get_all_drives should succeed");

    let drives = result.unwrap();
    assert_eq!(drives.len(), 3, "Should return 3 drives");

    // Verify all drives are included
    let uuids: Vec<&str> = drives.iter().map(|d| d["uuid"].as_str().unwrap()).collect();
    assert!(uuids.contains(&drive1_uuid.as_str()));
    assert!(uuids.contains(&drive2_uuid.as_str()));
    assert!(uuids.contains(&drive3_uuid.as_str()));
}

#[test]
fn test_get_all_drives_empty() {
    let service = create_test_sqlite_service();

    let result = service.get_all_drives();
    assert!(
        result.is_ok(),
        "get_all_drives should succeed even when empty"
    );

    let drives = result.unwrap();
    assert_eq!(
        drives.len(),
        0,
        "Should return empty list when no drives exist"
    );
}

#[test]
fn test_drive_file_count_calculation() {
    let service = create_test_sqlite_service();
    let drive_uuid = Uuid::new_v4().to_string();

    // Add a test drive
    service
        .add_drive(&drive_uuid, "Test Drive", "/test/path", true)
        .expect("Failed to add drive");

    // Initially, file count should be 0
    let drives = service.get_all_drives().unwrap();
    let drive = drives.iter().find(|d| d["uuid"] == drive_uuid).unwrap();
    assert_eq!(drive["indexed_files_count"].as_i64().unwrap(), 0);

    // Note: Testing actual file count would require setting up image vectors
    // which is beyond the scope of this specific test
}

#[test]
fn test_drive_metadata_edge_cases() {
    let service = create_test_sqlite_service();
    let drive_uuid = Uuid::new_v4().to_string();

    // Add drive with edge case values
    let result = service.add_drive(
        &drive_uuid,
        "", // Empty name
        "", // Empty mount path
        false,
    );
    assert!(
        result.is_ok(),
        "Adding drive with empty strings should succeed"
    );

    // Update with edge case metadata
    let result = service.update_drive_metadata(
        &drive_uuid,
        Some(""),                                               // Empty custom name
        Some("Very Long Location String ".repeat(50).as_str()), // Very long location
    );
    assert!(
        result.is_ok(),
        "Updating with edge case metadata should succeed"
    );

    // Verify the values were stored
    let drive = service
        .get_drive_by_uuid(&drive_uuid)
        .expect("Failed to get drive")
        .expect("Drive should exist");

    assert_eq!(drive["custom_name"].as_str(), Some(""));
    assert!(drive["physical_location"].as_str().unwrap().len() > 100);
}

#[test]
fn test_concurrent_drive_operations() {
    let service = create_test_sqlite_service();
    let service_arc = Arc::new(service);

    // Create multiple drives concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let service = service_arc.clone();
            std::thread::spawn(move || {
                let uuid = Uuid::new_v4().to_string();
                service.add_drive(
                    &uuid,
                    &format!("Drive {}", i),
                    &format!("/path/{}", i),
                    i % 2 == 0,
                )
            })
        })
        .collect();

    // Wait for all operations to complete
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All operations should succeed
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.is_ok(),
            "Concurrent drive creation {} should succeed",
            i
        );
    }

    // Verify all drives were created
    let drives = service_arc.get_all_drives().unwrap();
    assert_eq!(drives.len(), 10, "All 10 drives should be created");
}

#[test]
fn test_invalid_uuid_handling() {
    let service = create_test_sqlite_service();

    // Test operations with invalid UUIDs
    let invalid_uuids = vec![
        "",
        "not-a-uuid",
        "123",
        "550e8400-e29b-41d4-a716", // Too short
    ];

    for invalid_uuid in invalid_uuids {
        // These operations may succeed or fail depending on implementation
        // but they should not crash
        let _ = service.add_drive(invalid_uuid, "Test", "/test", true);
        let _ = service.get_drive_by_uuid(invalid_uuid);
        let _ = service.update_drive_metadata(invalid_uuid, Some("Test"), None);
        let _ = service.update_drive_status(invalid_uuid, "connected", Some("/test"));
    }
}
