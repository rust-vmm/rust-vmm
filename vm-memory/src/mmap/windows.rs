// Copyright (C) 2019 CrowdStrike, Inc. All rights reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

//! Helper structure for working with mmaped memory regions in Windows.

use std;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::ptr::{null, null_mut};

use libc::{c_void, size_t};

use winapi::um::errhandlingapi::GetLastError;

use crate::bitmap::{Bitmap, NewBitmap, BS};
use crate::guest_memory::FileOffset;
use crate::volatile_memory::{self, compute_offset, VolatileMemory, VolatileSlice};

#[allow(non_snake_case)]
#[link(name = "kernel32")]
extern "system" {
    pub fn VirtualAlloc(
        lpAddress: *mut c_void,
        dwSize: size_t,
        flAllocationType: u32,
        flProtect: u32,
    ) -> *mut c_void;

    pub fn VirtualFree(lpAddress: *mut c_void, dwSize: size_t, dwFreeType: u32) -> u32;

    pub fn CreateFileMappingA(
        hFile: RawHandle,                       // HANDLE
        lpFileMappingAttributes: *const c_void, // LPSECURITY_ATTRIBUTES
        flProtect: u32,                         // DWORD
        dwMaximumSizeHigh: u32,                 // DWORD
        dwMaximumSizeLow: u32,                  // DWORD
        lpName: *const u8,                      // LPCSTR
    ) -> RawHandle; // HANDLE

    pub fn MapViewOfFile(
        hFileMappingObject: RawHandle,
        dwDesiredAccess: u32,
        dwFileOffsetHigh: u32,
        dwFileOffsetLow: u32,
        dwNumberOfBytesToMap: size_t,
    ) -> *mut c_void;

    pub fn UnmapViewOfFile(lpBaseAddress: *const c_void) -> u32; // BOOL
}

const MM_HIGHEST_VAD_ADDRESS: u64 = 0x000007FFFFFDFFFF;

const MEM_COMMIT: u32 = 0x00001000;
const MEM_RELEASE: u32 = 0x00008000;
const FILE_MAP_ALL_ACCESS: u32 = 0xf001f;
const PAGE_READWRITE: u32 = 0x04;

pub const MAP_FAILED: *mut c_void = null_mut::<c_void>();
pub const INVALID_HANDLE_VALUE: RawHandle = (-1isize) as RawHandle;
pub const ERROR_INVALID_PARAMETER: i32 = 87;

/// Helper structure for working with mmaped memory regions in Unix.
///
/// The structure is used for accessing the guest's physical memory by mmapping it into
/// the current process.
///
/// # Limitations
/// When running a 64-bit virtual machine on a 32-bit hypervisor, only part of the guest's
/// physical memory may be mapped into the current process due to the limited virtual address
/// space size of the process.
#[derive(Debug)]
pub struct MmapRegion<B> {
    size: usize,
    bitmap: B,
    file_offset: Option<FileOffset>,
    allocation: Allocation,
}

#[derive(Debug)]
enum Allocation {
    VirtualAlloc(*mut u8),
    MappedView(*mut u8),
}

impl Allocation {
    fn as_ptr(&self) -> *mut u8 {
        match self {
            Self::VirtualAlloc(address) | Self::MappedView(address) => *address,
        }
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        // SAFETY: this owner contains one successful native allocation and uses
        // its matching release operation, including on constructor unwinding.
        let result = unsafe {
            match self {
                Self::MappedView(address) => UnmapViewOfFile(address.cast()),
                // MEM_RELEASE requires size zero for a VirtualAlloc reservation.
                Self::VirtualAlloc(address) => VirtualFree(address.cast(), 0, MEM_RELEASE),
            }
        };
        if result == 0 {
            // No logger is available here; preserve the native failure evidence.
            println!(
                "WARNING: Could not deallocate mmap allocation {self:?}. Error: {}",
                unsafe { GetLastError() }
            );
        }
    }
}

// Send and Sync aren't automatically inherited for the raw address pointer.
// Accessing that pointer is only done through the stateless interface which
// allows the object to be shared by multiple threads without a decrease in
// safety.
unsafe impl<B: Send> Send for MmapRegion<B> {}
unsafe impl<B: Sync> Sync for MmapRegion<B> {}

impl<B: NewBitmap> MmapRegion<B> {
    /// Creates a shared anonymous mapping of `size` bytes.
    ///
    /// # Arguments
    /// * `size` - The size of the memory region in bytes.
    pub fn new(size: usize) -> io::Result<Self> {
        if (size == 0) || (size > MM_HIGHEST_VAD_ADDRESS as usize) {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }
        // This is safe because we are creating an anonymous mapping in a place not already used by
        // any other area in this process.
        let addr = unsafe { VirtualAlloc(null_mut::<c_void>(), size, MEM_COMMIT, PAGE_READWRITE) };
        if addr == MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let allocation = Allocation::VirtualAlloc(addr.cast());
        Ok(Self {
            size,
            bitmap: B::with_len(size),
            file_offset: None,
            allocation,
        })
    }

    /// Creates a shared file mapping of `size` bytes.
    ///
    /// # Arguments
    /// * `file_offset` - The mapping will be created at offset `file_offset.start` in the file
    ///   referred to by `file_offset.file`.
    /// * `size` - The size of the memory region in bytes.
    ///
    /// The offset must be aligned to the system allocation granularity. Windows
    /// may extend a short file to `offset + size`, even if creating the view fails.
    pub fn from_file(file_offset: FileOffset, size: usize) -> io::Result<Self> {
        let handle = file_offset.file().as_raw_handle();
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::from_raw_os_error(libc::EBADF));
        }

        let offset = file_offset.start();
        let end = offset
            .checked_add(size as u64)
            .filter(|_| size != 0)
            .ok_or_else(|| io::Error::from_raw_os_error(ERROR_INVALID_PARAMETER))?;

        let mapping = unsafe {
            CreateFileMappingA(
                handle,
                null(),
                PAGE_READWRITE,
                (end >> 32) as u32,
                end as u32,
                null(),
            )
        };
        if mapping == 0 as RawHandle {
            return Err(io::Error::last_os_error());
        }

        // SAFETY: CreateFileMappingA returned a valid handle that we uniquely own.
        let mapping = unsafe { OwnedHandle::from_raw_handle(mapping) };

        // This is safe because we are creating a mapping in a place not already used by any other
        // area in this process.
        let addr = unsafe {
            MapViewOfFile(
                mapping.as_raw_handle(),
                FILE_MAP_ALL_ACCESS,
                (offset >> 32) as u32,
                offset as u32,
                size,
            )
        };

        if addr.is_null() {
            return Err(io::Error::last_os_error());
        }
        let allocation = Allocation::MappedView(addr.cast());
        Ok(Self {
            size,
            bitmap: B::with_len(size),
            file_offset: Some(file_offset),
            allocation,
        })
    }
}

impl<B: Bitmap> MmapRegion<B> {
    /// Returns a pointer to the beginning of the memory region. Mutable accesses performed
    /// using the resulting pointer are not automatically accounted for by the dirty bitmap
    /// tracking functionality.
    ///
    /// Should only be used for passing this region to ioctls for setting guest memory.
    pub fn as_ptr(&self) -> *mut u8 {
        self.allocation.as_ptr()
    }

    /// Returns the size of this region.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns information regarding the offset into the file backing this region (if any).
    pub fn file_offset(&self) -> Option<&FileOffset> {
        self.file_offset.as_ref()
    }

    /// Returns a reference to the inner bitmap object.
    pub fn bitmap(&self) -> &B {
        &self.bitmap
    }
}

impl<B: Bitmap> VolatileMemory for MmapRegion<B> {
    type B = B;

    fn len(&self) -> usize {
        self.size
    }

    fn get_slice(
        &self,
        offset: usize,
        count: usize,
    ) -> volatile_memory::Result<VolatileSlice<'_, BS<'_, Self::B>>> {
        let end = compute_offset(offset, count)?;
        if end > self.size {
            return Err(volatile_memory::Error::OutOfBounds { addr: end });
        }

        // Safe because we checked that offset + count was within our range and we only ever hand
        // out volatile accessors.
        Ok(unsafe {
            VolatileSlice::with_bitmap(
                self.as_ptr().add(offset),
                count,
                self.bitmap.slice_at(offset),
                None,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use std::os::windows::io::FromRawHandle;
    use std::sync::Arc;

    use vmm_sys_util::tempfile::TempFile;
    use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};

    #[cfg(feature = "backend-bitmap")]
    use crate::bitmap::AtomicBitmap;
    use crate::guest_memory::FileOffset;
    use crate::mmap::windows::{ERROR_INVALID_PARAMETER, INVALID_HANDLE_VALUE};

    type MmapRegion = super::MmapRegion<()>;

    #[test]
    fn map_invalid_handle() {
        let file = unsafe { std::fs::File::from_raw_handle(INVALID_HANDLE_VALUE) };
        let file_offset = FileOffset::new(file, 0);
        let e = MmapRegion::from_file(file_offset, 1024).unwrap_err();
        assert_eq!(e.raw_os_error(), Some(libc::EBADF));
    }

    #[test]
    fn map_file_offset() {
        // SAFETY: GetSystemInfo initializes the supplied SYSTEM_INFO.
        let info = unsafe {
            let mut info: SYSTEM_INFO = std::mem::zeroed();
            GetSystemInfo(&mut info);
            info
        };
        let offset = u64::from(info.dwAllocationGranularity);
        let size = info.dwPageSize as usize;
        let file = Arc::new(TempFile::new().unwrap().into_file());
        file.set_len(offset + size as u64).unwrap();
        let region =
            MmapRegion::from_file(FileOffset::from_arc(file.clone(), offset), size).unwrap();

        // Windows refuses truncation while mapped views remain. Check the file
        // object rather than probing an address another test could have reused.
        assert!(file.set_len(0).is_err());
        drop(region);
        file.set_len(0)
            .expect("file view and mapping handle released");
    }

    #[test]
    fn map_invalid_extents() {
        let file = Arc::new(TempFile::new().unwrap().into_file());
        file.set_len(0x10000).unwrap();
        for (offset, size) in [(0, 0), (u64::MAX, 2)] {
            let error = MmapRegion::from_file(FileOffset::from_arc(file.clone(), offset), size)
                .unwrap_err();
            assert_eq!(error.raw_os_error(), Some(ERROR_INVALID_PARAMETER));
        }
        assert_eq!(file.metadata().unwrap().len(), 0x10000);
    }

    #[test]
    fn map_failed_view() {
        let file = Arc::new(TempFile::new().unwrap().into_file());
        file.set_len(4097).unwrap();
        let error = MmapRegion::from_file(FileOffset::from_arc(file.clone(), 1), 4096).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(1132)); // ERROR_MAPPED_ALIGNMENT
        file.set_len(0)
            .expect("failed view released its section handle");
    }

    #[test]
    #[cfg(feature = "backend-bitmap")]
    fn test_dirty_tracking() {
        // Using the `crate` prefix because we aliased `MmapRegion` to `MmapRegion<()>` for
        // the rest of the unit tests above.
        let m = crate::MmapRegion::<AtomicBitmap>::new(0x1_0000).unwrap();
        crate::bitmap::tests::test_volatile_memory(&m);
    }
}
