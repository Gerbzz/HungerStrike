use hungerstrike_core::migrate_legacy_data;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}

#[test]
fn missing_source_does_not_create_target_or_its_parent() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("new-parent/inventory.sqlite");
    assert!(!migrate_legacy_data(&dir.path().join("missing.sqlite"), &target).unwrap());
    assert!(!target.parent().unwrap().exists());
}

#[test]
fn nonregular_source_stops_migration_without_creating_target() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.sqlite");
    let new = dir.path().join("new-parent/inventory.sqlite");
    std::fs::create_dir(&old).unwrap();
    assert!(migrate_legacy_data(&old, &new).is_err());
    assert!(!new.parent().unwrap().exists());
}

#[cfg(unix)]
#[test]
fn dangling_legacy_symlink_is_not_mistaken_for_absent_data() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.sqlite");
    let new = dir.path().join("new-parent/inventory.sqlite");
    std::os::unix::fs::symlink(dir.path().join("missing.sqlite"), &old).unwrap();
    assert!(migrate_legacy_data(&old, &new).is_err());
    assert!(!new.parent().unwrap().exists());
    assert!(std::fs::symlink_metadata(&old)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn locked_legacy_database_fails_without_publishing_or_modifying_source() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.sqlite");
    let new = dir.path().join("new.sqlite");
    let writer = Connection::open(&old).unwrap();
    writer.execute_batch("CREATE TABLE records(value); INSERT INTO records VALUES (42); BEGIN EXCLUSIVE; UPDATE records SET value=99;").unwrap();
    assert!(migrate_legacy_data(&old, &new).is_err());
    assert!(!new.exists());
    writer.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        writer
            .query_row("SELECT value FROM records", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        42
    );
}

#[cfg(unix)]
#[test]
fn dangling_destination_symlink_is_rejected_without_creating_its_referent() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.sqlite");
    let new = dir.path().join("new.sqlite");
    let referent = dir.path().join("untouched.sqlite");
    Connection::open(&old)
        .unwrap()
        .execute_batch("CREATE TABLE records(value)")
        .unwrap();
    std::os::unix::fs::symlink(&referent, &new).unwrap();
    assert!(migrate_legacy_data(&old, &new).is_err());
    assert!(!referent.exists());
    assert!(std::fs::symlink_metadata(&new)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn migration_rejects_corrupt_source_without_publishing_a_database() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.sqlite");
    let new = dir.path().join("new.sqlite");
    std::fs::write(&old, b"not a SQLite database").unwrap();
    assert!(migrate_legacy_data(&old, &new).is_err());
    assert!(!new.exists());
    assert_eq!(std::fs::read(old).unwrap(), b"not a SQLite database");
}

#[test]
fn migration_never_overwrites_or_consumes_destination_sidecars() {
    for suffix in ["-wal", "-shm", "-journal"] {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("old.sqlite");
        let new = dir.path().join("new.sqlite");
        Connection::open(&old)
            .unwrap()
            .execute_batch("CREATE TABLE records(value); INSERT INTO records VALUES (42);")
            .unwrap();
        let existing = sidecar(&new, suffix);
        std::fs::write(&existing, b"preserve destination state").unwrap();
        assert!(migrate_legacy_data(&old, &new).is_err());
        assert!(!new.exists());
        assert_eq!(
            std::fs::read(existing).unwrap(),
            b"preserve destination state"
        );
    }
}

#[test]
fn migration_snapshots_live_wal_without_copying_shared_memory() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.sqlite");
    let new = dir.path().join("new.sqlite");
    let writer = Connection::open(&old).unwrap();
    writer.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value); INSERT INTO records VALUES (42); BEGIN IMMEDIATE; INSERT INTO records VALUES (99);").unwrap();
    assert!(sidecar(&old, "-wal").exists());
    assert!(migrate_legacy_data(&old, &new).unwrap());
    let copy = Connection::open(&new).unwrap();
    let values: Vec<i64> = copy
        .prepare("SELECT value FROM records")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(values, vec![42]);
    for suffix in ["-wal", "-shm", "-journal"] {
        assert!(!sidecar(&new, suffix).exists());
    }
    writer.execute_batch("COMMIT").unwrap();
    assert_eq!(
        copy.query_row("SELECT count(*) FROM records", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        writer
            .query_row("SELECT count(*) FROM records", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn simultaneous_launches_publish_one_complete_database_without_clobbering() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.sqlite");
    let new = dir.path().join("new.sqlite");
    Connection::open(&old)
        .unwrap()
        .execute_batch("CREATE TABLE records(value); INSERT INTO records VALUES (42);")
        .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let joins: Vec<_> = (0..8)
        .map(|_| {
            let (old, new, barrier) = (old.clone(), new.clone(), barrier.clone());
            std::thread::spawn(move || {
                barrier.wait();
                migrate_legacy_data(&old, &new)
            })
        })
        .collect();
    let results: Vec<bool> = joins
        .into_iter()
        .map(|join| join.join().unwrap().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|&&copied| copied).count(), 1);
    let copy = Connection::open(&new).unwrap();
    assert_eq!(
        copy.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        copy.query_row("SELECT value FROM records", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        42
    );
    copy.execute("UPDATE records SET value=77", []).unwrap();
    assert!(!migrate_legacy_data(&old, &new).unwrap());
    assert_eq!(
        copy.query_row("SELECT value FROM records", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        77
    );
}
