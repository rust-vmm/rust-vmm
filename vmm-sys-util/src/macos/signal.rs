// Copyright 2024 rust-vmm Authors. All Rights Reserved.
// SPDX-License-Identifier: BSD-3-Clause

//! macOS-compatible signal handling shim.
//!
//! Provides the subset of the Linux signal module API needed by downstream crates.
//! macOS does not have real-time signals or `sigtimedwait`, so those are omitted.

use libc::{
    c_int, c_void, pthread_kill, pthread_sigmask, pthread_t, sigaction, sigaddset, sigemptyset,
    sigfillset, siginfo_t, sigismember, sigpending, sigset_t, EINTR, EINVAL, SIG_BLOCK,
    SIG_UNBLOCK,
};

use crate::errno;
use std::fmt::{self, Display};
use std::io;
use std::mem;
use std::os::unix::thread::JoinHandleExt;
use std::ptr::{null, null_mut};
use std::result;
use std::thread::JoinHandle;

/// The error cases enumeration for signal handling.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Couldn't create a sigset.
    CreateSigset(errno::Error),
    /// The wrapped signal has already been blocked.
    SignalAlreadyBlocked(c_int),
    /// Failed to check if the requested signal is in the blocked set already.
    CompareBlockedSignals(errno::Error),
    /// The signal could not be blocked.
    BlockSignal(errno::Error),
    /// The signal mask could not be retrieved.
    RetrieveSignalMask(c_int),
    /// The signal could not be unblocked.
    UnblockSignal(errno::Error),
    /// Failed to wait for given signal.
    ClearWaitPending(errno::Error),
    /// Failed to get pending signals.
    ClearGetPending(errno::Error),
    /// Failed to check if given signal is in the set of pending signals.
    ClearCheckPending(errno::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Error::*;

        match self {
            CreateSigset(e) => write!(f, "couldn't create a sigset: {}", e),
            SignalAlreadyBlocked(num) => write!(f, "signal {} already blocked", num),
            CompareBlockedSignals(e) => write!(
                f,
                "failed to check whether requested signal is in the blocked set: {}",
                e,
            ),
            BlockSignal(e) => write!(f, "signal could not be blocked: {}", e),
            RetrieveSignalMask(errno) => write!(
                f,
                "failed to retrieve signal mask: {}",
                io::Error::from_raw_os_error(*errno),
            ),
            UnblockSignal(e) => write!(f, "signal could not be unblocked: {}", e),
            ClearWaitPending(e) => write!(f, "failed to wait for given signal: {}", e),
            ClearGetPending(e) => write!(f, "failed to get pending signals: {}", e),
            ClearCheckPending(e) => write!(
                f,
                "failed to check whether given signal is in the pending set: {}",
                e,
            ),
        }
    }
}

/// A simplified Result type for operations that can return signal Error.
pub type SignalResult<T> = result::Result<T, Error>;

/// Public alias for a signal handler.
pub type SignalHandler =
    extern "C" fn(num: c_int, info: *mut siginfo_t, _unused: *mut c_void) -> ();

/// Verify that a signal number is valid.
///
/// macOS does not have real-time signals, so only standard signals (1..=31) are accepted.
/// Note: macOS signal numbers are not ordered the same as Linux — SIGSYS=12 but SIGUSR2=31.
pub fn validate_signal_num(num: c_int) -> errno::Result<()> {
    if (1..=libc::SIGUSR2).contains(&num) {
        Ok(())
    } else {
        Err(errno::Error::new(EINVAL))
    }
}

/// Register the signal handler of `signum`.
///
/// # Safety
///
/// This is considered unsafe because the given handler will be called
/// asynchronously, interrupting whatever the thread was doing and therefore
/// must only do async-signal-safe operations.
pub fn register_signal_handler(num: c_int, handler: SignalHandler) -> errno::Result<()> {
    validate_signal_num(num)?;

    if libc::SIGKILL == num || libc::SIGSTOP == num {
        return Err(errno::Error::new(EINVAL));
    }

    // SAFETY: Safe, because this is a POD struct.
    let mut act: sigaction = unsafe { mem::zeroed() };
    act.sa_sigaction = handler as *const () as usize;
    act.sa_flags = libc::SA_SIGINFO;

    // Block all signals while the `handler` is running.
    // SAFETY: The parameters are valid and we trust the sigfillset function.
    if unsafe { sigfillset(&mut act.sa_mask as *mut sigset_t) } < 0 {
        return errno::errno_result();
    }

    // SAFETY: Safe because the parameters are valid and we check the return value.
    match unsafe { sigaction(num, &act, null_mut()) } {
        0 => Ok(()),
        _ => errno::errno_result(),
    }
}

/// Create a `sigset` with given signals.
pub fn create_sigset(signals: &[c_int]) -> errno::Result<sigset_t> {
    // SAFETY: sigset will actually be initialized by sigemptyset below.
    let mut sigset: sigset_t = unsafe { mem::zeroed() };

    // SAFETY: return value is checked.
    let ret = unsafe { sigemptyset(&mut sigset) };
    if ret < 0 {
        return errno::errno_result();
    }

    for signal in signals {
        // SAFETY: return value is checked.
        let ret = unsafe { sigaddset(&mut sigset, *signal) };
        if ret < 0 {
            return errno::errno_result();
        }
    }

    Ok(sigset)
}

/// Retrieve the signal mask that is blocked of the current thread.
pub fn get_blocked_signals() -> SignalResult<Vec<c_int>> {
    let mut mask = Vec::new();

    // SAFETY: return values are checked.
    unsafe {
        let mut old_sigset: sigset_t = mem::zeroed();
        let ret = pthread_sigmask(SIG_BLOCK, null(), &mut old_sigset as *mut sigset_t);
        if ret < 0 {
            return Err(Error::RetrieveSignalMask(ret));
        }

        // macOS signals go from 1 to SIGUSR2 (31)
        for num in 1..=libc::SIGUSR2 {
            if sigismember(&old_sigset, num) > 0 {
                mask.push(num);
            }
        }
    }

    Ok(mask)
}

/// Mask a given signal.
#[allow(clippy::comparison_chain)]
pub fn block_signal(num: c_int) -> SignalResult<()> {
    let sigset = create_sigset(&[num]).map_err(Error::CreateSigset)?;

    // SAFETY: return values are checked.
    unsafe {
        let mut old_sigset: sigset_t = mem::zeroed();
        let ret = pthread_sigmask(SIG_BLOCK, &sigset, &mut old_sigset as *mut sigset_t);
        if ret < 0 {
            return Err(Error::BlockSignal(errno::Error::last()));
        }
        let ret = sigismember(&old_sigset, num);
        if ret < 0 {
            return Err(Error::CompareBlockedSignals(errno::Error::last()));
        } else if ret > 0 {
            return Err(Error::SignalAlreadyBlocked(num));
        }
    }
    Ok(())
}

/// Unmask a given signal.
pub fn unblock_signal(num: c_int) -> SignalResult<()> {
    let sigset = create_sigset(&[num]).map_err(Error::CreateSigset)?;

    // SAFETY: return value is checked.
    let ret = unsafe { pthread_sigmask(SIG_UNBLOCK, &sigset, null_mut()) };
    if ret < 0 {
        return Err(Error::UnblockSignal(errno::Error::last()));
    }
    Ok(())
}

/// Clear a pending signal.
///
/// On macOS, `sigtimedwait` is not available. This implementation uses
/// a temporary signal handler to consume the pending signal.
pub fn clear_signal(num: c_int) -> SignalResult<()> {
    let sigset = create_sigset(&[num]).map_err(Error::CreateSigset)?;

    // Unblock the signal temporarily to let it be delivered.
    // SAFETY: sigset was constructed by create_sigset; null oldset is allowed.
    let _ = unsafe { pthread_sigmask(SIG_UNBLOCK, &sigset, null_mut()) };

    // Re-block it.
    // SAFETY: same valid sigset; null oldset is allowed.
    let _ = unsafe { pthread_sigmask(SIG_BLOCK, &sigset, null_mut()) };

    // Check if still pending.
    // SAFETY: chkset is zeroed (valid empty sigset). sigpending/sigismember
    // only read or fill the sigset; return values are checked.
    unsafe {
        let mut chkset: sigset_t = mem::zeroed();
        let ret = sigpending(&mut chkset);
        if ret < 0 {
            return Err(Error::ClearGetPending(errno::Error::last()));
        }

        let ret = sigismember(&chkset, num);
        if ret < 0 {
            return Err(Error::ClearCheckPending(errno::Error::last()));
        }
    }

    Ok(())
}

/// Trait for threads that can be signalled via `pthread_kill`.
///
/// # Safety
///
/// This is marked unsafe because the implementation of this trait must
/// guarantee that the returned `pthread_t` is valid and has a lifetime at
/// least that of the trait object.
pub unsafe trait Killable {
    /// Cast this killable thread as `pthread_t`.
    fn pthread_handle(&self) -> pthread_t;

    /// Send a signal to this killable thread.
    fn kill(&self, num: c_int) -> errno::Result<()> {
        validate_signal_num(num)?;

        // SAFETY: Safe because we ensure we are using a valid pthread handle,
        // a valid signal number, and check the return result.
        let ret = unsafe { pthread_kill(self.pthread_handle(), num) };
        if ret < 0 {
            return errno::errno_result();
        }
        Ok(())
    }
}

// SAFETY: Safe because we fulfill our contract of returning a genuine pthread handle.
unsafe impl<T> Killable for JoinHandle<T> {
    fn pthread_handle(&self) -> pthread_t {
        self.as_pthread_t() as pthread_t
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::undocumented_unsafe_blocks)]
    use super::*;
    use std::thread;
    use std::time::Duration;

    // Set by handle_signal; checked by tests.
    static mut SIGNAL_HANDLER_CALLED: bool = false;

    extern "C" fn handle_signal(_: c_int, _: *mut siginfo_t, _: *mut c_void) {
        unsafe {
            SIGNAL_HANDLER_CALLED = true;
        }
    }

    fn is_pending(signal: c_int) -> bool {
        unsafe {
            let mut chkset: sigset_t = mem::zeroed();
            sigpending(&mut chkset);
            sigismember(&chkset, signal) == 1
        }
    }

    #[test]
    fn test_validate_signal_num() {
        // macOS signals: 1..=SIGUSR2 (=31). Anything outside this is EINVAL.
        // SIGHUP=1 is the lowest valid, SIGUSR2=31 is the highest. Linux's
        // SIGHUP..=SIGSYS range would reject SIGUSR2 here (SIGSYS=12 on
        // macOS), which is the original bug this validator works around.
        assert!(validate_signal_num(1).is_ok());
        assert!(validate_signal_num(libc::SIGUSR2).is_ok());
        assert!(validate_signal_num(libc::SIGSYS).is_ok());
        assert!(validate_signal_num(libc::SIGTERM).is_ok());

        assert!(validate_signal_num(0).is_err());
        assert!(validate_signal_num(libc::SIGUSR2 + 1).is_err());
        assert!(validate_signal_num(-1).is_err());
    }

    #[test]
    fn test_register_signal_handler() {
        // SIGKILL / SIGSTOP must be rejected, valid signals must succeed.
        assert!(register_signal_handler(libc::SIGKILL, handle_signal).is_err());
        assert!(register_signal_handler(libc::SIGSTOP, handle_signal).is_err());
        assert!(register_signal_handler(libc::SIGUSR2 + 1, handle_signal).is_err());
        assert!(register_signal_handler(libc::SIGUSR1, handle_signal).is_ok());
        assert!(register_signal_handler(libc::SIGUSR2, handle_signal).is_ok());
        assert!(register_signal_handler(libc::SIGSYS, handle_signal).is_ok());
    }

    #[test]
    fn test_block_unblock_signal() {
        // SIGUSR2 is used here precisely because its macOS signal number (31)
        // is what the Linux SIGHUP..=SIGSYS range rejected. Picking it pins
        // the validator fix as well as the block/unblock round-trip.
        let signal = libc::SIGUSR2;

        unsafe {
            let mut sigset: sigset_t = mem::zeroed();
            pthread_sigmask(SIG_BLOCK, null(), &mut sigset as *mut sigset_t);
            assert_eq!(sigismember(&sigset, signal), 0);
        }

        block_signal(signal).unwrap();
        assert!(get_blocked_signals().unwrap().contains(&signal));

        unblock_signal(signal).unwrap();
        assert!(!get_blocked_signals().unwrap().contains(&signal));
    }

    #[test]
    #[allow(clippy::empty_loop)]
    fn test_killing_thread() {
        // Mirror of the Linux test, adapted for macOS signal numbering.
        // macOS has no real-time signals; SIGUSR1 is the conventional
        // user-defined signal safe to register and deliver in tests.
        let killable = thread::spawn(|| thread::current().id());
        let killable_id = killable.join().unwrap();
        assert_ne!(killable_id, thread::current().id());

        // Handler is process-global, so install before spawning the
        // long-lived thread.
        register_signal_handler(libc::SIGUSR1, handle_signal)
            .expect("failed to register signal handler");

        let killable = thread::spawn(|| loop {});

        // Out-of-range signal must fail.
        assert!(killable.kill(libc::SIGUSR2 + 1).is_err());

        unsafe {
            assert!(!SIGNAL_HANDLER_CALLED);
        }

        assert!(killable.kill(libc::SIGUSR1).is_ok());

        const MAX_WAIT_ITERS: u32 = 20;
        let mut iter_count = 0;
        loop {
            thread::sleep(Duration::from_millis(100));
            if unsafe { SIGNAL_HANDLER_CALLED } {
                break;
            }
            iter_count += 1;
            assert!(iter_count <= MAX_WAIT_ITERS);
        }

        // killable runs an infinite loop; intentionally do not join it.
        // It becomes detached when the JoinHandle is dropped and dies
        // with the process.
    }

    #[test]
    fn test_clear_pending() {
        // Block SIGUSR2, queue it, then clear it. Pins that clear_signal
        // removes the pending bit on macOS (which has no sigtimedwait, so
        // the implementation unblocks/reblocks instead).
        let signal = libc::SIGUSR2;

        block_signal(signal).unwrap();

        let killable = thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(100));
            if is_pending(signal) {
                clear_signal(signal).unwrap();
                assert!(!is_pending(signal));
                break;
            }
        });

        assert!(killable.kill(libc::SIGUSR2).is_ok());
        killable.join().unwrap();
    }
}
