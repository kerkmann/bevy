use std::cell::UnsafeCell;

use crate::world::World;

/// Pointer into memory with lifetime.
/// Guaranteed to be correctly aligned, non-null and safe to write for a particular type.
pub struct PtrMut<'a, T>(&'a UnsafeCell<T>);

impl<'a, T> PtrMut<'a, T> {
    /// Constructs a `PtrMut<'a, T>` from `&'a mut T`.
    pub fn from_mut(value: &mut T) -> Self {
        // SAFETY: `&mut` ensures unique access.
        unsafe { Self(&*(value as *mut T as *const UnsafeCell<T>)) }
    }

    /// Returns a shared reference to the `T`.
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - Value is not accessed in ways that violate Rust's rules for references.
    pub unsafe fn as_ref(self) -> &'a T {
        self.deref()
    }

    /// Returns mutable reference to the `T`.
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - Value is not accessed in ways that violate Rust's rules for references.
    pub unsafe fn as_mut(self) -> &'a mut T {
        self.deref_mut()
    }

    /// Returns a shared reference to the `T`.
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - Value is not accessed in ways that violate Rust's rules for references.
    pub unsafe fn deref(self) -> &'a T {
        &*self.0.get()
    }

    /// Returns mutable reference to the `T`.
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - Value is not accessed in ways that violate Rust's rules for references.
    pub unsafe fn deref_mut(self) -> &'a mut T {
        &mut *self.0.get()
    }
}

// for some reason #[derive(Clone, Copy)] did not work
impl<T> Copy for PtrMut<'_, T> {}
impl<T> Clone for PtrMut<'_, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

// SAFETY: We only internally make use of this in the multi-threaded executor,
// which does not run systems with conflicting access at the same time.
unsafe impl Send for PtrMut<'_, World> {}
unsafe impl Sync for PtrMut<'_, World> {}
