// Copyright © 2020, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//

#[cfg(target_arch = "x86_64")]
#[macro_use]
#[cfg(feature = "fam-wrappers")]
extern crate vmm_sys_util;

#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use self::x86_64::*;

#[cfg(target_arch = "aarch64")]
mod arm64;
#[cfg(target_arch = "aarch64")]
pub use self::arm64::*;

pub mod hvdef;
pub use hvdef::*;

pub mod hvcall;
pub use hvcall::*;

#[derive(Debug)]
pub struct RegisterPage(pub *mut hv_vp_register_page);

// SAFETY: struct is based on register page in the hypervisor,
// safe to Send across threads
unsafe impl Send for RegisterPage {}

// SAFETY: struct is based on Register page in the hypervisor,
// safe to Sync across threads as this is only required for Vcpu trait
// functionally not used anyway
unsafe impl Sync for RegisterPage {}
