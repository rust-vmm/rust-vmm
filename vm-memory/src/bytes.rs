// Portions Copyright 2019 Red Hat, Inc.
//
// Portions Copyright 2018 Amazon.com, Inc. or its affiliates. All Rights Reserved.
//
// Portions Copyright 2017 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE-BSD-3-Clause file.
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

//! Define the `AtomicAccess` and `Bytes` traits.

use std::result::Result;
use std::sync::atomic::Ordering;
use zerocopy::{FromBytes, FromZeros, IntoBytes};

use crate::atomic_integer::AtomicInteger;
use crate::{ReadVolatile, WriteVolatile};

/// A trait used to identify types which can be accessed atomically by proxy.
pub trait AtomicAccess:
    Copy + Send + Sync + FromBytes + IntoBytes + FromZeros
    // Could not find a more succinct way of stating that `Self` can be converted
    // into `Self::A::V`, and the other way around.
    + From<<<Self as AtomicAccess>::A as AtomicInteger>::V>
    + Into<<<Self as AtomicAccess>::A as AtomicInteger>::V>
{
    /// The `AtomicInteger` that atomic operations on `Self` are based on.
    type A: AtomicInteger;
}

macro_rules! impl_atomic_access {
    ($T:ty, $A:path) => {
        impl AtomicAccess for $T {
            type A = $A;
        }
    };
}

impl_atomic_access!(i8, std::sync::atomic::AtomicI8);
impl_atomic_access!(i16, std::sync::atomic::AtomicI16);
impl_atomic_access!(i32, std::sync::atomic::AtomicI32);
#[cfg(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "powerpc64",
    target_arch = "s390x",
    target_arch = "riscv64"
))]
impl_atomic_access!(i64, std::sync::atomic::AtomicI64);

impl_atomic_access!(u8, std::sync::atomic::AtomicU8);
impl_atomic_access!(u16, std::sync::atomic::AtomicU16);
impl_atomic_access!(u32, std::sync::atomic::AtomicU32);
#[cfg(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "powerpc64",
    target_arch = "s390x",
    target_arch = "riscv64"
))]
impl_atomic_access!(u64, std::sync::atomic::AtomicU64);

impl_atomic_access!(isize, std::sync::atomic::AtomicIsize);
impl_atomic_access!(usize, std::sync::atomic::AtomicUsize);

/// A container to host a range of bytes and access its content.
///
/// Candidates which may implement this trait include:
/// - anonymous memory areas
/// - mmapped memory areas
/// - data files
/// - a proxy to access memory on remote
pub trait Bytes<A> {
    /// Associated error codes
    type E;

    /// Writes a slice into the container at `addr`.
    ///
    /// Returns the number of bytes written. The number of bytes written can
    /// be less than the length of the slice if there isn't enough room in the
    /// container.
    ///
    /// If the given slice is empty (e.g. has length 0), always returns `Ok(0)`, even if `addr`
    /// is otherwise out of bounds. However, if the container is empty, it will
    /// return an error (unless the slice is also empty, in which case the above takes precedence).
    ///
    /// ```rust
    /// # use vm_memory::{Bytes, VolatileMemoryError, VolatileSlice};
    /// # use matches::assert_matches;
    /// let mut arr = [1, 2, 3, 4, 5];
    /// let slice = VolatileSlice::from(arr.as_mut_slice());
    ///
    /// assert_eq!(slice.write(&[1, 2, 3], 0).unwrap(), 3);
    /// assert_eq!(slice.write(&[1, 2, 3], 3).unwrap(), 2);
    /// assert_matches!(
    ///     slice.write(&[1, 2, 3], 5).unwrap_err(),
    ///     VolatileMemoryError::OutOfBounds { addr: 5 }
    /// );
    /// assert_eq!(slice.write(&[], 5).unwrap(), 0);
    /// ```
    fn write(&self, buf: &[u8], addr: A) -> Result<usize, Self::E>;

    /// Reads data from the container at `addr` into a slice.
    ///
    /// Returns the number of bytes read. The number of bytes read can be less than the length
    /// of the slice if there isn't enough data within the container.
    ///
    /// If the given slice is empty (e.g. has length 0), always returns `Ok(0)`, even if `addr`
    /// is otherwise out of bounds. However, if the container is empty, it will
    /// return an error (unless the slice is also empty, in which case the above takes precedence).
    fn read(&self, buf: &mut [u8], addr: A) -> Result<usize, Self::E>;

    /// Writes the entire content of a slice into the container at `addr`.
    ///
    /// If the given slice is empty (e.g. has length 0), always returns `Ok(0)`, even if `addr`
    /// is otherwise out of bounds.
    ///
    /// # Errors
    ///
    /// Returns an error if there isn't enough space within the container to write the entire slice.
    /// Part of the data may have been copied nevertheless.
    fn write_slice(&self, buf: &[u8], addr: A) -> Result<(), Self::E>;

    /// Reads data from the container at `addr` to fill an entire slice.
    ///
    /// If the given slice is empty (e.g. has length 0), always returns `Ok(0)`, even if `addr`
    /// is otherwise out of bounds.
    ///
    /// # Errors
    ///
    /// Returns an error if there isn't enough data within the container to fill the entire slice.
    /// Part of the data may have been copied nevertheless.
    fn read_slice(&self, buf: &mut [u8], addr: A) -> Result<(), Self::E>;

    /// Writes an object into the container at `addr`.
    ///
    /// # Errors
    ///
    /// Returns an error if the object doesn't fit inside the container.
    fn write_obj<T: Copy + Send + Sync + FromBytes + IntoBytes + FromZeros>(
        &self,
        mut val: T,
        addr: A,
    ) -> Result<(), Self::E> {
        self.write_slice(val.as_mut_bytes(), addr)
    }

    /// Reads an object from the container at `addr`.
    ///
    /// Reading from a volatile area isn't strictly safe as it could change mid-read.
    /// However, as long as the type T is plain old data and can handle random initialization,
    /// everything will be OK.
    ///
    /// # Errors
    ///
    /// Returns an error if there's not enough data inside the container.
    fn read_obj<T: Copy + Send + Sync + FromBytes + IntoBytes + FromZeros>(
        &self,
        addr: A,
    ) -> Result<T, Self::E> {
        let mut result = T::new_zeroed();
        self.read_slice(result.as_mut_bytes(), addr).map(|_| result)
    }

    /// Reads up to `count` bytes from `src` and writes them into the container at `addr`.
    /// Unlike `VolatileRead::read_volatile`, this function retries on `EINTR` being returned from
    /// the underlying I/O `read` operation.
    ///
    /// Returns the number of bytes written into the container.
    ///
    /// # Arguments
    /// * `addr` - Begin writing at this address.
    /// * `src` - Copy from `src` into the container.
    /// * `count` - Copy `count` bytes from `src` into the container.
    ///
    /// # Examples
    ///
    /// * Read bytes from /dev/urandom (uses the `backend-mmap` feature)
    ///
    /// ```
    /// # #[cfg(all(feature = "backend-mmap", feature = "rawfd"))]
    /// # {
    /// # use vm_memory::{Address, GuestMemoryBackend, Bytes, GuestAddress, GuestMemoryMmap};
    /// # use std::fs::File;
    /// # use std::path::Path;
    /// #
    /// # let start_addr = GuestAddress(0x1000);
    /// # let gm = GuestMemoryMmap::<()>::from_ranges(&vec![(start_addr, 0x400)])
    /// #    .expect("Could not create guest memory");
    /// # let addr = GuestAddress(0x1010);
    /// # let mut file = if cfg!(target_family = "unix") {
    /// let mut file = File::open(Path::new("/dev/urandom")).expect("Could not open /dev/urandom");
    /// #   file
    /// # } else {
    /// #   File::open(Path::new("c:\\Windows\\system32\\ntoskrnl.exe"))
    /// #       .expect("Could not open c:\\Windows\\system32\\ntoskrnl.exe")
    /// # };
    ///
    /// gm.read_volatile_from(addr, &mut file, 128)
    ///     .expect("Could not read from /dev/urandom into guest memory");
    ///
    /// let read_addr = addr.checked_add(8).expect("Could not compute read address");
    /// let rand_val: u32 = gm
    ///     .read_obj(read_addr)
    ///     .expect("Could not read u32 val from /dev/urandom");
    /// # }
    /// ```
    fn read_volatile_from<F>(&self, addr: A, src: &mut F, count: usize) -> Result<usize, Self::E>
    where
        F: ReadVolatile;

    /// Reads exactly `count` bytes from an object and writes them into the container at `addr`.
    ///
    /// # Errors
    ///
    /// Returns an error if `count` bytes couldn't have been copied from `src` to the container.
    /// Part of the data may have been copied nevertheless.
    ///
    /// # Arguments
    /// * `addr` - Begin writing at this address.
    /// * `src` - Copy from `src` into the container.
    /// * `count` - Copy exactly `count` bytes from `src` into the container.
    fn read_exact_volatile_from<F>(
        &self,
        addr: A,
        src: &mut F,
        count: usize,
    ) -> Result<(), Self::E>
    where
        F: ReadVolatile;

    /// Reads up to `count` bytes from the container at `addr` and writes them into `dst`.
    /// Unlike `VolatileWrite::write_volatile`, this function retries on `EINTR` being returned by
    /// the underlying I/O `write` operation.
    ///
    /// Returns the number of bytes written into the object.
    ///
    /// # Arguments
    /// * `addr` - Begin reading from this address.
    /// * `dst` - Copy from the container to `dst`.
    /// * `count` - Copy `count` bytes from the container to `dst`.
    fn write_volatile_to<F>(&self, addr: A, dst: &mut F, count: usize) -> Result<usize, Self::E>
    where
        F: WriteVolatile;

    /// Reads exactly `count` bytes from the container at `addr` and writes them into an object.
    ///
    /// # Errors
    ///
    /// Returns an error if `count` bytes couldn't have been copied from the container to `dst`.
    /// Part of the data may have been copied nevertheless.
    ///
    /// # Arguments
    /// * `addr` - Begin reading from this address.
    /// * `dst` - Copy from the container to `dst`.
    /// * `count` - Copy exactly `count` bytes from the container to `dst`.
    fn write_all_volatile_to<F>(&self, addr: A, dst: &mut F, count: usize) -> Result<(), Self::E>
    where
        F: WriteVolatile;

    /// Atomically store a value at the specified address.
    fn store<T: AtomicAccess>(&self, val: T, addr: A, order: Ordering) -> Result<(), Self::E>;

    /// Atomically load a value from the specified address.
    fn load<T: AtomicAccess>(&self, addr: A, order: Ordering) -> Result<T, Self::E>;
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(clippy::undocumented_unsafe_blocks)]
    use super::*;

    use std::cell::RefCell;
    use std::fmt::Debug;

    // Helper method to test atomic accesses for a given `b: Bytes` that's supposed to be
    // zero-initialized.
    pub fn check_atomic_accesses<A, B>(b: B, addr: A, bad_addr: A)
    where
        A: Copy,
        B: Bytes<A>,
        B::E: Debug,
    {
        let val = 100u32;

        assert_eq!(b.load::<u32>(addr, Ordering::Relaxed).unwrap(), 0);
        b.store(val, addr, Ordering::Relaxed).unwrap();
        assert_eq!(b.load::<u32>(addr, Ordering::Relaxed).unwrap(), val);

        b.load::<u32>(bad_addr, Ordering::Relaxed).unwrap_err();
        b.store(val, bad_addr, Ordering::Relaxed).unwrap_err();
    }

    pub const MOCK_BYTES_CONTAINER_SIZE: usize = 10;

    pub struct MockBytesContainer {
        container: RefCell<[u8; MOCK_BYTES_CONTAINER_SIZE]>,
    }

    impl MockBytesContainer {
        pub fn new() -> Self {
            MockBytesContainer {
                container: RefCell::new([0; MOCK_BYTES_CONTAINER_SIZE]),
            }
        }

        pub fn validate_slice_op(&self, buf: &[u8], addr: usize) -> Result<(), ()> {
            if MOCK_BYTES_CONTAINER_SIZE - buf.len() <= addr {
                return Err(());
            }

            Ok(())
        }
    }

    impl Bytes<usize> for MockBytesContainer {
        type E = ();

        fn write(&self, _: &[u8], _: usize) -> Result<usize, Self::E> {
            unimplemented!()
        }

        fn read(&self, _: &mut [u8], _: usize) -> Result<usize, Self::E> {
            unimplemented!()
        }

        fn write_slice(&self, buf: &[u8], addr: usize) -> Result<(), Self::E> {
            self.validate_slice_op(buf, addr)?;

            let mut container = self.container.borrow_mut();
            container[addr..addr + buf.len()].copy_from_slice(buf);

            Ok(())
        }

        fn read_slice(&self, buf: &mut [u8], addr: usize) -> Result<(), Self::E> {
            self.validate_slice_op(buf, addr)?;

            let container = self.container.borrow();
            buf.copy_from_slice(&container[addr..addr + buf.len()]);

            Ok(())
        }

        fn read_volatile_from<F>(
            &self,
            _addr: usize,
            _src: &mut F,
            _count: usize,
        ) -> Result<usize, Self::E>
        where
            F: ReadVolatile,
        {
            unimplemented!()
        }

        fn read_exact_volatile_from<F>(
            &self,
            _addr: usize,
            _src: &mut F,
            _count: usize,
        ) -> Result<(), Self::E>
        where
            F: ReadVolatile,
        {
            unimplemented!()
        }

        fn write_volatile_to<F>(
            &self,
            _addr: usize,
            _dst: &mut F,
            _count: usize,
        ) -> Result<usize, Self::E>
        where
            F: WriteVolatile,
        {
            unimplemented!()
        }

        fn write_all_volatile_to<F>(
            &self,
            _addr: usize,
            _dst: &mut F,
            _count: usize,
        ) -> Result<(), Self::E>
        where
            F: WriteVolatile,
        {
            unimplemented!()
        }

        fn store<T: AtomicAccess>(
            &self,
            _val: T,
            _addr: usize,
            _order: Ordering,
        ) -> Result<(), Self::E> {
            unimplemented!()
        }

        fn load<T: AtomicAccess>(&self, _addr: usize, _order: Ordering) -> Result<T, Self::E> {
            unimplemented!()
        }
    }

    #[test]
    fn test_bytes() {
        let bytes = MockBytesContainer::new();

        bytes.write_obj(u64::MAX, 0).unwrap();
        assert_eq!(bytes.read_obj::<u64>(0).unwrap(), u64::MAX);

        assert_eq!(
            bytes.write_obj(u64::MAX, MOCK_BYTES_CONTAINER_SIZE),
            Err(())
        );
        assert_eq!(bytes.read_obj::<u64>(MOCK_BYTES_CONTAINER_SIZE), Err(()));
    }
}
