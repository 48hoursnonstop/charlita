use crate::model::Document;
use anyhow::{Context, Result};
use fs2::FileExt;
use rusqlite::{Connection, params};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct Store {
    pub root: PathBuf,
    db: Mutex<Connection>,
    _lock: File,
}
impl Store {
    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(root.join("assets"))?;
        fs::create_dir_all(root.join("logs"))?;
        let lock = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("instance.lock"))?;
        lock.try_lock_exclusive().context("Charlita is already running. Open it from the system tray / Charlita ya está abierta. Ábrela desde la bandeja")?;
        let db = Connection::open(root.join("charlita.sqlite"))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS documents (name TEXT PRIMARY KEY, body TEXT NOT NULL); CREATE TABLE IF NOT EXISTS metadata (name TEXT PRIMARY KEY, value TEXT NOT NULL);")?;
        let this = Self {
            root,
            db: Mutex::new(db),
            _lock: lock,
        };
        if this.load("live")?.is_none() {
            this.save("live", &Document::default())?;
        }
        if this.load("draft")?.is_none() {
            this.save("draft", &this.load("live")?.unwrap())?;
        }
        Ok(this)
    }
    pub fn load(&self, name: &str) -> Result<Option<Document>> {
        let db = self.db.lock().unwrap();
        let mut q = db.prepare("SELECT body FROM documents WHERE name=?1")?;
        let mut rows = q.query([name])?;
        if let Some(r) = rows.next()? {
            let text: String = r.get(0)?;
            let d: Document = serde_json::from_str(&text)?;
            d.validate()?;
            Ok(Some(d))
        } else {
            Ok(None)
        }
    }
    pub fn save(&self, name: &str, d: &Document) -> Result<()> {
        d.validate()?;
        self.db.lock().unwrap().execute("INSERT INTO documents(name,body) VALUES(?1,?2) ON CONFLICT(name) DO UPDATE SET body=excluded.body",params![name,serde_json::to_string(d)?])?;
        Ok(())
    }
    pub fn apply(&self, d: &Document) -> Result<()> {
        self.save_pair(d, d)
    }
    pub fn save_pair(&self, draft: &Document, live: &Document) -> Result<()> {
        draft.validate()?;
        live.validate()?;
        let mut db = self.db.lock().unwrap();
        let tx = db.transaction()?;
        for (name, document) in [("draft", draft), ("live", live)] {
            let body = serde_json::to_string(document)?;
            tx.execute("INSERT INTO documents(name,body) VALUES(?1,?2) ON CONFLICT(name) DO UPDATE SET body=excluded.body",params![name,body])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn metadata(&self, name: &str, value: &str) -> Result<()> {
        self.db.lock().unwrap().execute("INSERT INTO metadata(name,value) VALUES(?1,?2) ON CONFLICT(name) DO UPDATE SET value=excluded.value",params![name,value])?;
        Ok(())
    }
    pub fn metadata_value(&self, name: &str) -> Result<Option<String>> {
        use rusqlite::OptionalExtension;
        Ok(self
            .db
            .lock()
            .unwrap()
            .query_row("SELECT value FROM metadata WHERE name=?1", [name], |row| {
                row.get(0)
            })
            .optional()?)
    }
    pub fn asset_path(&self, a: &crate::model::Asset) -> PathBuf {
        self.root.join("assets").join(a.filename())
    }
    pub fn token(&self) -> Result<String> {
        let db = self.db.lock().unwrap();
        if let Ok(t) = db.query_row(
            "SELECT value FROM metadata WHERE name='overlay_key'",
            [],
            |r| r.get::<_, String>(0),
        ) {
            return Ok(t);
        }
        let token = format!("{}{}", crate::model::id(), crate::model::id());
        db.execute(
            "INSERT INTO metadata(name,value) VALUES('overlay_key',?1)",
            [&token],
        )?;
        Ok(token)
    }
}
pub fn data_root(portable: bool, explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(std::path::absolute(p)?);
    }
    let executable = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_exe()?);
    if portable || executable.with_file_name("portable.flag").exists() {
        return Ok(executable.parent().unwrap().join("data"));
    }
    Ok(
        directories::ProjectDirs::from("io.github", "48hoursnonstop", "Charlita")
            .context("Cannot locate application data folder")?
            .data_local_dir()
            .to_owned(),
    )
}
pub fn portable_distribution(portable: bool) -> Result<bool> {
    let executable = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_exe()?);
    Ok(portable || executable.with_file_name("portable.flag").exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn apply_is_atomic_and_draft_does_not_publish() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open(dir.path().into()).unwrap();
        let mut d = s.load("draft").unwrap().unwrap();
        d.profiles[0].name = "Draft".into();
        s.save("draft", &d).unwrap();
        assert_ne!(s.load("live").unwrap().unwrap().profiles[0].name, "Draft");
        s.apply(&d).unwrap();
        assert_eq!(s.load("live").unwrap().unwrap(), d);
    }
    #[test]
    fn stable_key_survives_restart_and_single_instance_is_enforced() {
        let d = tempfile::tempdir().unwrap();
        let key;
        {
            let s = Store::open(d.path().into()).unwrap();
            key = s.token().unwrap();
            assert!(Store::open(d.path().into()).is_err());
        }
        assert_eq!(Store::open(d.path().into()).unwrap().token().unwrap(), key);
    }
}
