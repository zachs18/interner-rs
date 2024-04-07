use std::{marker::PhantomData, mem, ops, ptr};

use bytemuck::NoUninit;

use crate::util::{align_offset, Interner};

pub struct SliceBuilder<'a, T: NoUninit, I: Interner> {
    // Capacity should always be size_of::<T>() * self.cap + (align_of::<T>() - 1)
    data: Vec<u8>,
    // Byte index into data
    start: usize,
    // Element length
    len: usize,
    // Element capacity
    cap: usize,
    interner: &'a I,
    _phantom: PhantomData<Vec<T>>,
}

impl<'a, T: NoUninit, I: Interner> SliceBuilder<'a, T, I> {
    pub fn new(interner: &'a I) -> Self {
        let cap = if mem::size_of::<T>() == 0 {
            usize::MAX
        } else {
            0
        };
        Self {
            data: vec![],
            start: 0,
            len: 0,
            cap,
            interner,
            _phantom: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        if mem::size_of::<T>() == 0 || self.cap == 0 {
            return ptr::NonNull::dangling().as_ptr();
        }
        self.data.as_mut_ptr().wrapping_add(self.start).cast()
    }

    pub fn as_ptr(&self) -> *const T {
        if mem::size_of::<T>() == 0 || self.cap == 0 {
            return ptr::NonNull::dangling().as_ptr();
        }
        self.data.as_ptr().wrapping_add(self.start).cast()
    }

    pub fn reserve(&mut self, additional: usize) {
        let requested_cap = self.len.checked_add(additional).expect("capacity overflow");
        let additional_cap = match requested_cap.checked_sub(self.cap) {
            None | Some(0) => return, // Already sufficient
            Some(additional_cap) => additional_cap,
        };
        let additional_raw_cap = additional_cap
            .checked_mul(mem::size_of::<T>())
            .expect("capacity overflow");
        if mem::size_of::<T>() == 0 {
            // ZST already has max capacity
            panic!("capacity overflow");
        }

        if mem::align_of::<T>() == 1 {
            // Just multiply and divide, no offsetting needed
            debug_assert!(self.start == 0);
            self.data.reserve(additional_raw_cap);
            self.cap = self.data.capacity() / mem::size_of::<T>();
            debug_assert!(self.cap >= requested_cap);
            return;
        } else if self.data.capacity() == 0 {
            // Initial allocation, allocate alignment padding
            self.data
                .reserve(additional_raw_cap + mem::align_of::<T>() - 1);
            let ptr = self.data.as_mut_ptr();
            // SAFETY: align_of is a power of two
            let align_offset = unsafe { align_offset(mem::align_of::<T>(), ptr) };
            self.start = align_offset;
            self.cap = (self.data.capacity() - align_offset) / mem::size_of::<T>();
            debug_assert!(
                self.cap >= requested_cap,
                "{} >= {} (raw_cap = {}, align_offset = {align_offset})",
                self.cap,
                requested_cap,
                self.data.capacity()
            );
            unsafe {
                // Prevent having uninit bytes in the initial part of the vec
                // SAFETY: ptr points to at least (additional_raw_cap + mem::align_of::<T>() - 1) bytes
                // and that > align_offset (since align_offset < mem::align_of::<T>())
                ptr.write_bytes(0, align_offset);
                self.data.set_len(align_offset);
            }
        } else {
            // Subsequent allocation, do not re-allocate alignment padding, but may need to shuffle bytes around
            // if old padding is not the same as new padding
            self.data.reserve(additional_raw_cap);
            let ptr = self.data.as_mut_ptr();
            // SAFETY: align_of is a power of two
            let new_align_offset = unsafe { align_offset(mem::align_of::<T>(), ptr) };
            if self.start != new_align_offset {
                // Re-align Ts
                let src = ptr.wrapping_add(self.start);
                let dst = ptr.wrapping_add(new_align_offset);
                // SAFETY: the vec is big enough that the padding cause this to go past the end.
                unsafe {
                    dst.copy_from(src, self.len * mem::size_of::<T>());
                }

                unsafe {
                    // Prevent having uninit bytes in the initial part of the vec
                    // SAFETY: ptr points to at least (additional_raw_cap + mem::align_of::<T>() - 1) bytes
                    // and that > align_offset (since align_offset < mem::align_of::<T>())
                    ptr.write_bytes(0, new_align_offset);

                    self.data
                        .set_len(new_align_offset + self.len * mem::size_of::<T>());
                }

                self.start = new_align_offset;
            }
            self.cap = (self.data.capacity() - new_align_offset) / mem::size_of::<T>();
            debug_assert!(self.cap >= requested_cap);
        }
    }

    pub fn extend_from_slice(&mut self, slice: &[T]) {
        let additional = slice.len();
        if self.len + additional > self.cap {
            self.reserve(additional);
        }
    }

    pub fn finalize(self) -> &'a [T] {
        if mem::size_of::<T>() == 0 {
            let ptr = ptr::NonNull::dangling().as_ptr();
            // SAFETY: ZSTs can dangle
            unsafe {
                return std::slice::from_raw_parts(ptr, self.len);
            }
        }
        if self.capacity() == 0 {
            return &[];
        }
        let data = self.interner.add_owned_bytes(self.data);
        let data = &data[self.start..];
        unsafe {
            // SAFETY: if self is not empty, &data[start] is aligned for T and is valid for reads for self.len * size_of::<T>() bytes
            let ptr = data.as_ptr().cast();
            std::slice::from_raw_parts(ptr, self.len)
        }
    }

    pub fn push(&mut self, value: T) {
        if self.len == self.cap {
            // Reallocate
            self.reserve(1);
        }

        let old_raw_len = self.start + self.len * mem::size_of::<T>();
        let new_raw_len = old_raw_len + mem::size_of::<T>();
        // SAFETY: ptr is valid for size_of::<T>() bytes write and is aligned.
        unsafe {
            let ptr = self.data.as_mut_ptr().wrapping_add(old_raw_len);
            std::ptr::write(ptr.cast(), value)
        }

        self.len += 1;
        // SAFETY: we wrote to the added bytes
        unsafe {
            self.data.set_len(new_raw_len);
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        self.len.checked_sub(1).map(|new_len| {
            self.len = new_len;
            let new_raw_len = self.start + self.len * mem::size_of::<T>();
            let data = &self.data[new_raw_len..][..mem::size_of::<T>()];
            // SAFETY: data is valid for size_of::<T>() bytes read and is aligned.
            let value = unsafe { std::ptr::read(data.as_ptr() as *const T) };
            // SAFETY: reducing length
            unsafe {
                self.data.set_len(new_raw_len);
            }
            value
        })
    }
}

impl<'a, T: NoUninit, I: Interner> ops::Deref for SliceBuilder<'a, T, I> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        if mem::size_of::<T>() == 0 {
            let ptr = ptr::NonNull::dangling().as_ptr();
            // SAFETY: ZSTs can dangle
            unsafe { std::slice::from_raw_parts(ptr, self.len) }
        } else if self.len == 0 {
            &[]
        } else {
            let data = &self.data[self.start..];
            unsafe {
                // SAFETY: if self is not empty, &data[start] is aligned for T and is valid for reads for self.len * size_of::<T>() bytes
                let ptr = data.as_ptr().cast();
                std::slice::from_raw_parts(ptr, self.len)
            }
        }
    }
}

impl<'a, T: NoUninit, I: Interner> ops::DerefMut for SliceBuilder<'a, T, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if mem::size_of::<T>() == 0 {
            let ptr = ptr::NonNull::dangling().as_ptr();
            // SAFETY: ZSTs can dangle
            unsafe { std::slice::from_raw_parts_mut(ptr, self.len) }
        } else if self.len == 0 {
            &mut []
        } else {
            let data = &mut self.data[self.start..];
            unsafe {
                // SAFETY: &data[start] is aligned for T and is valid for reads for self.len * size_of::<T>() bytes
                let ptr = data.as_mut_ptr().cast();
                std::slice::from_raw_parts_mut(ptr, self.len)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::unsync::DataInterner;

    use super::SliceBuilder;

    #[test]
    fn usize() {
        let interner = DataInterner::new();
        let mut slice_builder = SliceBuilder::<usize, _>::new(&interner);
        slice_builder.push(0);
        slice_builder.push(0x5555555555555555);
        slice_builder.push(0xAAAAAAAAAAAAAAAA);
        slice_builder.push(0);
        slice_builder.push(0);
        slice_builder.push(0x5555555555555555);
        slice_builder.push(0xAAAAAAAAAAAAAAAA);
        slice_builder.push(0);

        let slice = slice_builder.finalize();
        assert_eq!(
            slice,
            [
                0,
                0x5555555555555555,
                0xAAAAAAAAAAAAAAAA,
                0,
                0,
                0x5555555555555555,
                0xAAAAAAAAAAAAAAAA,
                0
            ]
        );
    }
}
