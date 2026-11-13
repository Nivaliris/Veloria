use std::path::{Path,PathBuf};
use anyhow::Result;
use rusqlite::{params, Connection};

pub struct Storage { conn: Connection }
impl Storage {
    pub fn open() -> Result<Self> {
        let folder = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("Veloria");
        std::fs::create_dir_all(&folder)?;
        let conn = Connection::open(folder.join("veloria.db"))?;
        conn.execute_batch("PRAGMA foreign_keys=ON;
          CREATE TABLE IF NOT EXISTS preferences (key TEXT PRIMARY KEY,value TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS categories (name TEXT PRIMARY KEY);
          CREATE TABLE IF NOT EXISTS book_data (path TEXT PRIMARY KEY, favorite INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'Por ler');
          CREATE TABLE IF NOT EXISTS book_categories (path TEXT NOT NULL, category TEXT NOT NULL, PRIMARY KEY(path,category));")?;
        Ok(Self { conn })
    }
    pub fn root(&self) -> Option<PathBuf> {
        self.conn.query_row("SELECT value FROM preferences WHERE key='root'", [], |r| r.get::<_,String>(0)).ok().map(PathBuf::from)
    }
    pub fn set_root(&self, path: &Path) -> Result<()> {
        self.conn.execute("INSERT INTO preferences(key,value) VALUES('root',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [path.to_string_lossy().as_ref()])?;
        Ok(())
    }
    pub fn categories(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT name FROM categories ORDER BY name COLLATE NOCASE")?;
        Ok(stmt.query_map([], |r| r.get::<_,String>(0))?.filter_map(Result::ok).collect())
    }
    pub fn add_category(&self, name: &str) -> Result<()> {
        self.conn.execute("INSERT OR IGNORE INTO categories(name) VALUES(?1)", [name])?;
        Ok(())
    }
    pub fn remove_category(&self, name: &str) -> Result<()> {
        self.conn.execute("DELETE FROM book_categories WHERE category=?1", [name])?;
        self.conn.execute("DELETE FROM categories WHERE name=?1", [name])?;
        Ok(())
    }
    pub fn rename_category(&self, old: &str, new: &str) -> Result<()> {
        self.add_category(new)?;
        self.conn.execute("UPDATE OR IGNORE book_categories SET category=?2 WHERE category=?1", params![old,new])?;
        self.remove_category(old)?;
        Ok(())
    }
    pub fn book_data(&self, path: &Path) -> (bool,String,Vec<String>) {
        let key = path.to_string_lossy();
        let (favorite,status) = self.conn.query_row("SELECT favorite,status FROM book_data WHERE path=?1",[key.as_ref()],|r| Ok((r.get::<_,i64>(0)? != 0,r.get::<_,String>(1)?))).unwrap_or((false,"Por ler".into()));
        let categories = self.conn.prepare("SELECT category FROM book_categories WHERE path=?1 ORDER BY category")
          .and_then(|mut stmt| stmt.query_map([key.as_ref()],|r| r.get::<_,String>(0)).map(|x| x.filter_map(Result::ok).collect())).unwrap_or_default();
        (favorite,status,categories)
    }
    pub fn set_favorite(&self,path:&Path,fav:bool)->Result<()> {
        self.conn.execute("INSERT INTO book_data(path,favorite) VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET favorite=excluded.favorite",params![path.to_string_lossy().as_ref(),fav as i64])?;
        Ok(())
    }
    pub fn set_status(&self,path:&Path,status:&str)->Result<()> {
        self.conn.execute("INSERT INTO book_data(path,status) VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET status=excluded.status",params![path.to_string_lossy().as_ref(),status])?;
        Ok(())
    }
    pub fn toggle_category(&self,path:&Path,category:&str,has:bool)->Result<()> {
        let key=path.to_string_lossy();
        if has { self.conn.execute("DELETE FROM book_categories WHERE path=?1 AND category=?2",params![key.as_ref(),category])?; }
        else { self.conn.execute("INSERT OR IGNORE INTO book_categories(path,category) VALUES(?1,?2)",params![key.as_ref(),category])?; }
        Ok(())
    }
}
