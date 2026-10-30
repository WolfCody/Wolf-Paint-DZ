//! Integration scenarios: the 8000x8000 acceptance canvas and concurrent use of the engine.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use core_model::{CanvasSize, DocumentMode, Engine, EngineConfig, RpxError};

fn engine(tag: &str) -> (Engine, PathBuf) {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("rpx-it-{tag}-{}-{nanos}", std::process::id()));
    (Engine::new(EngineConfig { work_dir: dir.clone() }).unwrap(), dir)
}

#[test]
fn creating_8000_square_documents_allocates_no_pixel_memory() {
    let (engine, dir) = engine("large");
    let canvas = CanvasSize::new(8000, 8000).unwrap();
    // 32 documents would need 8 GB if each held a dense RGBA buffer.
    let handles: Vec<_> = (0..32)
        .map(|i| engine.create_document(&format!("big {i}"), DocumentMode::Raster, canvas).unwrap())
        .collect();
    for h in &handles {
        let info = engine.document(*h).unwrap().info();
        assert_eq!(info.tiles_total, 1024);
        assert_eq!(info.tiles_allocated, 0);
        assert_eq!(info.dense_rgba_bytes, 256_000_000);
    }
    for h in handles {
        engine.close_document(h).unwrap();
    }
    assert!(engine.open_documents().is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn concurrent_create_read_close_is_consistent() {
    let (engine, dir) = engine("threads");
    let canvas = CanvasSize::new(512, 512).unwrap();
    std::thread::scope(|scope| {
        for t in 0..8 {
            let engine = &engine;
            scope.spawn(move || {
                for i in 0..200 {
                    let h = engine
                        .create_document(&format!("t{t}-{i}"), DocumentMode::Vector, canvas)
                        .unwrap();
                    assert_eq!(engine.document(h).unwrap().name(), format!("t{t}-{i}"));
                    engine.close_document(h).unwrap();
                    assert!(matches!(engine.document(h), Err(RpxError::InvalidHandle(_))));
                }
            });
        }
    });
    assert!(engine.open_documents().is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn closing_a_document_while_another_thread_uses_it_is_safe() {
    let (engine, dir) = engine("race");
    let canvas = CanvasSize::new(64, 64).unwrap();
    let h = engine.create_document("shared", DocumentMode::Raster, canvas).unwrap();
    let held = engine.document(h).unwrap();
    engine.close_document(h).unwrap();
    // The Arc obtained before close stays valid; new lookups fail.
    assert_eq!(held.info().width, 64);
    assert!(engine.document(h).is_err());
    let _ = std::fs::remove_dir_all(dir);
}
