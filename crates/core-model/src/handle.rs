//! Opaque, validated handles and the registry that owns the objects behind them.
//!
//! Layout of a handle (u64, always > 0 and < 2^63 so it is a positive Java `long`):
//!
//! ```text
//!  bit 63      62 .. 32            31 .. 0
//!  [ 0 ] [ generation 1..=2^31-1 ] [ slot index + 1 (never 0) ]
//! ```
//!
//! * Closing an object bumps the slot generation, so every stale copy of the handle is
//!   rejected with `InvalidHandle` instead of touching a different object (ABA safety).
//! * Objects are stored as `Arc<T>`: `get` hands out a clone, so closing a document while
//!   another thread is still using it never frees memory under that thread.

use std::sync::{Arc, Mutex, MutexGuard};

use crate::error::{Result, RpxError};

const GENERATION_MASK: u32 = 0x7FFF_FFFF;

fn next_generation(current: u32) -> u32 {
    let next = current.wrapping_add(1) & GENERATION_MASK;
    if next == 0 { 1 } else { next }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(u64);

impl Handle {
    fn new(index: u32, generation: u32) -> Self {
        debug_assert!(index < u32::MAX);
        debug_assert!((1..=GENERATION_MASK).contains(&generation));
        Handle((u64::from(generation) << 32) | (u64::from(index) + 1))
    }

    /// Validates the *structure* of a raw value coming from the outside world.
    /// Whether the handle is still alive is checked by `Registry`.
    pub fn from_raw(raw: u64) -> Option<Self> {
        let low = raw & 0xFFFF_FFFF;
        let generation = raw >> 32;
        if low == 0 || generation == 0 || generation > u64::from(GENERATION_MASK) {
            None
        } else {
            Some(Handle(raw))
        }
    }

    pub fn raw(self) -> u64 {
        self.0
    }

    fn index(self) -> usize {
        ((self.0 & 0xFFFF_FFFF) - 1) as usize
    }

    fn generation(self) -> u32 {
        (self.0 >> 32) as u32
    }
}

struct Slot<T> {
    generation: u32,
    value: Option<Arc<T>>,
}

struct Inner<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}

/// Thread-safe slot map with generational handles.
pub struct Registry<T> {
    inner: Mutex<Inner<T>>,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Registry<T> {
    pub fn new() -> Self {
        Registry {
            inner: Mutex::new(Inner {
                slots: Vec::new(),
                free: Vec::new(),
            }),
        }
    }

    // Poisoning is recoverable here: every operation below leaves `Inner` consistent at each
    // step that can panic (there are none today), so we keep serving instead of bricking the app.
    fn lock(&self) -> MutexGuard<'_, Inner<T>> {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn insert(&self, value: T) -> Result<Handle> {
        let value = Arc::new(value);
        let mut inner = self.lock();
        if let Some(index) = inner.free.pop() {
            let slot = &mut inner.slots[index as usize];
            debug_assert!(slot.value.is_none());
            slot.value = Some(value);
            return Ok(Handle::new(index, slot.generation));
        }
        if inner.slots.len() >= (u32::MAX - 1) as usize {
            return Err(RpxError::Internal("handle space exhausted".into()));
        }
        let index = inner.slots.len() as u32;
        inner.slots.push(Slot {
            generation: 1,
            value: Some(value),
        });
        Ok(Handle::new(index, 1))
    }

    pub fn get(&self, handle: Handle) -> Result<Arc<T>> {
        let inner = self.lock();
        match inner.slots.get(handle.index()) {
            Some(Slot {
                generation,
                value: Some(value),
            }) if *generation == handle.generation() => Ok(Arc::clone(value)),
            _ => Err(RpxError::InvalidHandle(handle.raw())),
        }
    }

    /// Removes the object. Other threads that already hold an `Arc` keep a valid object.
    pub fn remove(&self, handle: Handle) -> Result<Arc<T>> {
        let mut inner = self.lock();
        let index = handle.index();
        let removed = match inner.slots.get_mut(index) {
            Some(slot) if slot.generation == handle.generation() => slot.value.take().map(|value| {
                slot.generation = next_generation(slot.generation);
                value
            }),
            _ => None,
        };
        match removed {
            Some(value) => {
                inner.free.push(index as u32);
                Ok(value)
            }
            None => Err(RpxError::InvalidHandle(handle.raw())),
        }
    }

    /// Live handles in slot order.
    pub fn handles(&self) -> Vec<Handle> {
        let inner = self.lock();
        inner
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.value.is_some())
            .map(|(index, slot)| Handle::new(index as u32, slot.generation))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.lock().slots.iter().filter(|slot| slot.value.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_remove_roundtrip() {
        let reg = Registry::new();
        let h = reg.insert(String::from("a")).unwrap();
        assert_eq!(*reg.get(h).unwrap(), "a");
        assert_eq!(reg.len(), 1);
        assert_eq!(*reg.remove(h).unwrap(), "a");
        assert!(reg.is_empty());
    }

    #[test]
    fn handles_are_positive_java_longs_and_never_zero() {
        let reg = Registry::new();
        for _ in 0..100 {
            let raw = reg.insert(0u8).unwrap().raw();
            assert!(raw > 0);
            assert!(i64::try_from(raw).is_ok());
        }
    }

    #[test]
    fn stale_handle_is_rejected_after_close() {
        let reg = Registry::new();
        let h = reg.insert(1u32).unwrap();
        reg.remove(h).unwrap();
        assert_eq!(reg.get(h).unwrap_err(), RpxError::InvalidHandle(h.raw()));
        assert_eq!(reg.remove(h).unwrap_err(), RpxError::InvalidHandle(h.raw()));
    }

    #[test]
    fn reused_slot_gets_new_generation_and_old_handle_stays_dead() {
        let reg = Registry::new();
        let old = reg.insert(1u32).unwrap();
        reg.remove(old).unwrap();
        let new = reg.insert(2u32).unwrap();
        assert_ne!(old, new);
        assert_eq!(old.index(), new.index(), "slot is reused");
        assert!(reg.get(old).is_err());
        assert_eq!(*reg.get(new).unwrap(), 2);
    }

    #[test]
    fn generation_wraps_without_reaching_zero() {
        assert_eq!(next_generation(1), 2);
        assert_eq!(next_generation(GENERATION_MASK), 1);
        let reg = Registry::new();
        let h = reg.insert(1u32).unwrap();
        reg.lock().slots[0].generation = GENERATION_MASK;
        // `h` now has an outdated generation (as if it were closed long ago).
        assert!(reg.get(h).is_err());
        let live = Handle::new(0, GENERATION_MASK);
        reg.remove(live).unwrap();
        let after = reg.insert(2u32).unwrap();
        assert_eq!(after.generation(), 1);
        assert!(reg.get(live).is_err());
    }

    #[test]
    fn from_raw_rejects_malformed_values() {
        assert!(Handle::from_raw(0).is_none());
        assert!(Handle::from_raw(5).is_none(), "generation 0");
        assert!(Handle::from_raw(1u64 << 32).is_none(), "slot field 0");
        assert!(Handle::from_raw(u64::MAX).is_none());
        assert!(Handle::from_raw((1u64 << 63) | 1).is_none(), "negative as i64");
        assert!(Handle::from_raw((1u64 << 32) | 1).is_some());
    }

    #[test]
    fn forged_handle_with_unknown_slot_is_rejected() {
        let reg: Registry<u8> = Registry::new();
        let forged = Handle::from_raw((1u64 << 32) | 99).unwrap();
        assert!(reg.get(forged).is_err());
        assert!(reg.remove(forged).is_err());
    }

    #[test]
    fn closing_does_not_invalidate_a_clone_held_elsewhere() {
        let reg = Registry::new();
        let h = reg.insert(vec![1, 2, 3]).unwrap();
        let held = reg.get(h).unwrap();
        reg.remove(h).unwrap();
        assert_eq!(*held, vec![1, 2, 3]);
        assert!(reg.get(h).is_err());
    }

    #[test]
    fn handles_lists_only_live_objects() {
        let reg = Registry::new();
        let a = reg.insert(1u8).unwrap();
        let b = reg.insert(2u8).unwrap();
        let c = reg.insert(3u8).unwrap();
        reg.remove(b).unwrap();
        assert_eq!(reg.handles(), vec![a, c]);
    }
}
