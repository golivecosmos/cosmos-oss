//! Regression tests for the cancel-during-indexing invariants introduced in
//! the HN-readiness correctness pass. These cover:
//!
//! - Atomic replace: `replace_text_chunks_for_file` and its transcript
//!   variant must delete + insert as one transaction so a crash or cancel
//!   between the two can never leave a file with zero indexed chunks.
//! - Purge coverage: `purge_indexed_data_for_file` must remove every
//!   artifact table a worker might have written for a given file (images,
//!   text chunks + vec rows, transcriptions) and must be idempotent on
//!   repeated invocation.

use crate::services::sqlite_service::SqliteVectorService;
use crate::services::vector_service::{ImageVectorBulkData, TextChunkBulkData};

fn make_chunk(file_path: &str, idx: i64, text: &str) -> TextChunkBulkData {
    TextChunkBulkData {
        id: format!("{}:chunk:{}", file_path, idx),
        file_path: file_path.to_string(),
        parent_file_path: None,
        file_name: file_path.rsplit('/').next().unwrap_or(file_path).to_string(),
        mime_type: Some("text/plain".to_string()),
        chunk_index: idx,
        chunk_text: text.to_string(),
        char_start: 0,
        char_end: text.len() as i64,
        token_estimate: (text.len() as i64) / 4,
        metadata: serde_json::json!({}),
        embedding: vec![0.01_f32; 768],
        drive_uuid: None,
    }
}

fn make_transcript_chunk(file_path: &str, idx: i64, text: &str) -> TextChunkBulkData {
    let mut chunk = make_chunk(file_path, idx, text);
    // Use a distinct ID namespace so transcript rows don't collide with
    // document rows at the same `(file_path, chunk_index)` via
    // `INSERT OR REPLACE` on primary key.
    chunk.id = format!("{}:transcript:{}", file_path, idx);
    chunk.metadata = serde_json::json!({ "source_type": "transcript_chunk" });
    chunk
}

fn count_text_chunks(service: &SqliteVectorService, file_path: &str) -> i64 {
    let db = service.get_database_service();
    let conn = db.get_connection();
    let db = conn.lock().unwrap();
    db.query_row(
        "SELECT COUNT(*) FROM text_chunks WHERE file_path = ?1",
        rusqlite::params![file_path],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

fn count_vec_rows_for_file(service: &SqliteVectorService, file_path: &str) -> i64 {
    let db = service.get_database_service();
    let conn = db.get_connection();
    let db = conn.lock().unwrap();
    db.query_row(
        "SELECT COUNT(*) FROM vec_text_chunks
         WHERE rowid IN (SELECT rowid FROM text_chunks WHERE file_path = ?1)",
        rusqlite::params![file_path],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

#[test]
fn replace_text_chunks_is_atomic() {
    let service = SqliteVectorService::new_in_memory().expect("in-memory service");
    let file_path = "/test/doc.txt";

    service
        .replace_text_chunks_for_file(
            file_path,
            vec![
                make_chunk(file_path, 0, "hello world"),
                make_chunk(file_path, 1, "second chunk"),
            ],
        )
        .expect("first replace");

    assert_eq!(count_text_chunks(&service, file_path), 2);
    assert_eq!(count_vec_rows_for_file(&service, file_path), 2);

    // Re-replace with new content. Old rows must be gone, new rows present.
    service
        .replace_text_chunks_for_file(
            file_path,
            vec![make_chunk(file_path, 0, "updated content")],
        )
        .expect("second replace");

    assert_eq!(count_text_chunks(&service, file_path), 1);
    assert_eq!(count_vec_rows_for_file(&service, file_path), 1);

    let db = service.get_database_service();
    let conn = db.get_connection();
    let db = conn.lock().unwrap();
    let text: String = db
        .query_row(
            "SELECT chunk_text FROM text_chunks WHERE file_path = ?1",
            rusqlite::params![file_path],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(text, "updated content");
}

#[test]
fn replace_transcript_chunks_preserves_document_chunks_for_same_path() {
    let service = SqliteVectorService::new_in_memory().expect("in-memory service");
    let media_path = "/test/interview.mp4";

    // Seed both a document-style chunk and a transcript chunk at the same path.
    service
        .replace_text_chunks_for_file(
            media_path,
            vec![make_chunk(media_path, 0, "video metadata note")],
        )
        .expect("seed document chunk");

    service
        .replace_transcript_chunks_for_file(
            media_path,
            vec![
                make_transcript_chunk(media_path, 0, "hello speaker one"),
                make_transcript_chunk(media_path, 1, "thanks for joining"),
            ],
        )
        .expect("seed transcript chunks");

    assert_eq!(count_text_chunks(&service, media_path), 3);

    // Re-run transcript replacement with fewer chunks. Document chunks must
    // survive because replace_transcript_chunks filters by source_type.
    service
        .replace_transcript_chunks_for_file(
            media_path,
            vec![make_transcript_chunk(media_path, 0, "fresh transcript")],
        )
        .expect("replace transcript");

    let db = service.get_database_service();
    let conn = db.get_connection();
    let db = conn.lock().unwrap();
    let doc_count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM text_chunks WHERE file_path = ?1
             AND (metadata IS NULL OR json_extract(metadata, '$.source_type') IS NULL)",
            rusqlite::params![media_path],
            |row| row.get(0),
        )
        .unwrap();
    let transcript_count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM text_chunks WHERE file_path = ?1
             AND json_extract(metadata, '$.source_type') = 'transcript_chunk'",
            rusqlite::params![media_path],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(doc_count, 1, "document chunk must survive transcript replace");
    assert_eq!(transcript_count, 1, "new transcript chunk must be present");
}

#[test]
fn replace_with_invalid_embedding_leaves_prior_data_intact() {
    // Regression: the pre-fix code deleted old chunks before attempting the
    // bulk insert. An invalid-embedding error during insert would leave the
    // file with zero indexed chunks. The atomic replace aborts the whole
    // transaction, preserving the prior state.
    let service = SqliteVectorService::new_in_memory().expect("in-memory service");
    let file_path = "/test/doc.md";

    service
        .replace_text_chunks_for_file(
            file_path,
            vec![make_chunk(file_path, 0, "original content")],
        )
        .expect("seed");

    // Build a bad chunk with wrong embedding dimensions.
    let mut bad = make_chunk(file_path, 0, "replacement attempt");
    bad.embedding = vec![0.0_f32; 128]; // wrong dimension on purpose

    let result =
        service.replace_text_chunks_for_file(file_path, vec![bad, make_chunk(file_path, 1, "ok")]);
    assert!(result.is_err(), "replace must fail on invalid embedding");

    // Prior chunk must still be present and searchable.
    assert_eq!(count_text_chunks(&service, file_path), 1);
    let db = service.get_database_service();
    let conn = db.get_connection();
    let db = conn.lock().unwrap();
    let text: String = db
        .query_row(
            "SELECT chunk_text FROM text_chunks WHERE file_path = ?1",
            rusqlite::params![file_path],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        text, "original content",
        "original chunk must survive failed replace"
    );
}

fn count_images(service: &SqliteVectorService, file_path: &str) -> i64 {
    let db = service.get_database_service();
    let conn = db.get_connection();
    let db = conn.lock().unwrap();
    db.query_row(
        "SELECT COUNT(*) FROM images WHERE file_path = ?1",
        rusqlite::params![file_path],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

fn count_vec_image_rows_for_file(service: &SqliteVectorService, file_path: &str) -> i64 {
    let db = service.get_database_service();
    let conn = db.get_connection();
    let db = conn.lock().unwrap();
    db.query_row(
        "SELECT COUNT(*) FROM vec_images
         WHERE rowid IN (SELECT rowid FROM images WHERE file_path = ?1)",
        rusqlite::params![file_path],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

fn make_image_row(file_path: &str, unique_id: &str) -> ImageVectorBulkData {
    ImageVectorBulkData {
        id: unique_id.to_string(),
        file_path: file_path.to_string(),
        parent_file_path: None,
        file_name: file_path.rsplit('/').next().unwrap_or(file_path).to_string(),
        mime_type: Some("image/jpeg".to_string()),
        embedding: vec![0.02_f32; 768],
        metadata: serde_json::json!({}),
        drive_uuid: None,
    }
}

/// Regression for the re-index failure mode surfaced by the cancel flow:
/// purge used to clear `images` but not `vec_images`. The orphaned vec rows
/// remained, and when a re-index hit a recycled rowid, sqlite-vec's vec0
/// tables (which do not honor ON CONFLICT REPLACE) returned a UNIQUE
/// constraint error. This test proves both halves of the fix: purge now
/// clears `vec_images`, and the insert path is tolerant of any stragglers.
#[test]
fn reindex_after_purge_does_not_collide_on_recycled_rowid() {
    let service = SqliteVectorService::new_in_memory().expect("in-memory service");
    let file_path = "/test/movie.mov";

    // First pass: seed one image, purge everything for the path.
    service
        .store_image_vectors_bulk(vec![make_image_row(file_path, "movie:frame:0001")])
        .expect("initial store");
    assert_eq!(count_images(&service, file_path), 1);
    assert_eq!(count_vec_image_rows_for_file(&service, file_path), 1);

    let purged = service
        .purge_indexed_data_for_file(file_path)
        .expect("purge");
    assert!(purged >= 1);
    assert_eq!(count_images(&service, file_path), 0);
    assert_eq!(count_vec_image_rows_for_file(&service, file_path), 0);

    // Second pass: re-index with a different id. Must succeed — even if
    // SQLite happens to recycle the rowid from the purged row, the insert
    // now clears any stale vec row first.
    service
        .store_image_vectors_bulk(vec![make_image_row(file_path, "movie:frame:0001:v2")])
        .expect("reindex after purge must not UNIQUE-constraint");
    assert_eq!(count_images(&service, file_path), 1);
    assert_eq!(count_vec_image_rows_for_file(&service, file_path), 1);
}

#[test]
fn purge_indexed_data_covers_text_and_vec_rows() {
    let service = SqliteVectorService::new_in_memory().expect("in-memory service");
    let file_path = "/test/page.md";

    service
        .replace_text_chunks_for_file(
            file_path,
            vec![
                make_chunk(file_path, 0, "chunk a"),
                make_chunk(file_path, 1, "chunk b"),
            ],
        )
        .expect("seed");

    let purged = service
        .purge_indexed_data_for_file(file_path)
        .expect("purge");
    assert!(purged >= 2, "should report at least the text_chunks removed");

    assert_eq!(count_text_chunks(&service, file_path), 0);
    assert_eq!(count_vec_rows_for_file(&service, file_path), 0);
}

#[test]
fn purge_indexed_data_is_idempotent() {
    let service = SqliteVectorService::new_in_memory().expect("in-memory service");
    let file_path = "/test/nothing-here.txt";

    // No prior data at this path — purge must succeed and return 0.
    let first = service
        .purge_indexed_data_for_file(file_path)
        .expect("purge empty");
    assert_eq!(first, 0);

    // Repeated purge is still safe.
    let second = service
        .purge_indexed_data_for_file(file_path)
        .expect("purge empty again");
    assert_eq!(second, 0);
}
