use std::{
    marker::PhantomData,
    ptr::NonNull,
};

/// Pointer into memory with lifetime.
/// Guaranteed to be correctly aligned, non-null and safe to write for a particular type.
#[derive(Clone, Copy)]
pub struct PtrMut<'a, T>(NonNull<T>, PhantomData<&'a ()>);

impl<'a, T> PtrMut<'a, T> {
    /// Constructs a `PtrMut<'a, T>` from `&'a mut T`.
    pub fn from_mut(value: &'a mut T) -> Self {
        // SAFETY: References are non-null.
        unsafe {
            Self(NonNull::new_unchecked(value), PhantomData)
        }
    }

    /// Returns the underlying raw pointer.
    pub fn inner(self) -> NonNull<T> {
        self.0
    }

    /// Returns a shared reference to `T`.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure:
    /// - Pointee is a valid instance of `T`.
    /// - Pointee is not accessed in ways that violate Rust's rules for references.
    pub unsafe fn as_ref(self) -> &'a T {
        self.deref()
    }

    /// Returns mutable reference to a `T`.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure:
    /// - Pointee is a valid instance of `T`.
    /// - Pointee is not accessed in ways that violate Rust's rules for references.
    pub unsafe fn as_mut(self) -> &'a mut T {
        self.deref_mut()
    }

    /// Returns a shared reference to a `T`.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure:
    /// - Pointee is a valid instance of `T`.
    /// - Pointee is not accessed in ways that violate Rust's rules for references.
    pub unsafe fn deref(self) -> &'a T {
        &*self.inner().as_ptr()
    }

    /// Returns mutable reference to a `T`.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure:
    /// - Pointee is a valid instance of `T`.
    /// - Pointee is not accessed in ways that violate Rust's rules for references.
    pub unsafe fn deref_mut(self) -> &'a mut T {
        &mut *self.inner().as_ptr()
    }
}
