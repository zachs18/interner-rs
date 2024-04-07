use std::{
    rc::Rc,
    sync::{RwLock as StdRwLock, RwLockReadGuard, RwLockWriteGuard},
};

#[allow(dead_code)]
#[derive(Default)]
#[repr(transparent)]
pub struct RwLock<T: ?Sized> {
    inner: StdRwLock<T>,
}

/// Wrapper for std::sync::RwLock that panics if poisoned.
///
#[allow(dead_code)]
/// This way the API matches `parking_lot::RwLock`.
impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: value.into(),
        }
    }

    pub fn into_inner(self) -> T {
        self.inner.into_inner().unwrap()
    }
}

#[allow(dead_code)]
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
pub(crate) unsafe fn is_aligned_to(align: usize, ptr: *const u8) -> bool {
    (ptr as usize).trailing_zeros() >= align.trailing_zeros()
}

/// Returns the byte offset required to make `ptr` aligned to `align`.
///
/// SAFETY: align must be a power of two
pub(crate) unsafe fn align_offset(align: usize, ptr: *const u8) -> usize {
    // (align - ((ptr as usize) % align)) % align
    let mask = align - 1;
    (align - ((ptr as usize) & mask)) & mask
}

#[cfg(feature = "bytemuck")]
use bytemuck::NoUninit;

pub unsafe trait Interner {
    /// Attempts to clear all data held by this interner without deallocating.
    ///
    /// This function is safe because it takes a &mut self, which guarantees no other references exist into data held by this interner.
    ///
    /// If this Interner could not be cleared (e.g. because it is shared, e.g. Rc<dyn Interner>), then an Err variant is returned.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let mut interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let name: &str = "Ferris";
    /// let greeting1 = interner.add_str(&format!("Hello, {name}!"));
    /// let greeting2 = interner.find_str("Hello, Ferris!");
    /// assert_eq!(greeting1, "Hello, Ferris!");
    /// assert_eq!(greeting2, Some("Hello, Ferris!"));
    /// interner.try_clear().unwrap();
    /// let greeting3 = interner.find_str("Hello, Ferris!");
    /// assert_eq!(greeting3, None);
    /// ```
    fn try_clear(&mut self) -> Result<(), ()>;

    /// Return a reference to data equal to `value` in this interner, if it exists.
    ///
    /// Empty slices will always succeed and may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, unsync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let name: &str = "Mary Sue";
    /// let greeting1 = interner.add_str(&format!("Hello, {name}!"));
    /// let greeting2 = interner.find_bytes(b"Hello, Mary");
    /// let greeting3 = interner.find_bytes(b"Hello, Sue");
    /// assert_eq!(greeting1, "Hello, Mary Sue!");
    /// assert_eq!(greeting2, Some(b"Hello, Mary" as &[u8]));
    /// assert_eq!(greeting3, None);
    /// ```
    fn find_bytes(&self, value: &[u8]) -> Option<&[u8]>;

    /// Return a reference to data equal to `value` in this interner, adding it if it does not yet exist.
    ///
    /// Empty slices may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, unsync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let name: &str = "Mary Sue";
    /// let greeting1 = interner.add_str(&format!("Hello, {name}!"));
    /// let greeting2 = interner.find_or_add_bytes(b"Hello, Mary");
    /// let greeting3 = interner.find_or_add_bytes(b"Hello, Sue");
    /// assert_eq!(greeting1, "Hello, Mary Sue!");
    /// assert_eq!(greeting2, b"Hello, Mary");
    /// assert_eq!(greeting3, b"Hello, Sue");
    /// ```
    fn find_or_add_bytes(&self, value: &[u8]) -> &[u8];

    /// Insert data equal to `value` into this interner, returning a reference to it.
    ///
    /// Empty slices may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, unsync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let name: &str = "Ferris";
    /// let greeting = interner.add_bytes(format!("Hello, {name}!").as_bytes());
    /// assert_eq!(greeting, b"Hello, Ferris!");
    /// ```
    fn add_bytes(&self, value: &[u8]) -> &[u8];

    /// Insert `value` into this interner, returning a reference to it's data.
    ///
    /// This will always succeed if `value.capacity() == 0`. Note that in this case a static slice may be returned.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let name: &str = "Ferris";
    /// let greeting = interner.add_owned_bytes(format!("Hello, {name}!").into_bytes());
    /// assert_eq!(greeting, b"Hello, Ferris!");
    /// ```
    fn add_owned_bytes(&self, value: Vec<u8>) -> &[u8];

    /// Return a reference to data equal to `value` in this interner, if it exists.
    ///
    /// Empty slices will always succeed and may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let name: &str = "Mary Sue";
    /// let greeting1 = interner.add_str(&format!("Hello, {name}!"));
    /// let greeting2 = interner.find_str("Hello, Mary");
    /// let greeting3 = interner.find_str("Hello, Sue");
    /// assert_eq!(greeting1, "Hello, Mary Sue!");
    /// assert_eq!(greeting2, Some("Hello, Mary"));
    /// assert_eq!(greeting3, None);
    /// ```
    fn find_str(&self, value: &str) -> Option<&str> {
        let owned = self.find_bytes(value.as_bytes())?;
        // SAFETY: owned == value.as_bytes() bytewise, and value is valid utf8
        Some(unsafe { std::str::from_utf8_unchecked(owned) })
    }

    /// Return a reference to data equal to `value` in this interner, adding it if it does not yet exist.
    ///
    /// Empty slices may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let name: &str = "Mary Sue";
    /// let greeting1 = interner.add_str(&format!("Hello, {name}!"));
    /// let greeting2 = interner.find_or_add_str("Hello, Mary");
    /// let greeting3 = interner.find_or_add_str("Hello, Sue");
    /// assert_eq!(greeting1, "Hello, Mary Sue!");
    /// assert_eq!(greeting2, "Hello, Mary");
    /// assert_eq!(greeting3, "Hello, Sue");
    /// ```
    fn find_or_add_str(&self, value: &str) -> &str {
        let owned = self.find_or_add_bytes(value.as_bytes());
        // SAFETY: owned == value.as_bytes() bytewise, and value is valid utf8
        unsafe { std::str::from_utf8_unchecked(owned) }
    }

    /// Insert data equal to `value` into this interner, returning a reference to it.
    ///
    /// Empty slices may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let name: &str = "Ferris";
    /// let greeting = interner.add_str(&format!("Hello, {name}!"));
    /// assert_eq!(greeting, "Hello, Ferris!");
    /// ```
    fn add_str(&self, value: &str) -> &str {
        let owned = self.add_bytes(value.as_bytes());
        // SAFETY: owned == value.as_bytes() bytewise, and value is valid utf8
        unsafe { std::str::from_utf8_unchecked(owned) }
    }

    /// Insert `value` into this interner, returning a reference to it's data.
    ///
    /// This will always succeed if `value.capacity() == 0`. Note that in this case a static slice may be returned.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let name: &str = "Ferris";
    /// let greeting = interner.add_owned_string(format!("Hello, {name}!"));
    /// assert_eq!(greeting, "Hello, Ferris!");
    /// ```
    fn add_owned_string(&self, value: String) -> &str {
        let owned = self.add_owned_bytes(value.into_bytes());
        // SAFETY: owned == value.as_bytes() bytewise, and value is valid utf8
        unsafe { std::str::from_utf8_unchecked(owned) }
    }

    /// Return a reference to data bytewise-equal to `value` in this interner, if it exists and is sufficiently aligned.
    ///
    /// Empty slices and ZSTs will always succeed and may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let value1 = interner.add_owned_bytes(vec![0x55; 7]);
    /// let value2 = interner.find_slice(&[0x5555u16; 3]);
    /// let value3 = interner.find_slice(&[0x5555u16; 4]);
    /// assert_eq!(value1, &[0x55; 7]);
    /// assert_eq!(value2.unwrap(), &[0x5555u16; 3]);
    /// assert_eq!(value3, None);
    /// ```
    #[cfg(feature = "bytemuck")]
    fn find_slice<T: NoUninit + 'static>(&self, value: &[T]) -> Option<&[T]>;

    /// Return a reference to data bytewise-equal to `value` in this interner, adding it if it does not yet exist.
    ///
    /// Empty slices and ZSTs may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let value1 = interner.add_owned_bytes(vec![0x55; 7]);
    /// let value2 = interner.find_or_add_slice(&[0x5555u16; 3]);
    /// let value3 = interner.find_or_add_slice(&[0xAAAAu16; 3]);
    /// assert_eq!(value1, &[0x55; 7]);
    /// assert_eq!(value2, &[0x5555u16; 3]);
    /// assert_eq!(value3, &[0xAAAAu16; 3]);
    /// ```
    #[cfg(feature = "bytemuck")]
    fn find_or_add_slice<T: NoUninit + 'static>(&self, value: &[T]) -> &[T];

    /// Return a reference to data bytewise-equal to `value` in this interner, adding it if it does not yet exist.
    ///
    /// Empty slices and ZSTs may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let value = interner.add_slice(&[0x5555u16; 3]);
    /// assert_eq!(value, &[0x5555u16; 3]);
    /// ```
    #[cfg(feature = "bytemuck")]
    fn add_slice<T: NoUninit + 'static>(&self, value: &[T]) -> &[T];

    /// Insert `value` into this interner, returning a reference to it's data.
    ///
    /// This will always succeed if `size_of::<T>() == 0` or `value.capacity() == 0`. Note that in this case a static slice may be returned.
    ///
    /// Otherwise, this will fail if `align_of::<T>() != 1`.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let value1 = interner.try_add_owned(vec![[1u8, 2, 3]; 8]);
    /// let value2 = interner.find_bytes(&[1, 2, 3, 1, 2, 3]);
    /// assert_eq!(value1, Ok(&[[1u8, 2, 3]; 8] as &[[u8; 3]]));
    /// assert_eq!(value2, Some(&[1u8, 2, 3, 1, 2, 3] as &[u8]));
    /// ```
    #[cfg(feature = "bytemuck")]
    fn try_add_owned<T: NoUninit + 'static>(&self, value: Vec<T>) -> Result<&[T], Vec<T>>;

    /// Return a reference to data bytewise-equal to `value` in this interner, if it exists and is sufficiently aligned.
    ///
    /// Empty slices and ZSTs will always succeed and may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let value1 = interner.add_owned_bytes(vec![0x55; 7]);
    /// let value2 = interner.find_value(&0x55555555u32);
    /// let value3 = interner.find_value(&0x5555u32);
    /// assert_eq!(value1, &[0x55; 7]);
    /// assert_eq!(value2, Some(&0x55555555u32));
    /// assert_eq!(value3, None);
    /// ```
    #[cfg(feature = "bytemuck")]
    fn find_value<T: NoUninit + 'static>(&self, value: &T) -> Option<&T>;

    /// Return a reference to data bytewise-equal to `value` in this interner, adding it if it does not exist or is not sufficiently aligned.
    ///
    /// ZSTs may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let value1 = interner.add_owned_bytes(vec![0x55; 7]);
    /// let value2 = interner.find_or_add_value(&0x55555555u32);
    /// let value3 = interner.find_or_add_value(&0x5555u32);
    /// assert_eq!(value1, &[0x55; 7]);
    /// assert_eq!(value2, &0x55555555u32);
    /// assert_eq!(value3, &0x5555u32);
    /// ```
    #[cfg(feature = "bytemuck")]
    fn find_or_add_value<T: NoUninit + 'static>(&self, value: &T) -> &T;

    /// Insert data bytewise-equal to `value` in this interner, returning a reference to it.
    ///
    /// ZSTs may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// interner.add_value(&0x55555555u32);
    /// let value = interner.find_value(&0x55555555u32);
    /// assert_eq!(value, Some(&0x55555555u32));
    /// ```
    #[cfg(feature = "bytemuck")]
    fn add_value<T: NoUninit + 'static>(&self, value: &T) -> &T;
}

unsafe impl<I: Interner + ?Sized> Interner for Rc<I> {
    fn try_clear(&mut self) -> Result<(), ()> {
        if let Some(this) = Rc::get_mut(self) {
            this.try_clear()
        } else {
            Err(())
        }
    }

    fn find_bytes(&self, value: &[u8]) -> Option<&[u8]> {
        (**self).find_bytes(value)
    }

    fn find_or_add_bytes(&self, value: &[u8]) -> &[u8] {
        (**self).find_or_add_bytes(value)
    }

    fn add_bytes(&self, value: &[u8]) -> &[u8] {
        (**self).add_bytes(value)
    }

    fn add_owned_bytes(&self, value: Vec<u8>) -> &[u8] {
        (**self).add_owned_bytes(value)
    }

    #[cfg(feature = "bytemuck")]
    fn find_slice<T: NoUninit + 'static>(&self, value: &[T]) -> Option<&[T]>
    where
        Self: Sized,
    {
        (**self).find_slice(value)
    }

    #[cfg(feature = "bytemuck")]
    fn find_or_add_slice<T: NoUninit + 'static>(&self, value: &[T]) -> &[T]
    where
        Self: Sized,
    {
        (**self).find_or_add_slice(value)
    }

    #[cfg(feature = "bytemuck")]
    fn add_slice<T: NoUninit + 'static>(&self, value: &[T]) -> &[T]
    where
        Self: Sized,
    {
        (**self).add_slice(value)
    }

    #[cfg(feature = "bytemuck")]
    fn try_add_owned<T: NoUninit + 'static>(&self, value: Vec<T>) -> Result<&[T], Vec<T>>
    where
        Self: Sized,
    {
        (**self).try_add_owned(value)
    }

    #[cfg(feature = "bytemuck")]
    fn find_value<T: NoUninit + 'static>(&self, value: &T) -> Option<&T>
    where
        Self: Sized,
    {
        (**self).find_value(value)
    }

    #[cfg(feature = "bytemuck")]
    fn find_or_add_value<T: NoUninit + 'static>(&self, value: &T) -> &T
    where
        Self: Sized,
    {
        (**self).find_or_add_value(value)
    }

    #[cfg(feature = "bytemuck")]
    fn add_value<T: NoUninit + 'static>(&self, value: &T) -> &T
    where
        Self: Sized,
    {
        (**self).add_value(value)
    }
}

#[cfg(feature = "yoke")]
use stable_deref_trait::StableDeref;
#[cfg(feature = "yoke")]
use std::ops::Deref;
#[cfg(feature = "yoke")]
use yoke::Yoke;

#[cfg(feature = "yoke")]
pub unsafe trait RcInterner: Clone + StableDeref
where
    <Self as Deref>::Target: Interner,
{
    /// Return a reference to data equal to `value` in this interner, if it exists.
    ///
    /// Empty slices will always succeed and may not actually be stored.
    fn yoked_find_bytes(&self, value: &[u8]) -> Option<Yoke<&'static [u8], Self>> {
        Yoke::try_attach_to_cart(self.clone(), |this| this.find_bytes(value).ok_or(())).ok()
    }

    /// Return a reference to data equal to `value` in this interner, adding it if it does not yet exist.
    ///
    /// Empty slices may not actually be stored.
    fn yoked_find_or_add_bytes(&self, value: &[u8]) -> Yoke<&'static [u8], Self> {
        Yoke::attach_to_cart(self.clone(), |this| this.find_or_add_bytes(value))
    }

    /// Insert data equal to `value` into this interner, returning a reference to it.
    ///
    /// Empty slices may not actually be stored.
    fn yoked_add_bytes(&self, value: &[u8]) -> Yoke<&'static [u8], Self> {
        Yoke::attach_to_cart(self.clone(), |this| this.add_bytes(value))
    }

    /// Insert `value` into this interner, returning a reference to it's data.
    ///
    /// Empty slices may not actually be stored.
    fn yoked_add_owned_bytes(&self, value: Vec<u8>) -> Yoke<&'static [u8], Self> {
        Yoke::attach_to_cart(self.clone(), |this| this.add_owned_bytes(value))
    }

    /// Return a reference to data equal to `value` in this interner, if it exists.
    ///
    /// Empty slices will always succeed and may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{RcInterner, unsync::DataInterner};
    /// # use std::rc::Rc;
    /// let interner: Rc<DataInterner>;
    /// # interner = Rc::new(DataInterner::new());
    /// // ...
    /// let name: &str = "Mary Sue";
    /// let greeting1 = interner.yoked_add_str(&format!("Hello, {name}!"));
    /// let greeting2 = interner.yoked_find_str("Hello, Mary");
    /// let greeting3 = interner.yoked_find_str("Hello, Sue");
    /// drop(interner);
    /// assert_eq!(*greeting1.get(), "Hello, Mary Sue!");
    /// assert_eq!(*greeting2.unwrap().get(), "Hello, Mary");
    /// assert!(matches!(greeting3, None));
    /// ```
    fn yoked_find_str(&self, value: &str) -> Option<Yoke<&'static str, Self>> {
        Yoke::try_attach_to_cart(self.clone(), |this| this.find_str(value).ok_or(())).ok()
    }

    /// Return a reference to data equal to `value` in this interner, adding it if it does not yet exist.
    ///
    /// Empty slices may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{RcInterner, unsync::DataInterner};
    /// # use std::rc::Rc;
    /// let interner: Rc<DataInterner>;
    /// # interner = Rc::new(DataInterner::new());
    /// // ...
    /// let name: &str = "Mary Sue";
    /// let greeting1 = interner.yoked_add_str(&format!("Hello, {name}!"));
    /// let greeting2 = interner.yoked_find_or_add_str("Hello, Mary");
    /// let greeting3 = interner.yoked_find_or_add_str("Hello, Sue");
    /// drop(interner);
    /// assert_eq!(*greeting1.get(), "Hello, Mary Sue!");
    /// assert_eq!(*greeting2.get(), "Hello, Mary");
    /// assert_eq!(*greeting3.get(), "Hello, Sue");
    /// ```
    fn yoked_find_or_add_str(&self, value: &str) -> Yoke<&'static str, Self> {
        Yoke::attach_to_cart(self.clone(), |this| this.find_or_add_str(value))
    }

    /// Insert data equal to `value` into this interner, returning a reference to it.
    ///
    /// Empty slices may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{RcInterner, unsync::DataInterner};
    /// # use std::rc::Rc;
    /// let interner: Rc<DataInterner>;
    /// # interner = Rc::new(DataInterner::new());
    /// // ...
    /// let name: &str = "Ferris";
    /// let greeting = interner.yoked_add_str(&format!("Hello, {name}!"));
    /// drop(interner);
    /// assert_eq!(*greeting.get(), "Hello, Ferris!");
    /// ```
    fn yoked_add_str(&self, value: &str) -> Yoke<&'static str, Self> {
        Yoke::attach_to_cart(self.clone(), |this| this.add_str(value))
    }

    /// Insert `value` into this interner, returning a reference to it's data.
    ///
    /// Empty slices may not actually be stored.
    ///
    /// # Example
    /// ```rust
    /// use interner::{RcInterner, unsync::DataInterner};
    /// # use std::rc::Rc;
    /// let interner: Rc<DataInterner>;
    /// # interner = Rc::new(DataInterner::new());
    /// // ...
    /// let name: &str = "Ferris";
    /// let greeting = interner.yoked_add_owned_string(format!("Hello, {name}!"));
    /// drop(interner);
    /// assert_eq!(*greeting.get(), "Hello, Ferris!");
    /// ```
    fn yoked_add_owned_string(&self, value: String) -> Yoke<&'static str, Self> {
        Yoke::attach_to_cart(self.clone(), |this| this.add_owned_string(value))
    }

    /// Return a reference to data bytewise-equal to `value` in this interner, if it exists and is sufficiently aligned.
    ///
    /// Empty slices and ZSTs will always succeed and may not actually be stored.
    #[cfg(feature = "bytemuck")]
    fn yoked_find_slice<T: NoUninit + 'static>(
        &self,
        value: &[T],
    ) -> Option<Yoke<&'static [T], Self>> {
        Yoke::try_attach_to_cart(self.clone(), |this| this.find_slice(value).ok_or(())).ok()
    }

    /// Return a reference to data bytewise-equal to `value` in this interner, adding it if it does exists or is not sufficiently aligned.
    ///
    /// Empty slices and ZSTs may not actually be stored.
    #[cfg(feature = "bytemuck")]
    fn yoked_find_or_add_slice<T: NoUninit + 'static>(
        &self,
        value: &[T],
    ) -> Yoke<&'static [T], Self> {
        Yoke::attach_to_cart(self.clone(), |this| this.find_or_add_slice(value))
    }

    /// Return a reference to data bytewise-equal to `value` in this interner, adding it if it does exists or is not sufficiently aligned.
    ///
    /// Empty slices and ZSTs may not actually be stored.
    #[cfg(feature = "bytemuck")]
    fn yoked_add_slice<T: NoUninit + 'static>(&self, value: &[T]) -> Yoke<&'static [T], Self> {
        Yoke::attach_to_cart(self.clone(), |this| this.add_slice(value))
    }

    /// Return a reference to data bytewise-equal to `value` in this interner, adding it if it does exists or is not sufficiently aligned.
    ///
    /// This will always succeed if `size_of::<T>() == 0` or `value.capacity() == 0`. Note that in this case a static slice may be returned.
    ///
    /// Otherwise, this will fail if `align_of::<T>() != 1`.
    #[cfg(feature = "bytemuck")]
    fn yoked_try_add_owned<T: NoUninit + 'static>(
        &self,
        value: Vec<T>,
    ) -> Result<Yoke<&'static [T], Self>, Vec<T>> {
        Yoke::try_attach_to_cart(self.clone(), move |this| this.try_add_owned(value))
    }

    /// Return a reference to data bytewise-equal to `value` in this interner, if it exists and is sufficiently aligned.
    ///
    /// Empty slices and ZSTs will always succeed and may not actually be stored.
    #[cfg(feature = "bytemuck")]
    fn yoked_find_value<T: NoUninit + 'static>(&self, value: &T) -> Option<Yoke<&'static T, Self>> {
        Yoke::try_attach_to_cart(self.clone(), move |this| this.find_value(value).ok_or(())).ok()
    }

    /// Return a reference to data bytewise-equal to `value` in this interner, adding it if it does not exist or is not sufficiently aligned.
    ///
    /// ZSTs may not actually be stored.
    #[cfg(feature = "bytemuck")]
    fn yoked_find_or_add_value<T: NoUninit + 'static>(&self, value: &T) -> Yoke<&'static T, Self> {
        Yoke::attach_to_cart(self.clone(), move |this| this.find_or_add_value(value))
    }

    /// Return a reference to data bytewise-equal to `value` in this interner, adding it if it does not exist or is not sufficiently aligned.
    ///
    /// ZSTs may not actually be stored.
    #[cfg(feature = "bytemuck")]
    fn yoked_add_value<T: NoUninit + 'static>(&self, value: &T) -> Yoke<&'static T, Self> {
        Yoke::attach_to_cart(self.clone(), move |this| this.add_value(value))
    }
}

/// Covers Rc<unsync::DataInterner> and Arc<sync::DataInterner>, for example.
#[cfg(feature = "yoke")]
unsafe impl<T: Clone + StableDeref> RcInterner for T where <Self as Deref>::Target: Interner {}
