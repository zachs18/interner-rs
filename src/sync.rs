
use crate::{inner::DataInternerInner, unsync::DataInterner as UnSyncDataInterner, util::Interner};
use std::cell::RefCell;

#[cfg(not(feature = "parking_lot"))]
pub(crate) use crate::util::RwLock;
#[cfg(feature = "parking_lot")]
pub(crate) use parking_lot::RwLock;

#[cfg(feature = "bytemuck")]
use std::{mem::size_of, ptr::NonNull};
#[cfg(feature = "bytemuck")]
use bytemuck::{NoUninit, cast_slice, try_cast_vec};

/// A thread-safe data interner.
/// 
/// You can hold a reference to interned data as long as you hold a reference to the interner.
/// 
/// With the `yoke` feature enabled, you can additionally acquire a [`Yoke<T, Arc<DataInterner>>`](yoke::Yoke), where `T` is the interned data, which allows holding a reference to the interned data along with a ref-counted interner.
/// 
/// By default, byte slices (`[u8]`) and string slices ([`str`]) can be interned.
/// 
/// With the `bytemuck` feature enabled, you can additionally intern slices of any type that implmements [`NoUninit`](bytemuck::NoUninit) (i.e. types that are [`Copy`] and `'static` and contain no padding).
/// 
/// Byte vectors ([`Vec<u8>`]) and strings ([`String`]) can be inserted into the interner, which may use their excess capacity for additional interned data.
/// 
/// With the `bytemuck` feature enabled, you can additionally insert vectors of any type that implmements [`NoUninit`](bytemuck::NoUninit) such that `align_of::<T>() == 1`. See the documentation for [`try_add_owned`](DataInterner::try_add_owned) for more information.
#[derive(Default)]
pub struct DataInterner {
    pub(crate) inner: RwLock<DataInternerInner>,
}

impl DataInterner {
    /// Constructs a new, empty `DataInterner`.
    /// 
    /// The interner will not allocate until something is added to it.
    /// 
    /// If the `parking_lot` feature is enabled, this function is `const`.
    #[cfg(feature = "parking_lot")]
    pub const fn new() -> Self {
        Self {
            inner: RwLock::new(DataInternerInner::new()),
        }
    }

    /// Constructs a new, empty `DataInterner`.
    /// 
    /// The interner will not allocate until something is added to it.
    /// 
    /// If the `parking_lot` feature is enabled, this function is `const`.
    #[cfg(not(feature = "parking_lot"))]
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(DataInternerInner::new()),
        }
    }

    /// Convert this to a non-thread-safe interner without deallocating or removing data.
    /// 
    /// This function will still invalidate all references, since it takes `self` by value.
    /// 
    /// # Example
    /// ```rust
    /// use interner::{Interner, sync::DataInterner};
    /// let interner: DataInterner;
    /// # interner = DataInterner::new();
    /// // ...
    /// let name: &str = "Ferris";
    /// let greeting1 = interner.add_str(&format!("Hello, {name}!"));
    /// let greeting2 = interner.find_str("Hello, Ferris!");
    /// assert_eq!(greeting1, "Hello, Ferris!");
    /// assert_eq!(greeting2, Some("Hello, Ferris!"));
    /// let interner = interner.into_unsync();
    /// let greeting3 = interner.find_str("Hello, Ferris!");
    /// assert_eq!(greeting3, Some("Hello, Ferris!"));
    /// ```
    pub fn into_unsync(self) -> UnSyncDataInterner {
        let inner = self.inner.into_inner();
        UnSyncDataInterner {
            inner: RefCell::new(inner),
        }
    }
    
    /// Clear all data held by this interner without deallocating.
    /// 
    /// This function is safe because it takes a &mut self, which guarantees no other references exist into data held by this interner.
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
    /// interner.clear();
    /// let greeting3 = interner.find_str("Hello, Ferris!");
    /// assert_eq!(greeting3, None);
    /// ```
    pub fn clear(&mut self) {
        let this = self.inner.get_mut();
        // SAFETY: We hold a &mut self.
        unsafe { this.clear() }
    }
}

unsafe impl Interner for DataInterner {
    /// Clear all data held by this interner without deallocating.
    /// 
    /// This function is safe because it takes a &mut self, which guarantees no other references exist into data held by this interner.
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
    fn try_clear(&mut self) -> Result<(), ()> {
        let this = self.inner.get_mut();
        // SAFETY: We hold a &mut self.
        unsafe { this.clear() }
        Ok(())
    }

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
    /// let greeting2 = interner.find_bytes(b"Hello, Mary");
    /// let greeting3 = interner.find_bytes(b"Hello, Sue");
    /// assert_eq!(greeting1, "Hello, Mary Sue!");
    /// assert_eq!(greeting2, Some(b"Hello, Mary" as &[u8]));
    /// assert_eq!(greeting3, None);
    /// ```
    fn find_bytes(&self, value: &[u8]) -> Option<&[u8]> {
        if value.is_empty() {
            return Some(&[]);
        }
        let this = self.inner.read();
        // SAFETY: self is borrowed immutably for the '_ lifetime, so no buffer will be invalidated in that lifetime.
        unsafe { this.find_bytes(value) }
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
    /// let greeting2 = interner.find_or_add_bytes(b"Hello, Mary");
    /// let greeting3 = interner.find_or_add_bytes(b"Hello, Sue");
    /// assert_eq!(greeting1, "Hello, Mary Sue!");
    /// assert_eq!(greeting2, b"Hello, Mary");
    /// assert_eq!(greeting3, b"Hello, Sue");
    /// ```
    fn find_or_add_bytes(&self, value: &[u8]) -> &[u8] {
        if value.is_empty() {
            return &[];
        }
        let mut this = self.inner.write();
        // SAFETY: self is borrowed immutably for the '_ lifetime, so no buffer will be invalidated in that lifetime.
        unsafe { this.find_or_add_bytes(value) }
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
    /// let greeting = interner.add_bytes(format!("Hello, {name}!").as_bytes());
    /// assert_eq!(greeting, b"Hello, Ferris!");
    /// ```
    fn add_bytes(&self, value: &[u8]) -> &[u8] {
        if value.is_empty() {
            return &[];
        }
        let mut this = self.inner.write();
        // SAFETY: self is borrowed immutably for the '_ lifetime, so no buffer will be invalidated in that lifetime.
        unsafe { this.add_bytes(value) }
    }

    /// Insert `value` into this interner, returning a reference to it's data.
    /// 
    /// If `value.capacity() == 0`, the value may not actually be added, and a static slice may be returned.
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
    fn add_owned_bytes(&self, value: Vec<u8>) -> &[u8] {
        if value.capacity() == 0 {
            // Ignore empty buffers
            debug_assert!(value.is_empty());
            &[]
        } else {
            let mut this = self.inner.write();
            // SAFETY: self is borrowed immutably for the '_ lifetime, so no buffer will be invalidated in that lifetime.
            unsafe { this.add_owned_bytes(value) }
        }

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
    fn find_slice<T: NoUninit + 'static>(&self, value: &[T]) -> Option<&[T]> {
        if value.is_empty() {
            // Ignore empty slices
            Some(&[])
        } else if size_of::<T>() == 0 {
            // Ignore ZSTs
            // SAFETY: T is a ZST
            unsafe {
                let ptr = NonNull::dangling();
                Some(std::slice::from_raw_parts(ptr.as_ptr(), value.len()))
            }
        } else {
            let len = value.len();
            let value: &[u8] = cast_slice(value);
            let align = std::mem::align_of::<T>();
            let this = self.inner.read();
            // SAFETY: self is borrowed immutably for the '_ lifetime, so no buffer will be invalidated in that lifetime.
            // SAFETY: align is a power of two.
            let owned = unsafe { this.find_bytes_with_align(value, align)? };
            // This would require T: AnyBitPattern, but that is more restrictive than necessary, since we know the bit pattern matches the original
            // Some(cast_slice(owned))
            unsafe {
                let ptr = owned.as_ptr();
                let ptr = ptr as *const T;
                // SAFETY: T is Copy has no interior mutability, and ptr points to equal bytes as value did.
                Some(std::slice::from_raw_parts(ptr, len))
            }
        }
    }

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
    fn find_or_add_slice<T: NoUninit + 'static>(&self, value: &[T]) -> &[T] {
        if value.is_empty() {
            // Ignore empty slices
            &[]
        } else if size_of::<T>() == 0 {
            // Ignore ZSTs
            // SAFETY: T is a ZST
            unsafe {
                let ptr = NonNull::dangling();
                std::slice::from_raw_parts(ptr.as_ptr(), value.len())
            }
        } else {
            let len = value.len();
            let value: &[u8] = cast_slice(value);
            let align = std::mem::align_of::<T>();
            let mut this = self.inner.write();
            // SAFETY: self is borrowed immutably for the '_ lifetime, so no buffer will be invalidated in that lifetime.
            // SAFETY: align is a power of two.
            let owned = unsafe { this.find_or_add_bytes_with_align(value, align) };
            // This would require T: AnyBitPattern, but that is more restrictive than necessary, since we know the bit pattern matches the original
            // cast_slice(owned)
            unsafe {
                let ptr = owned.as_ptr();
                let ptr = ptr as *const T;
                // SAFETY: T is Copy has no interior mutability, and ptr points to equal bytes as value did.
                std::slice::from_raw_parts(ptr, len)
            }
        }
    }

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
    fn add_slice<T: NoUninit + 'static>(&self, value: &[T]) -> &[T] {
        if value.is_empty() {
            // Ignore empty slices
            &[]
        } else if size_of::<T>() == 0 {
            // Ignore ZSTs
            // SAFETY: T is a ZST
            unsafe {
                let ptr = NonNull::dangling();
                std::slice::from_raw_parts(ptr.as_ptr(), value.len())
            }
        } else {
            let len = value.len();
            let value: &[u8] = cast_slice(value);
            let align = std::mem::align_of::<T>();
            let mut this = self.inner.write();
            // SAFETY: self is borrowed immutably for the '_ lifetime, so no buffer will be invalidated in that lifetime.
            // SAFETY: align is a power of two.
            let owned = unsafe { this.add_bytes_with_align(value, align) };
            // This would require T: AnyBitPattern, but that is more restrictive than necessary, since we know the bit pattern matches the original
            // cast_slice(owned)
            unsafe {
                let ptr = owned.as_ptr();
                let ptr = ptr as *const T;
                // SAFETY: T is Copy has no interior mutability, and ptr points to equal bytes as value did.
                std::slice::from_raw_parts(ptr, len)
            }
        }
    }

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
    fn try_add_owned<T: NoUninit + 'static>(&self, value: Vec<T>) -> Result<&[T], Vec<T>> {
        if value.capacity() == 0 {
            // Ignore empty buffers
            debug_assert!(value.is_empty());
            Ok(&[])
        } else if std::mem::size_of::<T>() == 0 {
            // Ignore ZSTs
            // SAFETY: T is a ZST
            unsafe {
                let ptr = NonNull::dangling();
                Ok(std::slice::from_raw_parts(ptr.as_ptr(), value.len()))
            }
        } else {
            let len = value.len();
            let value = match try_cast_vec(value) {
                Ok(value) => value,
                Err((_, value)) => return Err(value),
            };
            let owned = self.add_owned_bytes(value);
            // This would require T: AnyBitPattern, but that is more restrictive than necessary, since we know the bit pattern matches the original
            // cast_slice(owned)
            unsafe {
                let ptr = owned.as_ptr();
                let ptr = ptr as *const T;
                // SAFETY: T is Copy has no interior mutability, and ptr points to the same address as value did.
                Ok(std::slice::from_raw_parts(ptr, len))
            }
        }
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
    /// let value2 = interner.find_value(&0x55555555u32);
    /// let value3 = interner.find_value(&0x5555u32);
    /// assert_eq!(value1, &[0x55; 7]);
    /// assert_eq!(value2, Some(&0x55555555u32));
    /// assert_eq!(value3, None);
    /// ```
    #[cfg(feature = "bytemuck")]
    fn find_value<T: NoUninit + 'static>(&self, value: &T) -> Option<&T> {
        if size_of::<T>() == 0 {
            // Ignore ZSTs
            // SAFETY: T is a ZST
            unsafe {
                let ptr = NonNull::dangling();
                Some(&*ptr.as_ptr())
            }
        } else {
            let slice = unsafe {
                let ptr = value as *const T;
                // SAFETY: ptr is valid for size_of::<T>() bytes for reads 
                std::slice::from_raw_parts(ptr, 1)
            };
            Some(&self.find_slice(slice)?[0])
        }
    }

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
    fn find_or_add_value<T: NoUninit + 'static>(&self, value: &T) -> &T {
        if size_of::<T>() == 0 {
            // Ignore ZSTs
            // SAFETY: T is a ZST
            unsafe {
                let ptr = NonNull::dangling();
                &*ptr.as_ptr()
            }
        } else {
            let slice = unsafe {
                let ptr = value as *const T;
                // SAFETY: ptr is valid for size_of::<T>() bytes for reads 
                std::slice::from_raw_parts(ptr, 1)
            };
            &self.find_or_add_slice(slice)[0]
        }
    }

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
    fn add_value<T: NoUninit + 'static>(&self, value: &T) -> &T {
        if size_of::<T>() == 0 {
            // Ignore ZSTs
            // SAFETY: T is a ZST
            unsafe {
                let ptr = NonNull::dangling();
                &*ptr.as_ptr()
            }
        } else {
            let slice = unsafe {
                let ptr = value as *const T;
                // SAFETY: ptr is valid for size_of::<T>() bytes for reads 
                std::slice::from_raw_parts(ptr, 1)
            };
            &self.add_slice(slice)[0]
        }
    }
}

macro_rules! make_inherent_impls {
    (impl $ty:ty {
        $( $(#[cfg($($cfg:tt)*)])? $vis:vis fn $func:ident $([ $($generics:tt)* ])? (&self, value: $valty:ty) -> $retty:ty;)*
    }) => {
        impl $ty {
            $(
                $(#[cfg($($cfg)*)])?
                #[doc = concat!("See [`Interner::", stringify!($func), "`]")]
                $(#[cfg_attr(feature = "doc_cfg", doc(cfg($($cfg)*)))])?
                $vis fn $func $(< $($generics)* >)? (&self, value: $valty) -> $retty {
                    <Self as Interner>::$func(self, value)
                }
            )*
        }
    };
}

make_inherent_impls!{
    impl DataInterner {
        pub fn find_bytes(&self, value: &[u8]) -> Option<&[u8]>;
        pub fn find_or_add_bytes(&self, value: &[u8]) -> &[u8];
        pub fn add_bytes(&self, value: &[u8]) -> &[u8];
        pub fn add_owned_bytes(&self, value: Vec<u8>) -> &[u8];

        pub fn find_str(&self, value: &str) -> Option<&str>;
        pub fn find_or_add_str(&self, value: &str) -> &str;
        pub fn add_str(&self, value: &str) -> &str;
        pub fn add_owned_string(&self, value: String) -> &str;

        #[cfg(feature = "bytemuck")]
        pub fn find_slice[T: NoUninit + 'static](&self, value: &[T]) -> Option<&[T]>;
        #[cfg(feature = "bytemuck")]
        pub fn find_or_add_slice[T: NoUninit + 'static](&self, value: &[T]) -> &[T];
        #[cfg(feature = "bytemuck")]
        pub fn add_slice[T: NoUninit + 'static](&self, value: &[T]) -> &[T];
        #[cfg(feature = "bytemuck")]
        pub fn try_add_owned[T: NoUninit + 'static](&self, value: Vec<T>) -> Result<&[T], Vec<T>>;
    }
}
