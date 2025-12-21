use rusqlite::{params, Connection, Result};
use std::path::PathBuf;

use once_cell::sync::Lazy;

// We use a global connection for simplicity in this single-threaded UI app (mostly).
// In a real app we might pass this around or use a pool.
static DB_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("opennav");
    std::fs::create_dir_all(&path).ok();
    path.push("data.db");
    path
});

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn new() -> Result<Self> {
        let conn = Connection::open(&*DB_PATH)?;
        let mut store = Store { conn };
        store.init()?;
        Ok(store)
    }

    fn init(&mut self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS browser_stats (
                id TEXT PRIMARY KEY,
                usage_count INTEGER DEFAULT 0,
                is_pinned BOOLEAN DEFAULT 0,
                last_used INTEGER DEFAULT 0
            )",
            [],
        )?;
        Ok(())
    }

    pub fn increment_usage(&self, id: &str) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        self.conn.execute(
            "INSERT INTO browser_stats (id, usage_count, is_pinned, last_used)
             VALUES (?1, 1, 0, ?2)
             ON CONFLICT(id) DO UPDATE SET
                usage_count = usage_count + 1,
                last_used = ?2",
            params![id, now],
        )?;
        Ok(())
    }

    pub fn toggle_pin(&self, id: &str) -> Result<bool> {
        // First ensure it exists
        self.conn.execute(
            "INSERT OR IGNORE INTO browser_stats (id, usage_count, is_pinned, last_used)
             VALUES (?1, 0, 0, 0)",
            params![id],
        )?;
        
        let is_pinned: bool = self.conn.query_row(
            "SELECT is_pinned FROM browser_stats WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        
        let new_state = !is_pinned;
        
        self.conn.execute(
            "UPDATE browser_stats SET is_pinned = ?1 WHERE id = ?2",
            params![new_state, id],
        )?;
        
        Ok(new_state)
    }
    
    pub fn get_stats(&self) -> Result<Vec<(String, i64, bool, i64)>> {
        let mut stmt = self.conn.prepare("SELECT id, usage_count, is_pinned, last_used FROM browser_stats")?;
        let rows = stmt.query_map([], |row| {
             Ok((
                 row.get(0)?,
                 row.get(1)?,
                 row.get(2)?,
                 row.get(3)?,
             ))
        })?;
        
        let mut stats = Vec::new();
        for row in rows {
            stats.push(row?);
        }
        Ok(stats)
    }

    pub fn clear_stats(&self) -> Result<()> {
        self.conn.execute(
            "UPDATE browser_stats SET usage_count = 0, last_used = 0",
            [],
        )?;
        Ok(())
    }
}
