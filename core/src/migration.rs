//! Read a consistent SQLite snapshot, never raw-copy live journals or WAL files.
use rusqlite::{
    backup::{Backup, StepResult},
    Connection, OpenFlags,
};
use std::{
    io,
    path::Path,
    time::{Duration, Instant},
};

fn sql_error(error: rusqlite::Error) -> io::Error {
    io::Error::other(error)
}

fn target_exists(target: &Path) -> io::Result<bool> {
    match std::fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err(io::Error::other(
            "Migration destination is not a regular file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn reject_sidecars(target: &Path) -> io::Result<()> {
    for suffix in ["-journal", "-wal", "-shm"] {
        let mut name = target.as_os_str().to_owned();
        name.push(suffix);
        match std::fs::symlink_metadata(&name) {
            Ok(_) => {
                return Err(io::Error::other(
                    "Migration destination has existing SQLite sidecar data; nothing was replaced",
                ))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// Publish one validated SQLite snapshot in the destination directory. Existing
/// target files and sidecars are never replaced. The original remains untouched.
/// A competing successful migration wins; this invocation then returns false.
/// Errors must stop startup rather than opening a fresh or partial destination.
pub fn migrate_legacy_data(legacy: &Path, target: &Path) -> io::Result<bool> {
    if target_exists(target)? {
        return Ok(false);
    }
    // Only an actually absent legacy path means there is nothing to migrate.
    // Access failures, directories and symlinks must not start an empty kitchen.
    // In particular, do not silently treat a dangling legacy symlink as absent.
    match std::fs::symlink_metadata(legacy) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => {
            return Err(io::Error::other(
                "Legacy database is not a regular file; original path was preserved",
            ))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    }
    reject_sidecars(target)?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temporary = tempfile::Builder::new()
        .prefix(".hungerstrike-migration-")
        .tempfile_in(parent)?;
    let source =
        Connection::open_with_flags(legacy, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(sql_error)?;
    source
        .busy_timeout(Duration::from_millis(250))
        .map_err(sql_error)?;
    // Pin one read snapshot, including committed WAL pages, while an older
    // process may continue writing. Uncommitted transactions are not copied.
    source.execute_batch("BEGIN").map_err(sql_error)?;
    source
        .query_row("PRAGMA schema_version", [], |row| row.get::<_, i64>(0))
        .map_err(sql_error)?;
    let mut destination =
        Connection::open_with_flags(temporary.path(), OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(sql_error)?;
    {
        let backup = Backup::new(&source, &mut destination).map_err(sql_error)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "SQLite migration snapshot timed out; original data was preserved",
                ));
            }
            match backup.step(128).map_err(sql_error)? {
                StepResult::Done => break,
                StepResult::More => {}
                StepResult::Busy | StepResult::Locked => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                _ => return Err(io::Error::other("Unexpected SQLite backup state")),
            }
        }
    }
    let mode: String = destination
        .query_row("PRAGMA journal_mode=DELETE", [], |row| row.get(0))
        .map_err(sql_error)?;
    if !mode.eq_ignore_ascii_case("delete") {
        return Err(io::Error::other(
            "Migration could not make a standalone SQLite database",
        ));
    }
    let integrity: String = destination
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(sql_error)?;
    if integrity != "ok" {
        return Err(io::Error::other(
            "Migration snapshot failed SQLite integrity check",
        ));
    }
    destination.close().map_err(|(_, error)| sql_error(error))?;
    drop(source);
    temporary.as_file().sync_all()?;
    // Recheck immediately before atomic publication. Unlike exists()+copy,
    // persist_noclobber cannot replace a target created by another launch.
    if target_exists(target)? {
        return Ok(false);
    }
    reject_sidecars(target)?;
    match temporary.persist_noclobber(target) {
        Ok(file) => {
            file.sync_all()?;
            #[cfg(unix)]
            std::fs::File::open(parent)?.sync_all()?;
            Ok(true)
        }
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
            if target_exists(target)? {
                Ok(false)
            } else {
                Err(error.error)
            }
        }
        Err(error) => Err(error.error),
    }
}
