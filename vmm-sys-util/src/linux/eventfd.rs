// Copyright 2019 Intel Corporation. All Rights Reserved.
//
// Copyright 2017 The Chromium OS Authors. All rights reserved.
//
// SPDX-License-Identifier: BSD-3-Clause

//! Structure and wrapper functions for working with
//! [`eventfd`](http://man7.org/linux/man-pages/man2/eventfd.2.html).

use libc::eventfd;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::IntoRawFd;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::{io, result};

// Reexport commonly used flags from libc.
pub use libc::{EFD_CLOEXEC, EFD_NONBLOCK, EFD_SEMAPHORE};

/// A safe wrapper around Linux
/// [`eventfd`](http://man7.org/linux/man-pages/man2/eventfd.2.html).
#[derive(Debug)]
pub struct EventFd {
    eventfd: File,
}

impl EventFd {
    /// Create a new EventFd with an initial value.
    ///
    /// # Arguments
    ///
    /// * `flag`: The initial value used for creating the `EventFd`.
    ///   Refer to Linux [`eventfd`](http://man7.org/linux/man-pages/man2/eventfd.2.html).
    /// # Examples
    ///
    /// ```
    /// extern crate vmm_sys_util;
    /// use vmm_sys_util::eventfd::{EventFd, EFD_NONBLOCK};
    ///
    /// EventFd::new(EFD_NONBLOCK).unwrap();
    /// ```
    pub fn new(flag: i32) -> result::Result<EventFd, io::Error> {
        // SAFETY: This is safe because eventfd merely allocated an eventfd for
        // our process and we handle the error case.
        let ret = unsafe { eventfd(0, flag) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(EventFd {
                // SAFETY: This is safe because we checked ret for success and know
                // the kernel gave us an fd that we own.
                eventfd: unsafe { File::from_raw_fd(ret) },
            })
        }
    }

    /// Add a value to the eventfd's counter.
    ///
    /// When the addition causes the counter overflow, this would either block
    /// until a [`read`](http://man7.org/linux/man-pages/man2/read.2.html) is
    /// performed on the file descriptor, or fail with the
    /// error EAGAIN if the file descriptor has been made nonblocking.
    ///
    /// # Arguments
    ///
    /// * `v`: the value to be added to the eventfd's counter.
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate vmm_sys_util;
    /// use vmm_sys_util::eventfd::{EventFd, EFD_NONBLOCK};
    ///
    /// let evt = EventFd::new(EFD_NONBLOCK).unwrap();
    /// evt.write(55).unwrap();
    /// ```
    pub fn write(&self, v: u64) -> result::Result<(), io::Error> {
        (&self.eventfd).write_all(v.to_ne_bytes().as_slice())
    }

    /// Read a value from the eventfd.
    ///
    /// If the counter is zero, this would either block
    /// until the counter becomes nonzero, or fail with the
    /// error EAGAIN if the file descriptor has been made nonblocking.
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate vmm_sys_util;
    /// use vmm_sys_util::eventfd::{EventFd, EFD_NONBLOCK};
    ///
    /// let evt = EventFd::new(EFD_NONBLOCK).unwrap();
    /// evt.write(55).unwrap();
    /// assert_eq!(evt.read().unwrap(), 55);
    /// ```
    pub fn read(&self) -> result::Result<u64, io::Error> {
        let mut buf = [0u8; std::mem::size_of::<u64>()];
        (&self.eventfd).read_exact(&mut buf)?;
        Ok(u64::from_ne_bytes(buf))
    }

    /// Clone this EventFd.
    ///
    /// This internally creates a new file descriptor and it will share the same
    /// underlying count within the kernel.
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate vmm_sys_util;
    /// use vmm_sys_util::eventfd::{EventFd, EFD_NONBLOCK};
    ///
    /// let evt = EventFd::new(EFD_NONBLOCK).unwrap();
    /// let evt_clone = evt.try_clone().unwrap();
    /// evt.write(923).unwrap();
    /// assert_eq!(evt_clone.read().unwrap(), 923);
    /// ```
    pub fn try_clone(&self) -> result::Result<EventFd, io::Error> {
        Ok(EventFd {
            eventfd: self.eventfd.try_clone()?,
        })
    }
}

impl AsRawFd for EventFd {
    fn as_raw_fd(&self) -> RawFd {
        self.eventfd.as_raw_fd()
    }
}

impl FromRawFd for EventFd {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        EventFd {
            eventfd: File::from_raw_fd(fd),
        }
    }
}

impl IntoRawFd for EventFd {
    fn into_raw_fd(self) -> RawFd {
        self.eventfd.into_raw_fd()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        EventFd::new(EFD_NONBLOCK).unwrap();
        EventFd::new(0).unwrap();
    }

    #[test]
    fn test_read_write() {
        let evt = EventFd::new(EFD_NONBLOCK).unwrap();
        evt.write(55).unwrap();
        assert_eq!(evt.read().unwrap(), 55);
    }

    #[test]
    fn test_write_overflow() {
        let evt = EventFd::new(EFD_NONBLOCK).unwrap();
        evt.write(u64::MAX - 1).unwrap();
        let err = evt.write(1).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
    }
    #[test]
    fn test_read_nothing() {
        let evt = EventFd::new(EFD_NONBLOCK).unwrap();
        let err = evt.read().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
    }
    #[test]
    fn test_clone() {
        let evt = EventFd::new(EFD_NONBLOCK).unwrap();
        let evt_clone = evt.try_clone().unwrap();
        evt.write(923).unwrap();
        assert_eq!(evt_clone.read().unwrap(), 923);
    }
}
