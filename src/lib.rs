#![cfg_attr(feature = "doc_cfg", feature(doc_cfg))]
#![deny(unsafe_op_in_unsafe_fn)]
//! `interner` provides two data interner types, [`unsync::DataInterner`] and [`sync::DataInterner`].
//! A `DataInterner` can store byte slices, string slices, and (with the `bytemuck` feature enabled) slices and values of [`bytemuck::NoUninit`] types.
//!
//! The core API is essentially this:
//!
#![cfg_attr(doctest, doc = " ````no_test")] // see https://github.com/rust-lang/rust/issues/63193#issuecomment-1053702113
//! ```ignore
//! impl sync::DataInterner {
//!     pub /* #[cfg(feature = "parking_lot")] const */ fn new() -> Self;
//!     pub fn into_unsync(self) -> unsync::DataInterner;
//!     pub fn clear(&mut self);
//! }
//! impl unsync::DataInterner {
//!     pub const fn new() -> Self;
//!     pub fn into_sync(self) -> sync::DataInterner;
//!     pub fn clear(&mut self);
//! }
//! trait Interner {
//!     pub fn try_clear(&mut self) -> Result<(), ()>;
//!
//!     pub fn find_bytes(&self, value: &[u8]) -> Option<&[u8]>;
//!     pub fn find_or_add_bytes(&self, value: &[u8]) -> &[u8];
//!     pub fn add_bytes(&self, value: &[u8]) -> &[u8];
//!     pub fn add_owned_bytes(&self, value: Vec<u8>) -> &[u8];
//!
//!     pub fn find_str(&self, value: &str) -> Option<&str>;
//!     pub fn find_or_add_str(&self, value: &str) -> &str;
//!     pub fn add_str(&self, value: &str) -> &str;
//!     pub fn add_owned_string(&self, value: String) -> &str;
//! }
//! #[cfg(feature = "bytemuck")]
//! trait Interner {
//!     pub fn find_slice<T: NoUninit>(&self, value: &[T]) -> Option<&[T]>;
//!     pub fn find_or_add_slice<T: NoUninit>(&self, value: &[T]) -> &[T];
//!     pub fn add_slice<T: NoUninit>(&self, value: &[T]) -> &[T];
//!     pub fn try_add_owned<T: NoUninit>(&self, value: Vec<T>) -> Result<&[T], Vec<T>>;
//!
//!     pub fn find_value<T: NoUninit>(&self, value: &T) -> Option<&T>;
//!     pub fn find_or_add_value<T: NoUninit>(&self, value: &T) -> &T;
//!     pub fn add_value<T: NoUninit>(&self, value: &T) -> &T;
//! }
//! #[cfg(feature = "yoke")]
//! trait RcInterner: Clone + StableDeref + Deref<Target: Interner> {
//!     // This trait is implemented for (e.g.) Rc<unsync::DataInterner> and Arc<sync::DataInterner>.
//!     // The above functions, but put the return value in a Yoke<_, Self>
//!     pub fn yoked_find_bytes(&self, value: &[u8]) -> Option<Yoke<&'static [u8], Self>>;
//!     pub fn yoked_...(&self, value: &...) -> Yoke<&'static ..., Self>; // etc.
//! }
//! ```
//! ````

#[cfg(feature = "builder")]
pub mod builder;
pub(crate) mod inner;
pub mod sync;
pub mod unsync;

mod util;
pub use util::Interner;

#[cfg(feature = "yoke")]
pub use util::RcInterner;
