use anyhow::Context;
use rusqlite::Connection;
use std::path::Path;

/// Opens a connection to the shared database file. Each repository gets its
/// own connection (see `build_application`), so WAL mode lets readers on one
/// connection proceed while another holds the write lock, and `busy_timeout`
/// makes SQLite retry internally for up to 5s instead of immediately
/// returning `SQLITE_BUSY` ("database is locked") when two connections
/// briefly contend for the write lock.
pub fn open(path: &Path) -> anyhow::Result<Connection> {
    let conn =
        Connection::open(path).with_context(|| format!("failed to open database at {path:?}"))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .context("failed to enable WAL journal mode")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .context("failed to set busy timeout")?;
    Ok(conn)
}

/// A freshly migrated database file, private to one test and removed when
/// dropped. Connections are opened exactly as in production (one per
/// repository, WAL, busy timeout), so tests exercise the same multi-connection
/// setup the daemon runs with.
#[cfg(test)]
pub struct TestDatabase {
    dir: tempfile::TempDir,
}

#[cfg(test)]
impl TestDatabase {
    pub fn new() -> Self {
        let database = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        crate::infrastructure::shared::sqlite_migrations::apply(&mut database.connection())
            .unwrap();
        database
    }

    pub fn connection(&self) -> Connection {
        open(&self.dir.path().join("yarrtube.sqlite3")).unwrap()
    }

    pub fn shared_connection(&self) -> std::sync::Arc<std::sync::Mutex<Connection>> {
        std::sync::Arc::new(std::sync::Mutex::new(self.connection()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_share_one_migrated_database_across_its_connections() {
        let database = TestDatabase::new();
        let writer = database.connection();
        let reader = database.connection();

        writer
            .execute(
                "INSERT INTO channels (id, name, youtube_channel_id, quality, video_limit, path, created_at)
                 VALUES ('@somechannel', 'Some Channel', 'UC123', 'high', 10, 'creators/somechannel', '2024-01-01T00:00:00+00:00')",
                [],
            )
            .unwrap();

        let name: String = reader
            .query_row("SELECT name FROM channels", [], |row| row.get(0))
            .unwrap();
        assert_eq!(name, "Some Channel");
    }
}
