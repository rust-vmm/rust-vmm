// Copyright © 2020, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//
use mshv_bindings::{mshv_root_hvcall, HvError, HV_STATUS_SUCCESS};
use thiserror::Error;
use vmm_sys_util::errno;

pub mod device;
pub mod system;
pub mod vcpu;
pub mod vm;

/// A specialized `Error` type for MSHV ioctls
///
/// Exposes either a regular linux errno, or details of a hypercall-related
/// error.
///
/// For convenience, it can always be converted into an errno::Error
#[derive(Error, Debug, Copy, Clone, PartialEq)]
pub enum MshvError {
    /// A regular linux errno
    #[error("Kernel returned errno: {0}")]
    Errno(#[from] errno::Error),

    /// A failed hypercall
    ///
    /// Retains the hypercall code, and raw status code. The status code is
    /// converted to HvError enum for readability, if possible
    ///
    /// In case the caller requires an errno, this variant can be still be
    /// converted to an EIO with into(), from(), or errno() for the raw value
    #[error("Hypercall {code} failed with {status_raw:#x} : {}", status.map_or("Unknown".to_string(), |s| format!("{s:?}")))]
    Hypercall {
        /// The control code, i.e. what type of hypercall it was
        code: u16,
        /// The status or result code returned from the hypercall
        status_raw: u16,
        /// The status as an enum for pretty-printing
        status: Option<HvError>,
    },
}

impl MshvError {
    /// Create a MshvError from the output of the MSHV_ROOT_HVCALL IOCTL
    /// # Arguments
    /// * `error` - errno from the ioctl; usually errno:Error::last()
    /// * `ret_args` - MSHV_ROOT_HVCALL args struct, after the ioctl completed
    pub fn from_hvcall(error: errno::Error, ret_args: mshv_root_hvcall) -> Self {
        if ret_args.status != HV_STATUS_SUCCESS as u16 {
            let hv_err = HvError::try_from(ret_args.status);
            return MshvError::Hypercall {
                code: ret_args.code,
                status_raw: ret_args.status,
                status: hv_err.ok(),
            };
        }
        error.into()
    }

    /// Convert to error code. Analogous to errno::Error::errno()
    pub fn errno(self) -> i32 {
        errno::Error::from(self).errno()
    }
}

impl From<i32> for MshvError {
    fn from(err: i32) -> Self {
        MshvError::Errno(errno::Error::new(err))
    }
}

impl From<MshvError> for errno::Error {
    fn from(err: MshvError) -> Self {
        match err {
            MshvError::Errno(e) => e,
            MshvError::Hypercall { .. } => errno::Error::new(libc::EIO),
        }
    }
}

impl From<MshvError> for std::io::Error {
    fn from(err: MshvError) -> Self {
        errno::Error::from(err).into()
    }
}

/// A specialized `Result` type for MSHV ioctls.
///
/// This typedef is generally used to avoid writing out errno::Error directly and
/// is otherwise a direct mapping to Result.
pub type Result<T> = std::result::Result<T, MshvError>;

/// Set bits by index and OR them together
#[macro_export]
macro_rules! set_bits {
    ($int_type:ty, $bit:expr) => {{
        let bit: $int_type = ((1 as $int_type) << $bit);
        bit
    }};
    ($int_type:ty, $bit:expr, $($bits:expr),+) => {{
        set_bits!($int_type, $bit) | set_bits!($int_type, $($bits),+)
    }};
}

#[cfg(test)]
mod tests {
    use crate::MshvError;
    use mshv_bindings::*;
    use vmm_sys_util::errno;

    #[test]
    fn test_hv_status_from_mshv_root_hvcall() {
        let ioctl_err = errno::Error::new(libc::EIO);

        {
            let args = mshv_root_hvcall {
                code: HVCALL_GET_VP_REGISTERS as u16,
                status: HV_STATUS_INVALID_PARAMETER as u16,
                ..Default::default()
            };
            let mshv_err = MshvError::from_hvcall(ioctl_err, args);
            let mshv_err_check = MshvError::Hypercall {
                code: HVCALL_GET_VP_REGISTERS as u16,
                status_raw: HV_STATUS_INVALID_PARAMETER as u16,
                status: Some(HvError::InvalidParameter),
            };
            assert!(mshv_err == mshv_err_check);
        }
        {
            // special case: invalid hv status
            let args = mshv_root_hvcall {
                code: HVCALL_GET_VP_REGISTERS as u16,
                status: 0xFFFF,
                ..Default::default()
            };
            let mshv_err = MshvError::from_hvcall(ioctl_err, args);
            let mshv_err_check = MshvError::Hypercall {
                code: HVCALL_GET_VP_REGISTERS as u16,
                status_raw: 0xFFFF,
                status: None,
            };
            assert!(mshv_err == mshv_err_check);
        }
    }

    #[test]
    fn test_errno_from_mshv_root_hvcall() {
        let args = mshv_root_hvcall {
            code: HVCALL_GET_VP_REGISTERS as u16,
            status: HV_STATUS_SUCCESS as u16,
            ..Default::default()
        };

        {
            let ioctl_err = errno::Error::new(libc::EINVAL);
            let mshv_err = MshvError::from_hvcall(ioctl_err, args);

            assert!(mshv_err == MshvError::Errno(ioctl_err));
        }
    }

    #[test]
    fn test_set_bits() {
        assert!(set_bits!(u8, 0) == 1_u8);
        assert!(set_bits!(u8, 1) == 2_u8);
        assert!(set_bits!(u8, 0, 1) == 3_u8);
        assert!(set_bits!(u8, 2) == 4_u8);
        assert!(set_bits!(u8, 0, 2) == 5_u8);

        assert!(set_bits!(u16, 0) == 1_u16);
        assert!(set_bits!(u32, 0) == 1_u32);
        assert!(set_bits!(u64, 0) == 1_u64);
    }
}
