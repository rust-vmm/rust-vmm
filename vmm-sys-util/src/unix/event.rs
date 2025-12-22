// SPDX-License-Identifier: BSD-3-Clause
//! This module provides a platform-independent interface for event notification,
//! enabling support across multiple operating systems.
//!
//! Internally, it uses a file descriptor that can be backed by various mechanisms
//! such as `eventfd`, `pipe`, or other OS-specific primitives.

use std::io::{Read, Write};
use std::os::fd::RawFd;
use std::{
    fs::File,
    io,
    os::fd::{AsRawFd, FromRawFd, IntoRawFd},
    result::Result,
};

bitflags::bitflags! {
    /// EventFlag
    /// This enum is used to define flags for the event notifier and consumer.
    pub struct EventFlag: u8 {
        /// Non-blocking flag
        const NONBLOCK = 1 << 0;
        /// Close-on-exec flag
        const CLOEXEC = 1 << 1;
    }
}

/// EventNotifier
/// This is a generic event notifier that can be used with eventfd or pipefd.
/// It allows writing a value to the file descriptor to notify an event.
///
/// # Examples
///
/// ```
/// use std::os::fd::FromRawFd;
/// use std::os::unix::io::IntoRawFd;
/// use vmm_sys_util::event::EventNotifier;
/// let (_, writer) = std::io::pipe().expect("Failed to create pipe");
/// let notifier = unsafe { EventNotifier::from_raw_fd(writer.into_raw_fd()) };
/// ```
#[derive(Debug)]
pub struct EventNotifier {
    fd: File,
}

impl EventNotifier {
    /// Write a value to the EventNotifier's fd
    /// Writing 1 to fd is for compatibility with Eventfd
    pub fn notify(&self) -> Result<(), io::Error> {
        let v = 1u64;
        (&self.fd).write_all(&v.to_ne_bytes())
    }

    /// Clone this EventNotifier.
    pub fn try_clone(&self) -> Result<EventNotifier, io::Error> {
        Ok(EventNotifier {
            fd: self.fd.try_clone()?,
        })
    }
}

impl AsRawFd for EventNotifier {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl FromRawFd for EventNotifier {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        EventNotifier {
            fd: File::from_raw_fd(fd),
        }
    }
}

impl IntoRawFd for EventNotifier {
    fn into_raw_fd(self) -> RawFd {
        self.fd.into_raw_fd()
    }
}

/// EventConsumer
/// This is a generic event consumer that can be used with eventfd or pipefd.
/// It allows reading a value from the file descriptor to consume an event.
///
/// # Examples
///
/// ```
/// use std::os::fd::FromRawFd;
/// use std::os::unix::io::IntoRawFd;
/// use vmm_sys_util::event::EventConsumer;
/// let (reader, _) = std::io::pipe().expect("Failed to create pipe");
/// let consumer = unsafe { EventConsumer::from_raw_fd(reader.into_raw_fd()) };
/// ```
#[derive(Debug)]
pub struct EventConsumer {
    fd: File,
}

impl EventConsumer {
    /// Read a value from the EventConsumer.
    pub fn consume(&self) -> Result<(), io::Error> {
        let mut buf = [0u8; size_of::<u64>()];
        (&self.fd).read_exact(buf.as_mut_slice()).map(|_| Ok(()))?
    }

    /// Clone this EventConsumer.
    pub fn try_clone(&self) -> Result<EventConsumer, io::Error> {
        Ok(EventConsumer {
            fd: self.fd.try_clone()?,
        })
    }
}

impl AsRawFd for EventConsumer {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl FromRawFd for EventConsumer {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        EventConsumer {
            fd: File::from_raw_fd(fd),
        }
    }
}

impl IntoRawFd for EventConsumer {
    fn into_raw_fd(self) -> RawFd {
        self.fd.into_raw_fd()
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn fcntl_setfl(file: &File, flag: i32) -> Result<(), io::Error> {
    // SAFETY: Rust's I/O safety ensures `file` contains a valid FD.
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: Rust's I/O safety ensures `file` contains a valid FD.
    let ret = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | flag) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn fcntl_setfd(file: &File, flag: i32) -> Result<(), io::Error> {
    // SAFETY: Rust's I/O safety ensures `file` contains a valid FD.
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: Rust's I/O safety ensures `file` contains a valid FD.
    let ret = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, flags | flag) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Create a new EventNotifier and EventConsumer using a pipe.
///
/// # Arguments
///
/// * `flags` - Flags to set on the file descriptor, such as `EventFlag::NONBLOCK` or `EventFlag::CLOEXEC`.
///
/// # Examples
///
/// ```
/// use vmm_sys_util::event::{new_event_consumer_and_notifier, EventFlag};
/// let (consumer, notifier) = new_event_consumer_and_notifier(EventFlag::NONBLOCK)
///     .expect("Failed to create notifier and consumer");
/// notifier.notify().unwrap();
/// assert!(consumer.consume().is_ok());
/// ```
#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub fn new_event_consumer_and_notifier(
    flags: EventFlag,
) -> Result<(EventConsumer, EventNotifier), io::Error> {
    // Use a pipe for non-Linux platforms.
    let mut fds: [RawFd; 2] = [-1, -1];
    // SAFETY: This is safe because pipe merely allocated a read fd and a write fd
    // for our process and we handle the error case.
    let ret = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: Safe because we check the fd is valid. And the kernel gave us an fd that we own.
    let consumer = unsafe { EventConsumer::from_raw_fd(fds[0]) };
    // SAFETY: Safe because we check the fd is valid. And the kernel gave us an fd that we own.
    let notifier = unsafe { EventNotifier::from_raw_fd(fds[1]) };
    if flags.contains(EventFlag::NONBLOCK) {
        fcntl_setfl(&consumer.fd, libc::O_NONBLOCK)?;
        fcntl_setfl(&notifier.fd, libc::O_NONBLOCK)?;
    }
    if flags.contains(EventFlag::CLOEXEC) {
        fcntl_setfd(&consumer.fd, libc::FD_CLOEXEC)?;
        fcntl_setfd(&notifier.fd, libc::FD_CLOEXEC)?;
    }
    Ok((consumer, notifier))
}

/// Create a new EventNotifier and EventConsumer using eventfd.
///
/// # Arguments
///
/// * `flags` - Flags to set on the file descriptor, such as `EventFlag::NONBLOCK` or `EventFlag::CLOEXEC`.
///
/// # Examples
///
/// ```
/// use vmm_sys_util::event::{new_event_consumer_and_notifier, EventFlag};
/// let (consumer, notifier) = new_event_consumer_and_notifier(EventFlag::NONBLOCK)
///     .expect("Failed to create consumer and notifier");
/// notifier.notify().unwrap();
/// assert!(consumer.consume().is_ok());
/// ```
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn new_event_consumer_and_notifier(
    flags: EventFlag,
) -> Result<(EventConsumer, EventNotifier), io::Error> {
    let mut efd_flags = 0;
    if flags.contains(EventFlag::NONBLOCK) {
        efd_flags |= libc::EFD_NONBLOCK;
    }
    if flags.contains(EventFlag::CLOEXEC) {
        efd_flags |= libc::EFD_CLOEXEC;
    }
    let eventfd = crate::linux::eventfd::EventFd::new(efd_flags)?;
    let eventfd_clone = eventfd.try_clone()?;
    // SAFETY: Safe because we check the fd is valid. And the kernel gave us an fd that we own.
    let consumer = unsafe { EventConsumer::from_raw_fd(eventfd.into_raw_fd()) };
    // SAFETY: Safe because we check the fd is valid. And the kernel gave us an fd that we own.
    let notifier = unsafe { EventNotifier::from_raw_fd(eventfd_clone.into_raw_fd()) };
    Ok((consumer, notifier))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::pipe, os::fd::IntoRawFd};

    #[test]
    fn test_notify_and_consume() {
        let (reader, writer) = pipe().expect("Failed to create pipe");
        // SAFETY: Safe because we check the fd is valid. And the kernel gave us an fd that we own.
        let notifier = unsafe { EventNotifier::from_raw_fd(writer.into_raw_fd()) };
        // SAFETY: Safe because we check the fd is valid. And the kernel gave us an fd that we own.
        let consumer = unsafe { EventConsumer::from_raw_fd(reader.into_raw_fd()) };

        notifier.notify().unwrap();
        assert!(consumer.consume().is_ok());
    }

    #[test]
    fn test_clone() {
        let (reader, writer) = pipe().expect("Failed to create pipe");
        // SAFETY: Safe because we check the fd is valid. And the kernel gave us an fd that we own.
        let notifier = unsafe { EventNotifier::from_raw_fd(writer.into_raw_fd()) };
        // SAFETY: Safe because we check the fd is valid. And the kernel gave us an fd that we own.
        let consumer = unsafe { EventConsumer::from_raw_fd(reader.into_raw_fd()) };

        let cloned_notifier = notifier.try_clone().expect("Failed to clone notifier");
        let cloned_consumer = consumer.try_clone().expect("Failed to clone consumer");

        cloned_notifier.notify().unwrap();
        assert!(cloned_consumer.consume().is_ok());
    }

    #[test]
    fn test_new_event_notifier_and_consumer() {
        let (consumer, notifier) = new_event_consumer_and_notifier(EventFlag::empty())
            .expect("Failed to create notifier and consumer");
        notifier.notify().unwrap();
        assert!(consumer.consume().is_ok());
    }

    #[test]
    fn test_nonblock() {
        let (consumer, _notifier) = new_event_consumer_and_notifier(EventFlag::NONBLOCK)
            .expect("Failed to create notifier and consumer");
        let err = consumer.consume().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
    }

    #[test]
    fn test_cloexec() {
        let (consumer, notifier) = new_event_consumer_and_notifier(EventFlag::CLOEXEC)
            .expect("Failed to create notifier and consumer");
        // SAFETY: This is safe because we check the fd and the return value are valid.
        let flags = unsafe { libc::fcntl(consumer.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0 && flags & libc::FD_CLOEXEC == 1);
        // SAFETY: This is safe because we check the fd and the return value are valid.
        let flags = unsafe { libc::fcntl(notifier.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0 && flags & libc::FD_CLOEXEC == 1);
        let consumer_cloned = consumer.try_clone().expect("Failed to clone consumer");
        // SAFETY: This is safe because we check the fd and the return value are valid.
        let flags = unsafe { libc::fcntl(consumer_cloned.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0 && flags & libc::FD_CLOEXEC == 1);
        let notifier_cloned = notifier.try_clone().expect("Failed to clone notifier");
        // SAFETY: This is safe because we check the fd and the return value are valid.
        let flags = unsafe { libc::fcntl(notifier_cloned.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0 && flags & libc::FD_CLOEXEC == 1);
    }
}
