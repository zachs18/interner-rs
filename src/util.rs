#![cfg_attr(feature = "parking_lot", allow(dead_code))]

use std::sync::{RwLockReadGuard, RwLockWriteGuard, RwLock as StdRwLock};

#[repr(transparent)]
#[derive(Default)]
pub struct RwLock<T: ?Sized> {
    inner: StdRwLock<T>,
}

/// Wrapper for std::sync::RwLock that panics if poisoned.
/// 
/// This way the API matches `parking_lot::RwLock`.
impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        Self { inner: value.into() }
    }

    pub fn into_inner(self) -> T {
        self.inner.into_inner().unwrap()
    }
}

impl<T: ?Sized> RwLock<T> {
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.inner.read().unwrap()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.inner.write().unwrap()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.inner.get_mut().unwrap()
    }
}

/// Returns if `ptr` is aligned to a multiple of `align` bytes.
/// 
/// SAFETY: align must be a power of two
pub unsafe fn is_aligned_to(align: usize, ptr: *const u8) -> bool {
    (ptr as usize).trailing_zeros() >= align.trailing_zeros()
}


/// Returns the byte offset required to make `ptr` aligned to `align`.
/// 
/// SAFETY: align must be a power of two
pub unsafe fn align_offset(align: usize, ptr: *const u8) -> usize {
    // (align - ((ptr as usize) % align)) % align
    (align - ((ptr as usize) & align)) & align
}
