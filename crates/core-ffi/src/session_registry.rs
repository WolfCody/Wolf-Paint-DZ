use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use core_model::handle::Handle;
use crate::drawing_session::DrawingSession;

static SESSIONS: Mutex<Option<HashMap<u64, DrawingSession>>> = Mutex::new(None);

fn lock() -> MutexGuard<'static, Option<HashMap<u64, DrawingSession>>> {
    SESSIONS.lock().unwrap_or_else(|p| p.into_inner())
}

pub fn init() { let mut g = lock(); if g.is_none() { *g = Some(HashMap::new()); } }

pub fn insert(handle: Handle, s: DrawingSession) { lock().as_mut().unwrap().insert(handle.raw(), s); }
pub fn remove(handle: Handle) -> Option<DrawingSession> { lock().as_mut().unwrap().remove(&handle.raw()) }
pub fn with<R>(handle: Handle, f: impl FnOnce(&mut DrawingSession) -> R) -> Option<R> {
    lock().as_mut().unwrap().get_mut(&handle.raw()).map(f)
}
