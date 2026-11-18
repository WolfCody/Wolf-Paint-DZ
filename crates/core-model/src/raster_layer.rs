use crate::error::{Result, RpxError};

pub const MAX_LAYERS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerId(pub u64);

#[derive(Debug, Clone)]
pub struct LayerMeta {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
}

impl LayerMeta {
    pub fn new(id: LayerId, name: &str) -> Result<Self> {
        let n = name.trim();
        if n.is_empty() {
            return Err(RpxError::InvalidArgument("layer name must not be empty".into()));
        }
        Ok(LayerMeta { id, name: n.to_owned(), visible: true, locked: false, opacity: 1.0 })
    }
}
