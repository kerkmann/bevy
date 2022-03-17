use std::cell::UnsafeCell;

use crate::world::World;

/// Uses interior mutability to yeet unpopular API leftovers.
pub struct SemiSafeCell<'a, T>(&'a UnsafeCell<T>);

impl<'a, T> SemiSafeCell<'a, T> {
    /// Returns a `SemiSafeCell<T>` from a  `&mut T`.
    pub fn from_mut(value: &mut T) -> Self {
        // SAFETY: `&mut` ensures unique access.
        unsafe { Self(&*(value as *mut T as *const UnsafeCell<T>)) }
    }

    /// Returns a shared reference to the underlying data.
    ///
    /// # Safety
    ///
    /// Caller must ensure there are no active mutable references to the underlying data.
    pub unsafe fn as_ref(&self) -> &'a T {
        &*self.0.get()
    }

    /// Returns mutable reference to the underlying data.
    ///
    /// # Safety
    ///
    /// Caller must ensure access to the underlying data is unique (no active references, mutable or not).
    pub unsafe fn as_mut(&self) -> &'a mut T {
        &mut *self.0.get()
    }
}

impl<T> Copy for SemiSafeCell<'_, T> {}
impl<T> Clone for SemiSafeCell<'_, T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

// SAFETY: Multi-threaded executor does not run systems with conflicting access at the same time.
unsafe impl Send for SemiSafeCell<'_, World> {}
unsafe impl Sync for SemiSafeCell<'_, World> {}
