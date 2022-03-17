use std::cell::UnsafeCell;

/// Uses interior mutability to yeet unpopular API leftovers.
pub enum SemiSafeCell<'a, T> {
    Ref(&'a T),
    Mut(&'a UnsafeCell<T>),
}

impl<'a, T> SemiSafeCell<'a, T> {
    /// Returns a `SemiSafeCell<T>` from a `&T`.
    pub fn from_ref(value: &'a T) -> Self {
        Self::Ref(value)
    }

    /// Returns a `SemiSafeCell<T>` from a  `&mut T`.
    pub fn from_mut(value: &mut T) -> Self {
        // SAFETY: `&mut` ensures unique access.
        unsafe { Self::Mut(&*(value as *mut T as *const UnsafeCell<T>)) }
    }

    /// Returns a shared reference to the underlying data.
    ///
    /// # Safety
    ///
    /// Caller must ensure there are no active mutable references to the underlying data.
    pub unsafe fn as_ref(&self) -> &'a T {
        match self {
            Self::Ref(borrow) => *borrow,
            Self::Mut(cell) => &*cell.get(),
        }
    }

    /// Returns mutable reference to the underlying data.
    ///
    /// # Safety
    ///
    /// Caller must ensure access to the underlying data is unique (no active references, mutable or not).
    pub unsafe fn as_mut(&self) -> &'a mut T {
        match self {
            Self::Ref(_) => {
                panic!("cannot get a mutable reference from SemiSafeCell::Ref");
            }
            Self::Mut(cell) => &mut *cell.get(),
        }
    }
}

impl<T> Copy for SemiSafeCell<'_, T> {}
impl<T> Clone for SemiSafeCell<'_, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Ref(val) => Self::Ref(val),
            Self::Mut(val) => Self::Mut(val),
        }
    }
}

// SAFETY: Multi-threaded executor does not run systems with conflicting access at the same time.
unsafe impl<T> Send for SemiSafeCell<'_, T> {}
unsafe impl<T> Sync for SemiSafeCell<'_, T> {}
