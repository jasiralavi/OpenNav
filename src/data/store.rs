use rusqlite::{params, Connection, Result};
use std::path::PathBuf;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEngine {
    pub keyword: String,
    pub name: String,
    pub url: String, // Template with %s or {}
    pub icon_path: Option<String>,
}

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
        
        // Settings table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT
            )",
            [],
        )?;
        
        // Search Engines table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS search_engines (
                keyword TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                url TEXT NOT NULL,
                icon_path TEXT
            )",
            [],
        )?;
        
        // Seed defaults if empty
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM search_engines",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        
        if count == 0 {
            let defaults = vec![
                SearchEngine { keyword: "g".to_string(), name: "Google".to_string(), url: "https://www.google.com/search?q={}".to_string(), icon_path: None },
                SearchEngine { keyword: "d".to_string(), name: "DuckDuckGo".to_string(), url: "https://duckduckgo.com/?q={}".to_string(), icon_path: None },
                SearchEngine { keyword: "b".to_string(), name: "Bing".to_string(), url: "https://www.bing.com/search?q={}".to_string(), icon_path: None },
                SearchEngine { keyword: "br".to_string(), name: "Brave".to_string(), url: "https://search.brave.com/search?q={}".to_string(), icon_path: None },
                SearchEngine { keyword: "e".to_string(), name: "Ecosia".to_string(), url: "https://www.ecosia.org/search?q={}".to_string(), icon_path: None },
                SearchEngine { keyword: "gh".to_string(), name: "GitHub".to_string(), url: "https://github.com/search?q={}".to_string(), icon_path: None },
                SearchEngine { keyword: "yt".to_string(), name: "YouTube".to_string(), url: "https://www.youtube.com/results?search_query={}".to_string(), icon_path: None },
            ];
            
            for engine in defaults {
                self.conn.execute(
                    "INSERT INTO search_engines (keyword, name, url, icon_path) VALUES (?1, ?2, ?3, ?4)",
                    params![engine.keyword, engine.name, engine.url, engine.icon_path],
                )?;
            }
        }
        
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
    
    pub fn reset_recent_stats(&self) -> Result<()> {
        self.conn.execute(
            "UPDATE browser_stats SET last_used = 0",
            [],
        )?;
        Ok(())
    }
    
    pub fn reset_frequent_stats(&self) -> Result<()> {
        self.conn.execute(
            "UPDATE browser_stats SET usage_count = 0",
            [],
        )?;
        Ok(())
    }
    
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }
    
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }
    
    // --- Search Engine CRUD ---
    
    pub fn list_engines(&self) -> Result<Vec<SearchEngine>> {
        let mut stmt = self.conn.prepare("SELECT keyword, name, url, icon_path FROM search_engines ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(SearchEngine {
                keyword: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                icon_path: row.get(3)?,
            })
        })?;
        
        let mut engines = Vec::new();
        for row in rows {
            engines.push(row?);
        }
        Ok(engines)
    }
    
    pub fn add_engine(&self, engine: &SearchEngine) -> Result<()> {
        self.conn.execute(
            "INSERT INTO search_engines (keyword, name, url, icon_path) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(keyword) DO UPDATE SET name=?2, url=?3, icon_path=?4",
            params![engine.keyword, engine.name, engine.url, engine.icon_path],
        )?;
        Ok(())
    }
    
    pub fn update_engine(&self, original_keyword: &str, engine: &SearchEngine) -> Result<()> {
        // Transaction to handle key change safely? SQLite allows simple updates.
        // If keyword changed, we might need to handle uniqueness, but let's assume valid.
        
        self.conn.execute(
            "UPDATE search_engines SET keyword=?1, name=?2, url=?3, icon_path=?4 WHERE keyword=?5",
            params![engine.keyword, engine.name, engine.url, engine.icon_path, original_keyword],
        )?;
        Ok(())
    }
     
    pub fn delete_engine(&self, keyword: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM search_engines WHERE keyword = ?1",
            params![keyword],
        )?;
        Ok(())
    }
    
    pub fn get_engine_by_keyword(&self, keyword: &str) -> Result<Option<SearchEngine>> {
        let mut stmt = self.conn.prepare("SELECT keyword, name, url, icon_path FROM search_engines WHERE keyword = ?1")?;
        let mut rows = stmt.query(params![keyword])?;
        
        if let Some(row) = rows.next()? {
             Ok(Some(SearchEngine {
                keyword: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                icon_path: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }
}
