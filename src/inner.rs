use crate::util::{is_aligned_to, align_offset};


type FullBuf = Box<[u8]>;
type NonFullBuf = Vec<u8>;

#[derive(Default)]
pub(crate) struct DataInternerInner {
    /// SAFETY: No DataBuf in these vecs will be dropped, reallocated, or have initialized parts changed during the designated lifetime.
    /// The DataBufs may be moved between the vecs (from nonfull to full),
    /// and the vecs themselves may be reallocated.
    full_buffers: Vec<FullBuf>,
    nonfull_buffers: Vec<NonFullBuf>,
}

impl DataInternerInner {
    pub(crate) const fn new() -> Self {
        Self { full_buffers: Vec::new(), nonfull_buffers: Vec::new() }
    }

    // SAFETY: Caller must ensure that no references to any buffers exist. E.g. by owning or holding a &mut to the outer interner.
    pub(crate) unsafe fn clear(&mut self) {
        for buffer in &mut self.nonfull_buffers {
            buffer.clear();
        }
        for buffer in self.full_buffers.drain(..) {
            let mut buffer = buffer.into_vec();
            buffer.clear();
            self.nonfull_buffers.push(buffer);
        }
    }

    // SAFETY: Caller must ensure that buffers are not invalidated within the 'a lifetime.
    pub(crate) unsafe fn find_bytes<'a>(&self, value: &[u8]) -> Option<&'a [u8]> {
        // SAFETY: Same as this function. 1 is a power of 2.
        unsafe { self.find_bytes_with_align(value, 1) }
    }

    // SAFETY: Caller must ensure that buffers are not invalidated within the 'a lifetime.
    pub(crate) unsafe fn add_bytes<'a>(&mut self, value: &[u8]) -> &'a [u8] {
        // SAFETY: Same as this function. 1 is a power of 2.
        unsafe { self.add_bytes_with_align(value, 1) }
    }

    // SAFETY: Caller must ensure that buffers are not invalidated within the 'a lifetime.
    pub(crate) unsafe fn add_owned_bytes<'a>(&mut self, value: Vec<u8>) -> &'a [u8] {
        if value.capacity() == 0 {
            // Ignore empty buffers
            debug_assert!(value.is_empty());
            &[]
        } else if value.len() == value.capacity() {
            // Add to full_buffers
            self.full_buffers.push(value.into_boxed_slice());
            let owned: &[u8] = &*self.full_buffers.last().expect("just pushed");
            // SAFETY: The data buffer will never be reallocated
            let owned: &'static [u8] = unsafe { std::mem::transmute(owned) };
            owned
        } else {
            // Add to nonfull_buffers
            self.nonfull_buffers.push(value);
            let owned: &[u8] = &*self.nonfull_buffers.last().expect("just pushed");
            // SAFETY: The data buffer will never be reallocated
            let owned: &'static [u8] = unsafe { std::mem::transmute(owned) };
            owned
        }
    }

    // TODO: Future: Maybe check for prefixes at the end of nonfull buffers.
    // SAFETY: Caller must ensure that buffers are not invalidated within the 'a lifetime.
    pub(crate) unsafe fn find_or_add_bytes<'a>(&mut self, value: &[u8]) -> &'a [u8] {
        // SAFETY: Same safety requirements as this function
        match unsafe { self.find_bytes(value) } {
            Some(owned) => owned,
            // SAFETY: Same safety requirements as this function
            None => unsafe { self.add_bytes(value) },
        }
    }


    // SAFETY: Caller must ensure that buffers are not invalidated within the 'a lifetime, and that align is a power of two.
    pub(crate) unsafe fn find_bytes_with_align<'a>(&self, value: &[u8], align: usize) -> Option<&'a [u8]> {
        // TODO: Maybe use a memchr::memmem::Finder?
        for buf in &*self.full_buffers {
            if let Some(idx) = memchr::memmem::find(buf, value) {
                let owned: &[u8] = &buf[idx..][..value.len()];
                // SAFETY: align is a power of two.
                if unsafe { !is_aligned_to(align, owned.as_ptr()) } { continue; }
                // SAFETY: The string will never be reallocated
                let owned: &'static [u8] = unsafe { std::mem::transmute(owned) };
                return Some(owned);
            }
        }
        for buf in &*self.nonfull_buffers {
            if let Some(idx) = memchr::memmem::find(buf, value) {
                let owned: &[u8] = &buf[idx..][..value.len()];
                // SAFETY: align is a power of two.
                if unsafe { !is_aligned_to(align, owned.as_ptr()) } { continue; }
                // SAFETY: The data buffer will never be reallocated
                let owned: &'static [u8] = unsafe { std::mem::transmute(owned) };
                return Some(owned);
            }
        }
        None
    }

    // SAFETY: Caller must ensure that buffers are not invalidated within the 'a lifetime, and that align is a power of two.
    pub(crate) unsafe fn add_bytes_with_align<'a>(&mut self, value: &[u8], align: usize) -> &'a [u8] {
        for (i, nonfull_buffer) in self.nonfull_buffers.iter_mut().enumerate() {
            // Append to an existing nonfull buffer
            let remaining_capacity = nonfull_buffer.capacity() - nonfull_buffer.len();
            if remaining_capacity < value.len() { continue; }

            let old_len = nonfull_buffer.len();

            let ptr = nonfull_buffer.as_mut_ptr();
            let ptr = ptr.wrapping_add(old_len);

            // SAFETY: align is a power of 2.
            let offset = unsafe { align_offset(align, ptr) };
            if offset > remaining_capacity { continue; }
            if remaining_capacity - offset < value.len() { continue; }

            let fill_len = offset;
            let fill_ptr = ptr;
            let ptr = ptr.wrapping_add(offset);

            unsafe {
                // Prevent having uninit bytes in the init part of the vec
                // SAFETY:
                // * dst must be valid for writes of count * size_of::<T = u8>() bytes. -> ptr comes from a vec with enough remaining capacity
                // * dst must be properly aligned. -> align_of::<u8>() == 1
                std::ptr::write_bytes(fill_ptr, b'\n', fill_len);

                // SAFETY:
                // * src must be valid for reads of count * size_of::<T = u8>() bytes. -> src comes from value with length count
                // * dst must be valid for writes of count * size_of::<T = u8>() bytes. -> ptr comes from a vec with enough remaining capacity
                // * Both src and dst must be properly aligned. -> align_of::<u8>() == 1
                // * The region of memory beginning at src with a size of count * size_of::<T>() bytes must not overlap with the region of memory beginning at dst with the same size.
                //     -> value is a slice of initialized data (that may or may not be owned by this interner),
                //        and ptr points to uninitialized data that this interner owns (and has not exposed), so they cannot overlap.
                std::ptr::copy_nonoverlapping(value.as_ptr(), ptr, value.len());
                nonfull_buffer.set_len(old_len + fill_len + value.len());
            }

            let owned: &[u8] = &nonfull_buffer[old_len + fill_len..][..value.len()];
            // SAFETY: The data buffer will never be reallocated
            let owned: &'static [u8] = unsafe { std::mem::transmute(owned) };

            if nonfull_buffer.len() == nonfull_buffer.capacity() {
                // Move the buffer to full_buffers.
                // SAFETY: Vec::into_boxed_slice does not reallocate it's storage IF the length == the capacity.
                // from docs for std::vec::Vec "If len == capacity, (as is the case for the vec! macro), then a Vec<T> can be converted to and from a Box<[T]>
                // without reallocating or moving the elements.
                let newly_full_buffer = self.nonfull_buffers.swap_remove(i);
                self.full_buffers.push(newly_full_buffer.into_boxed_slice());
            }
            return owned;
        }
        // Add a new buffer
        if align == 1 {
            let vec = if value.len() < 1024 {
                let mut vec = Vec::with_capacity(1024);
                // NOTE: extend_from_slice may reallocate, but that is fine here because this vec is not used anywhere else, and we don't care about alignment.
                vec.extend_from_slice(value);
                vec
            } else {
                Vec::from(value)
            };
            // SAFETY: Same safety requirements as this function
            unsafe { self.add_owned_bytes(vec) }
        } else {
            let mut buffer = Vec::<u8>::with_capacity((value.len() + align - 1).max(1024));
            let capacity = buffer.capacity();

            let ptr = buffer.as_mut_ptr();

            // SAFETY: align is a power of 2.
            let offset = unsafe { align_offset(align, ptr) };
            debug_assert!(offset <= capacity, "we just allocated it with enough space");
            debug_assert!(capacity - offset >= value.len(), "we just allocated it with enough space");

            let fill_len = offset;
            let fill_ptr = ptr;
            let ptr = ptr.wrapping_add(offset);

            unsafe {
                // Prevent having uninit bytes in the init part of the vec.
                // SAFETY:
                // * dst must be valid for writes of count * size_of::<T = u8>() bytes. -> ptr comes from a vec with enough remaining capacity
                // * dst must be properly aligned. -> align_of::<u8>() == 1
                std::ptr::write_bytes(fill_ptr, b'\n', fill_len);

                // SAFETY:
                // * src must be valid for reads of count * size_of::<T = u8>() bytes. -> src comes from value with length count
                // * dst must be valid for writes of count * size_of::<T = u8>() bytes. -> ptr comes from a vec with enough remaining capacity
                // * Both src and dst must be properly aligned. -> align_of::<u8>() == 1
                // * The region of memory beginning at src with a size of count * size_of::<T>() bytes must not overlap with the region of memory beginning at dst with the same size.
                //     -> value is a slice of initialized data (that may or may not be owned by this interner),
                //        and ptr points to uninitialized data that this interner owns (and has not exposed), so they cannot overlap.
                std::ptr::copy_nonoverlapping(value.as_ptr(), ptr, value.len());
                buffer.set_len(fill_len + value.len());
            }

            let owned: &[u8] = &buffer[fill_len..][..value.len()];
            // SAFETY: The data buffer will never be reallocated
            let owned: &'static [u8] = unsafe { std::mem::transmute(owned) };

            if buffer.len() == buffer.capacity() {
                // Move the buffer to full_buffers. (The buffer may be full if the maximum alignment fill offset was required)
                // SAFETY: Vec::into_boxed_slice does not reallocate it's storage IF the length == the capacity.
                // from docs for std::vec::Vec "If len == capacity, (as is the case for the vec! macro), then a Vec<T> can be converted to and from a Box<[T]>
                // without reallocating or moving the elements.
                self.full_buffers.push(buffer.into_boxed_slice());
            } else {
                // Move the buffer to nonfull_buffers.
                // SAFETY: moving a vec does not reallocate it's storage.
                self.nonfull_buffers.push(buffer);
            }
            owned
        }
    }

    // TODO: Future: Maybe check for prefixes at the end of nonfull buffers.
    // SAFETY: Caller must ensure that buffers are not invalidated within the 'a lifetime, and align is a power of 2.
    #[cfg_attr(not(feature = "bytemuck"), allow(dead_code))]
    pub(crate) unsafe fn find_or_add_bytes_with_align<'a>(&mut self, value: &[u8], align: usize) -> &'a [u8] {
        // SAFETY: Same safety requirements as this function
        match unsafe { self.find_bytes_with_align(value, align) } {
            Some(owned) => owned,
            // SAFETY: Same safety requirements as this function
            None => unsafe { self.add_bytes_with_align(value, align) },
        }
    }
}
