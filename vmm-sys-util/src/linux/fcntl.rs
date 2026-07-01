// Copyright © 2025 Cyberus Technology GmbH
//
// SPDX-License-Identifier: BSD-3-Clause

//! Helpers for advisory file locking.
//!
//! Under the hood, the implementation uses open file description (OFD) locks
//! for the requested byte range, as described in [this article][ofd]. The
//! advantage over `F_SETLKW` (used by the Rust standard library's
//! `File::try_lock()`) is that only the very last `close()` on a file
//! description releases the lock. This prevents mistakes and unexpected
//! behavior when descriptors are duplicated (e.g. via `fork()` or `dup()`).
//!
//! [ofd]: https://apenwarr.ca/log/20101213

use std::fmt;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};

/// Errors that can happen when working with file locks.
#[derive(Debug)]
pub enum LockError {
    /// The file is already locked by another open file description.
    ///
    /// A call to [`get_lock_state`] can help to identify the reason.
    AlreadyLocked,
    /// The lock state could not be checked or set.
    Io(io::Error),
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LockError::AlreadyLocked => write!(f, "the file is already locked"),
            LockError::Io(_) => write!(f, "the lock state could not be checked or set"),
        }
    }
}

impl std::error::Error for LockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LockError::AlreadyLocked => None,
            LockError::Io(e) => Some(e),
        }
    }
}

/// Commands for use with [`fcntl`].
#[allow(non_camel_case_types)]
enum FcntlArg<'a> {
    /// Set an OFD lock from the given lock description.
    F_OFD_SETLK(&'a libc::flock),
    /// Get the first OFD lock for the given lock description.
    F_OFD_GETLK(&'a mut libc::flock),
}

/// Wrapper for [`libc::fcntl`] that properly sets the function arguments.
fn fcntl(fd: RawFd, arg: FcntlArg) -> libc::c_int {
    // SAFETY: We pass a valid file descriptor along with a valid flock pointer
    // matching the requested OFD command.
    unsafe {
        match arg {
            FcntlArg::F_OFD_SETLK(flock) => libc::fcntl(fd, libc::F_OFD_SETLK, flock),
            FcntlArg::F_OFD_GETLK(flock) => libc::fcntl(fd, libc::F_OFD_GETLK, flock),
        }
    }
}

/// Describes the type of lock you want to set.
#[derive(Clone, Copy, Debug)]
pub enum LockType {
    /// Clear a lock.
    Unlock,
    /// Set a write lock (exclusive).
    Write,
    /// Set a read lock (shared).
    Read,
}

impl LockType {
    /// Returns the matching `l_type` value for [`struct@libc::flock`].
    pub const fn to_libc_val(self) -> libc::c_int {
        match self {
            Self::Unlock => libc::F_UNLCK as libc::c_int,
            Self::Write => libc::F_WRLCK as libc::c_int,
            Self::Read => libc::F_RDLCK as libc::c_int,
        }
    }
}

/// Describes the current state of a lock.
#[derive(Debug)]
pub enum LockState {
    /// No lock set.
    Unlocked,
    /// Locked for reading (non-exclusive).
    SharedRead,
    /// Locked for writing (exclusive mode).
    ExclusiveWrite,
}

impl LockState {
    fn new(value: libc::c_int) -> Self {
        const F_UNLCK: libc::c_int = libc::F_UNLCK as libc::c_int;
        const F_WRLCK: libc::c_int = libc::F_WRLCK as libc::c_int;
        const F_RDLCK: libc::c_int = libc::F_RDLCK as libc::c_int;
        match value {
            F_UNLCK => Self::Unlocked,
            F_WRLCK => Self::ExclusiveWrite,
            F_RDLCK => Self::SharedRead,
            // This is so unlikely that we want to avoid the complexity of
            // coping with this error case. Can only fail if either the kernel
            // is broken or memory is messed up.
            other => panic!("Unexpected lock state: {}", other),
        }
    }
}

/// The granularity of the advisory lock.
///
/// The granularity has significant implications in typical cloud deployments
/// with network storage. The Linux kernel will sync advisory locks to network
/// file systems, but these backends may have different policies and handle
/// locks differently. For example, NetApp speaks a NFS API but will treat
/// advisory OFD locks for the whole file as mandatory locks, whereas byte-range
/// locks for the whole file will remain advisory (see the [NetApp KB][netapp]).
///
/// As it is a valid use case to prevent multiple consumers from accessing the
/// same file (e.g. a disk image) while still allowing management software to
/// snapshot it, callers need control over the lock granularity. It is therefore
/// a valid use case to lock the whole byte range of a file without technically
/// locking the whole file - to get the best of both worlds.
///
/// [netapp]: https://kb.netapp.com/on-prem/ontap/da/NAS/NAS-KBs/How_is_Mandatory_Locking_supported_for_NFSv4_on_ONTAP_9
#[derive(Clone, Copy, Debug)]
pub enum LockGranularity {
    /// Lock the whole file (`l_start = 0`, `l_len = 0`).
    WholeFile,
    /// Lock the byte range `[from, from + len)`.
    ByteRange(u64 /* from, inclusive */, u64 /* len */),
}

impl LockGranularity {
    const fn l_start(self) -> u64 {
        match self {
            LockGranularity::WholeFile => 0,
            LockGranularity::ByteRange(start, _) => start,
        }
    }

    const fn l_len(self) -> u64 {
        match self {
            LockGranularity::WholeFile => 0, /* EOF */
            LockGranularity::ByteRange(_, len) => len,
        }
    }
}

/// Returns a [`struct@libc::flock`] structure for the requested parameters.
const fn get_flock(lock_type: LockType, granularity: LockGranularity) -> libc::flock {
    libc::flock {
        l_type: lock_type.to_libc_val() as libc::c_short,
        l_whence: libc::SEEK_SET as libc::c_short,
        l_start: granularity.l_start() as libc::off_t,
        l_len: granularity.l_len() as libc::off_t,
        l_pid: 0, /* filled by callee */
    }
}

/// Tries to acquire a lock using `fcntl()` with respect to the given
/// parameters.
///
/// Please note that `fcntl()` OFD locks are **advisory locks**, which do not
/// prevent to `open()` a file if a lock is already placed.
///
/// # Arguments
///
/// * `file`: The file to acquire a lock for. The file's state will be logically
///   mutated, but not technically.
/// * `lock_type`: The [`LockType`].
/// * `granularity`: The [`LockGranularity`].
///
/// # Examples
///
/// ```
/// use vmm_sys_util::fcntl::{try_acquire_lock, LockGranularity, LockType};
/// use vmm_sys_util::tempfile::TempFile;
///
/// let tmp = TempFile::new().unwrap();
/// try_acquire_lock(tmp.as_file(), LockType::Write, LockGranularity::WholeFile).unwrap();
/// ```
pub fn try_acquire_lock<Fd: AsRawFd>(
    file: &Fd,
    lock_type: LockType,
    granularity: LockGranularity,
) -> Result<(), LockError> {
    let flock = get_flock(lock_type, granularity);

    loop {
        let res = fcntl(file.as_raw_fd(), FcntlArg::F_OFD_SETLK(&flock));
        match res {
            0 => return Ok(()),
            -1 => {
                let io_error = io::Error::last_os_error();
                let errno = io_error.raw_os_error().unwrap();
                match errno {
                    // See man page for the error codes:
                    // <https://man7.org/linux/man-pages/man2/fcntl.2.html>
                    libc::EAGAIN | libc::EACCES => return Err(LockError::AlreadyLocked),
                    libc::EINTR => continue,
                    _ => return Err(LockError::Io(io_error)),
                }
            }
            val => panic!("Unexpected return value from fcntl(): {}", val),
        }
    }
}

/// Clears a lock.
///
/// # Arguments
///
/// * `file`: The file to clear all locks for.
/// * `granularity`: The [`LockGranularity`].
pub fn clear_lock<Fd: AsRawFd>(file: &Fd, granularity: LockGranularity) -> Result<(), LockError> {
    try_acquire_lock(file, LockType::Unlock, granularity)
}

/// Returns the current lock state using `fcntl()` with respect to the given
/// parameters.
///
/// # Arguments
///
/// * `file`: The file for which to get the lock state.
/// * `granularity`: The [`LockGranularity`].
pub fn get_lock_state<Fd: AsRawFd>(
    file: &Fd,
    granularity: LockGranularity,
) -> Result<LockState, LockError> {
    let mut flock = get_flock(LockType::Write, granularity);
    let res = fcntl(file.as_raw_fd(), FcntlArg::F_OFD_GETLK(&mut flock));
    match res {
        0 => {
            let state = flock.l_type as libc::c_int;
            let state = LockState::new(state);
            Ok(state)
        }
        -1 => {
            let io_error = io::Error::last_os_error();
            Err(LockError::Io(io_error))
        }
        val => panic!("Unexpected return value from fcntl(): {}", val),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tempfile::TempFile;

    #[test]
    fn test_acquire_and_clear_whole_file_lock() {
        let tmp = TempFile::new().unwrap();
        let file = tmp.as_file();

        // No lock yet.
        assert!(matches!(
            get_lock_state(file, LockGranularity::WholeFile).unwrap(),
            LockState::Unlocked
        ));

        // Acquire an exclusive (write) lock.
        try_acquire_lock(file, LockType::Write, LockGranularity::WholeFile).unwrap();

        // Re-acquiring through the same open file description succeeds (OFD locks
        // associated with the same description never conflict with themselves).
        try_acquire_lock(file, LockType::Write, LockGranularity::WholeFile).unwrap();

        // Clearing the lock works and leaves the file unlocked.
        clear_lock(file, LockGranularity::WholeFile).unwrap();
        assert!(matches!(
            get_lock_state(file, LockGranularity::WholeFile).unwrap(),
            LockState::Unlocked
        ));
    }

    #[test]
    fn test_conflicting_lock_is_reported() {
        let tmp = TempFile::new().unwrap();

        // Take an exclusive lock through the first open file description.
        try_acquire_lock(tmp.as_file(), LockType::Write, LockGranularity::WholeFile).unwrap();

        // A second, independent open file description must observe the conflict.
        let other = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(tmp.as_path())
            .unwrap();
        assert!(matches!(
            try_acquire_lock(&other, LockType::Write, LockGranularity::WholeFile),
            Err(LockError::AlreadyLocked)
        ));
        assert!(matches!(
            get_lock_state(&other, LockGranularity::WholeFile).unwrap(),
            LockState::ExclusiveWrite
        ));
    }

    #[test]
    fn test_byte_range_locks_do_not_overlap() {
        let tmp = TempFile::new().unwrap();
        try_acquire_lock(
            tmp.as_file(),
            LockType::Write,
            LockGranularity::ByteRange(0, 4096),
        )
        .unwrap();

        // A non-overlapping byte range on another description is free.
        let other = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(tmp.as_path())
            .unwrap();
        try_acquire_lock(
            &other,
            LockType::Write,
            LockGranularity::ByteRange(8192, 4096),
        )
        .unwrap();

        // ... but an overlapping range conflicts.
        assert!(matches!(
            try_acquire_lock(&other, LockType::Write, LockGranularity::ByteRange(0, 4096)),
            Err(LockError::AlreadyLocked)
        ));
    }
}
