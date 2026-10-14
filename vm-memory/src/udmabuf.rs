use std::{
    fs::File,
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    path::Path,
};

use vmm_sys_util::{
    fam::{self, FamStruct, FamStructWrapper},
    generate_fam_struct_impl,
    ioctl::ioctl_with_ref,
    ioctl_iow_nr,
};

use crate::{Address, GuestAddress, GuestMemoryBackend, GuestMemoryMmap, GuestMemoryRegion};

#[repr(C)]
#[derive(Debug, Default, PartialEq, Eq)]
struct __IncompleteArrayField<T>(::std::marker::PhantomData<T>, [T; 0]);
impl<T> __IncompleteArrayField<T> {
    #[inline]
    unsafe fn as_ptr(&self) -> *const T {
        self as *const __IncompleteArrayField<T> as *const T
    }
    #[inline]
    unsafe fn as_mut_ptr(&mut self) -> *mut T {
        self as *mut __IncompleteArrayField<T> as *mut T
    }
    #[inline]
    unsafe fn as_slice(&self, len: usize) -> &[T] {
        ::std::slice::from_raw_parts(self.as_ptr(), len)
    }
    #[inline]
    unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        ::std::slice::from_raw_parts_mut(self.as_mut_ptr(), len)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UdmabufError {
    #[error("could not open device: {0}")]
    OpenFailed(io::Error),

    #[error("could not create ioctl struct: {0}")]
    StructError(fam::Error),

    #[error("page size unavailable")]
    NoPageSize,

    #[error("could not find memory region")]
    RegionNotFound,

    #[error("memory region not backed by memfd")]
    RegionNotFileBacked,

    #[error("provided address and length are out of bounds for a region")]
    OutOfBounds,

    #[error("starting address or length not page aligned")]
    NotPageAligned,

    #[error("could not create udmabuf: {0}")]
    CreateFailed(io::Error),
}

pub type Result<T> = std::result::Result<T, UdmabufError>;

const UDMABUF_FLAGS_CLOEXEC: u32 = 1;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct UdmabufCreateItem {
    memfd: i32,
    __pad: u32,
    offset: u64,
    size: u64,
}

#[repr(C)]
#[derive(Debug, Default)]
struct UdmabufCreateList {
    flags: u32,
    count: u32,
    list: __IncompleteArrayField<UdmabufCreateItem>,
}

generate_fam_struct_impl!(
    UdmabufCreateList,
    UdmabufCreateItem,
    list,
    u32,
    count,
    usize::MAX
);

type UdmabufCreateListWrapper = FamStructWrapper<UdmabufCreateList>;

const UDMABUF_TYPE: u32 = 'u' as u32;

ioctl_iow_nr!(udmabuf_create_list, UDMABUF_TYPE, 0x43, UdmabufCreateList);

/// A convenience wrapper for the Linux kernel's udmabuf driver.
pub struct UdmabufDriver {
    driver_fd: OwnedFd,
    page_size: usize,
}

impl UdmabufDriver {
    pub fn new() -> Result<UdmabufDriver> {
        const UDMABUF_PATH: &str = "/dev/udmabuf";
        let path = Path::new(UDMABUF_PATH);
        let driver_fd = File::open(path).map_err(UdmabufError::OpenFailed)?.into();

        // SAFETY: Safe because this call just returns the page size and doesn't have any side effects.
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };

        Ok(UdmabufDriver {
            driver_fd,
            page_size,
        })
    }

    pub fn create_udmabuf(
        &self,
        mem: &GuestMemoryMmap,
        iovecs: &[(GuestAddress, usize)],
    ) -> Result<OwnedFd> {
        let mut list = UdmabufCreateListWrapper::from_header(UdmabufCreateList {
            flags: UDMABUF_FLAGS_CLOEXEC,
            ..Default::default()
        })
        .map_err(UdmabufError::StructError)?;
        for &(addr, len) in iovecs.iter() {
            let region = mem.find_region(addr).ok_or(UdmabufError::RegionNotFound)?;

            let Some(file_offset) = region.file_offset() else {
                return Err(UdmabufError::RegionNotFileBacked);
            };

            let map_offset = addr
                .checked_sub(region.start_addr().0)
                .ok_or(UdmabufError::OutOfBounds)?;

            if map_offset.0 as usize + len > region.len() as usize {
                return Err(UdmabufError::OutOfBounds);
            }

            let offset = file_offset.start() + map_offset.0;

            if offset as usize % self.page_size != 0 || len % self.page_size != 0 {
                return Err(UdmabufError::NotPageAligned);
            }

            list.push(UdmabufCreateItem {
                memfd: file_offset.file().as_raw_fd(),
                __pad: 0,
                offset: offset,
                size: len as u64,
            })
            .map_err(UdmabufError::StructError)?;
        }

        // SAFETY: We have correctly allocated the structure above
        let fd = unsafe {
            ioctl_with_ref(
                &self.driver_fd,
                udmabuf_create_list(),
                list.as_fam_struct_ref(),
            )
        };
        if fd < 0 {
            return Err(UdmabufError::CreateFailed(io::Error::last_os_error()));
        }

        // SAFETY: Returned i32 is a valid fd since it was positive
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}
