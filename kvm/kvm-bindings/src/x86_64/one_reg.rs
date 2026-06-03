// SPDX-License-Identifier: Apache-2.0

//! Register ids for `KVM_GET_ONE_REG` and `KVM_SET_ONE_REG` on x86_64.
//!
//! Linux 6.18 added them to `arch/x86/include/uapi/asm/kvm.h`. The constants
//! are what bindgen emits for that header; the id builders mirror the
//! `KVM_X86_REG_MSR` and `KVM_X86_REG_KVM` macros, which are statement
//! expressions bindgen cannot translate.

use super::bindings::{KVM_REG_SIZE_U64, KVM_REG_X86};

/// The guest's current shadow stack pointer, a `KVM_X86_REG_TYPE_KVM` register.
pub use super::bindings::KVM_REG_GUEST_SSP;
/// Register type of a KVM defined register, indexed from 0.
pub use super::bindings::KVM_X86_REG_TYPE_KVM;
/// Register type of a model specific register, indexed by its MSR number.
pub use super::bindings::KVM_X86_REG_TYPE_MSR;

const fn kvm_x86_reg_id(reg_type: u32, size: u64, index: u32) -> u64 {
    KVM_REG_X86 | ((reg_type as u64) << 32) | size | index as u64
}

/// The `KVM_ONE_REG` id of the MSR `index`.
pub const fn kvm_x86_reg_msr(index: u32) -> u64 {
    kvm_x86_reg_id(KVM_X86_REG_TYPE_MSR, KVM_REG_SIZE_U64, index)
}

/// The `KVM_ONE_REG` id of the KVM defined register `index`.
pub const fn kvm_x86_reg_kvm(index: u32) -> u64 {
    let size = if index == KVM_REG_GUEST_SSP {
        KVM_REG_SIZE_U64
    } else {
        0
    };
    kvm_x86_reg_id(KVM_X86_REG_TYPE_KVM, size, index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guest_ssp_id_matches_the_kernel() {
        assert_eq!(kvm_x86_reg_kvm(KVM_REG_GUEST_SSP), 0x2030_0003_0000_0000);
    }

    #[test]
    fn test_msr_id_carries_type_size_and_index() {
        assert_eq!(kvm_x86_reg_msr(0xc000_0080), 0x2030_0002_c000_0080);
    }

    #[test]
    fn test_unknown_kvm_register_has_no_size() {
        assert_eq!(kvm_x86_reg_kvm(1), 0x2000_0003_0000_0001);
    }
}
