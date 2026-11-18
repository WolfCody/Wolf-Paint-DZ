use std::sync::Arc;
use core_model::raster_layer::{LayerId, LayerMeta};
use crate::tile_store::TileStore;

pub struct RasterLayer {
    pub meta: LayerMeta,
    pub store: TileStore,
}

impl RasterLayer {
    pub fn new(id: LayerId, name: &str) -> core_model::error::Result<Self> {
        Ok(RasterLayer { meta: LayerMeta::new(id, name)?, store: TileStore::new() })
    }
    pub fn id(&self) -> LayerId { self.meta.id }
    pub fn snapshot_store(&self) -> TileStore { self.store.snapshot() }
}
