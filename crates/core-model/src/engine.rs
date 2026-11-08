//! Process-wide document registry and engine session.
//!
//! Stage 2.1b: `Engine` now stores `Mutex<Document>` so that `add_layer` and `set_pixel`
//! can mutate documents safely from any thread. All existing read-only operations use a
//! short-lived lock and copy what they need (`name()`, `info()`, handle list).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::document::{CanvasSize, Document, DocumentMode};
use crate::error::{Result, RpxError};
use crate::handle::{Handle, Registry};

pub struct EngineConfig {
    pub work_dir: PathBuf,
}

pub struct Engine {
    work_dir: PathBuf,
    documents: Registry<Mutex<Document>>,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.work_dir)
            .map_err(|e| RpxError::Io(e.to_string()))?;
        Ok(Engine {
            work_dir: config.work_dir,
            documents: Registry::new(),
        })
    }

    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    pub fn create_document(&self, name: &str, mode: DocumentMode, canvas: CanvasSize) -> Result<Handle> {
        let doc = Document::new(name, mode, canvas)?;
        self.documents.insert(Mutex::new(doc))
    }

    /// Returns the Arc wrapping `Mutex<Document>`. Callers lock it themselves.
    pub fn document(&self, handle: Handle) -> Result<Arc<Mutex<Document>>> {
        self.documents.get(handle)
    }

    /// Adds a raster layer to the document. Returns the new layer's unique ID.
    pub fn add_layer(&self, handle: Handle, name: String) -> Result<u64> {
        let doc_arc = self.documents.get(handle)?;
        let mut doc = doc_arc.lock().unwrap_or_else(|p| p.into_inner());
        Ok(doc.add_layer(name))
    }

    /// Writes one pixel (Android `0xAARRGGBB` format) into a layer of the document.
    /// Returns `true` on success, `false` if coordinates are out of bounds.
    /// Returns `Err` if the handle or layer_id is invalid.
    pub fn set_pixel(&self, handle: Handle, layer_id: u64, px: u32, py: u32, argb: u32) -> Result<bool> {
        let doc_arc = self.documents.get(handle)?;
        let mut doc = doc_arc.lock().unwrap_or_else(|p| p.into_inner());
        if doc.layer(layer_id).is_none() {
            return Err(RpxError::InvalidArgument(format!("layer {layer_id} not found")));
        }
        Ok(doc.set_pixel(layer_id, px, py, argb))
    }

    pub fn close_document(&self, handle: Handle) -> Result<()> {
        self.documents.remove(handle)?;
        Ok(())
    }

    pub fn open_documents(&self) -> Vec<Handle> {
        self.documents.handles()
    }
}
