//! Engine session: configuration plus the registry of open documents.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::document::{CanvasSize, Document, DocumentMode};
use crate::error::{Result, RpxError};
use crate::handle::{Handle, Registry};

#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Absolute directory owned by the engine (tile spill files, working copies later on).
    /// Created if missing.
    pub work_dir: PathBuf,
}

pub struct Engine {
    config: EngineConfig,
    documents: Registry<Document>,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Result<Self> {
        let dir = &config.work_dir;
        if dir.as_os_str().is_empty() {
            return Err(RpxError::InvalidArgument("work_dir must not be empty".into()));
        }
        if !dir.is_absolute() {
            return Err(RpxError::InvalidArgument(format!(
                "work_dir must be an absolute path, got {}",
                dir.display()
            )));
        }
        std::fs::create_dir_all(dir)
            .map_err(|e| RpxError::Io(format!("cannot create {}: {e}", dir.display())))?;
        if !dir.is_dir() {
            return Err(RpxError::Io(format!("{} is not a directory", dir.display())));
        }
        Ok(Engine {
            config,
            documents: Registry::new(),
        })
    }

    pub fn work_dir(&self) -> &Path {
        &self.config.work_dir
    }

    pub fn create_document(&self, name: &str, mode: DocumentMode, canvas: CanvasSize) -> Result<Handle> {
        let document = Document::new(name, mode, canvas)?;
        self.documents.insert(document)
    }

    pub fn document(&self, handle: Handle) -> Result<Arc<Document>> {
        self.documents.get(handle)
    }

    pub fn close_document(&self, handle: Handle) -> Result<()> {
        self.documents.remove(handle).map(|_| ())
    }

    pub fn open_documents(&self) -> Vec<Handle> {
        self.documents.handles()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("rpx-core-{tag}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn creates_missing_work_dir() {
        let dir = temp_dir("create").join("nested");
        let engine = Engine::new(EngineConfig { work_dir: dir.clone() }).unwrap();
        assert!(engine.work_dir().is_dir());
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn rejects_empty_and_relative_work_dir() {
        let empty = Engine::new(EngineConfig { work_dir: PathBuf::new() });
        assert!(matches!(empty, Err(RpxError::InvalidArgument(_))));
        let relative = Engine::new(EngineConfig { work_dir: PathBuf::from("relative/dir") });
        assert!(matches!(relative, Err(RpxError::InvalidArgument(_))));
    }

    #[test]
    fn work_dir_that_is_a_file_is_an_io_error() {
        let dir = temp_dir("file");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("not_a_dir");
        std::fs::write(&file, b"x").unwrap();
        let result = Engine::new(EngineConfig { work_dir: file });
        assert!(matches!(result, Err(RpxError::Io(_))));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn document_lifecycle_through_handles() {
        let dir = temp_dir("life");
        let engine = Engine::new(EngineConfig { work_dir: dir.clone() }).unwrap();
        let canvas = CanvasSize::new(1024, 768).unwrap();
        let h = engine.create_document("Poster", DocumentMode::Raster, canvas).unwrap();
        assert_eq!(engine.document(h).unwrap().name(), "Poster");
        assert_eq!(engine.open_documents(), vec![h]);
        engine.close_document(h).unwrap();
        assert!(matches!(engine.document(h), Err(RpxError::InvalidHandle(_))));
        assert!(matches!(engine.close_document(h), Err(RpxError::InvalidHandle(_))));
        assert!(engine.open_documents().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_name_does_not_consume_a_handle() {
        let dir = temp_dir("badname");
        let engine = Engine::new(EngineConfig { work_dir: dir.clone() }).unwrap();
        let canvas = CanvasSize::new(10, 10).unwrap();
        assert!(engine.create_document("", DocumentMode::Raster, canvas).is_err());
        assert!(engine.open_documents().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
