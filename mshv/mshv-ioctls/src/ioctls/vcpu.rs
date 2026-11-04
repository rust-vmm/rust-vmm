// Copyright © 2020, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//
use crate::ioctls::{MshvError, Result};
use crate::mshv_ioctls::*;
use mshv_bindings::*;
#[cfg(target_arch = "x86_64")]
use std::convert::TryFrom;
use std::fs::File;
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(all(test, target_arch = "x86_64"))]
use std::slice;
use vmm_sys_util::errno;
use vmm_sys_util::ioctl::ioctl_with_mut_ref;
#[cfg(target_arch = "x86_64")]
use vmm_sys_util::ioctl::ioctl_with_ref;

// Macro for setting up multiple 64 bit registers together
// Arguments:
///             1. vcpud fd
///             2. Array of Tuples of Register name and reguster value Example [(n1, v1), (n2,v2) ....]
#[allow(unused_macros)]
#[macro_export]
macro_rules! set_registers_64 {
    ($vcpu:expr, $arr_t:expr ) => {{
        let len = $arr_t.len();
        // Initialize with zero which is itself a enum value(HV_REGISTER_EXPLICIT_SUSPEND = 0).
        // This value does not have any effect as this is being overwritten anyway.
        let mut assocs: Vec<hv_register_assoc> = vec![
            hv_register_assoc {
                ..Default::default()
            };
            len
        ];
        for (i, x) in $arr_t.iter().enumerate() {
            let (a, b) = x;
            assocs[i].name = *a as u32;
            assocs[i].value = hv_register_value { reg64: *b };
        }
        #[allow(unused_parens)]
        $vcpu.set_reg(&assocs)
    }};
}

#[derive(Debug)]
/// Wrapper over Mshv vCPU ioctls.
pub struct VcpuFd {
    index: u32,
    vcpu: File,
    vp_page: Option<RegisterPage>,
}

/// Helper function to create a new `VcpuFd`.
///
/// This should not be exported as a public function because the preferred way is to use
/// `create_vcpu` from `VmFd`. The function cannot be part of the `VcpuFd` implementation because
/// then it would be exported with the public `VcpuFd` interface.
pub fn new_vcpu(index: u32, vcpu: File, vp_page: Option<RegisterPage>) -> VcpuFd {
    VcpuFd {
        index,
        vcpu,
        vp_page,
    }
}

impl AsRawFd for VcpuFd {
    fn as_raw_fd(&self) -> RawFd {
        self.vcpu.as_raw_fd()
    }
}

// Helper function to update interrupt bitmap in the SpecialRegisters struct
// This function is used by the two functions
// that get special registers.
#[cfg(not(target_arch = "aarch64"))]
fn update_interrupt_bitmap(ret_regs: &mut SpecialRegisters, pending_reg: u64) {
    if (pending_reg & 0x1) == 1 && // interruption pending
    (pending_reg >> 1).trailing_zeros() >= 3
    {
        // interrupt type external
        let interrupt_nr = (pending_reg >> 16) & 0xFFFF;
        if interrupt_nr > 255 {
            panic!("Invalid interrupt vector number > 255");
        }
        // we have a bit array of 4 u64s, so we can split it to get the array index and the
        // bit index
        let bit_offset = interrupt_nr & 0x3F; // 6 bits = 0-63
        let index = interrupt_nr >> 6;
        ret_regs.interrupt_bitmap[index as usize] |= 1 << bit_offset;
    }
}

#[cfg(not(target_arch = "aarch64"))]
static NON_VP_PAGE_SP_REGS: &[::std::os::raw::c_uint; 7] = &[
    hv_register_name_HV_X64_REGISTER_TR,
    hv_register_name_HV_X64_REGISTER_LDTR,
    hv_register_name_HV_X64_REGISTER_GDTR,
    hv_register_name_HV_X64_REGISTER_IDTR,
    hv_register_name_HV_X64_REGISTER_CR2,
    hv_register_name_HV_X64_REGISTER_APIC_BASE,
    hv_register_name_HV_REGISTER_PENDING_INTERRUPTION,
];

#[cfg(not(target_arch = "aarch64"))]
static VP_PAGE_SP_REGS: &[::std::os::raw::c_uint; 11] = &[
    hv_register_name_HV_X64_REGISTER_CS,
    hv_register_name_HV_X64_REGISTER_DS,
    hv_register_name_HV_X64_REGISTER_ES,
    hv_register_name_HV_X64_REGISTER_FS,
    hv_register_name_HV_X64_REGISTER_GS,
    hv_register_name_HV_X64_REGISTER_SS,
    hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR0,
    hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR3,
    hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR4,
    hv_register_name_HV_X64_REGISTER_CR8,
    hv_register_name_HV_X64_REGISTER_EFER,
];

impl VcpuFd {
    /// Get the reference of VP register page
    pub fn get_vp_reg_page(&self) -> Option<&RegisterPage> {
        self.vp_page.as_ref()
    }

    /// Check if the VP register page is valid
    #[cfg(not(target_arch = "aarch64"))]
    fn is_valid_vp_reg_page(&self) -> bool {
        let vp_reg_page = match self.get_vp_reg_page() {
            Some(page) => page.0,
            None => return false,
        };
        unsafe { (*vp_reg_page).isvalid != 0 }
    }

    /// Get the register values by providing an array of register names
    pub fn get_reg(&self, reg_names: &mut [hv_register_assoc]) -> Result<()> {
        self.hvcall_get_reg(reg_names)
    }
    /// Generic hvcall version of get_reg
    fn hvcall_get_reg(&self, reg_assocs: &mut [hv_register_assoc]) -> Result<()> {
        if reg_assocs.is_empty() {
            return Err(libc::EINVAL.into());
        }
        let reg_names: Vec<hv_register_name> = reg_assocs.iter().map(|assoc| assoc.name).collect();
        let input = make_rep_input!(
            hv_input_get_vp_registers {
                vp_index: self.index,
                ..Default::default()
            },
            names,
            reg_names.as_slice()
        );
        let mut output: Vec<hv_register_value> = reg_names
            .iter()
            .map(|_| hv_register_value {
                reg128: hv_u128 {
                    ..Default::default()
                },
            })
            .collect();
        let output_slice = output.as_mut_slice();

        let mut args = make_rep_args!(HVCALL_GET_VP_REGISTERS, input, output_slice);
        self.hvcall(&mut args)?;

        if args.reps as usize != reg_assocs.len() {
            return Err(libc::EINTR.into());
        }

        for (assoc, value) in reg_assocs.iter_mut().zip(output.iter()) {
            assoc.value = *value;
        }

        Ok(())
    }
    /// Set vcpu register values by providing an array of register assocs
    pub fn set_reg(&self, regs: &[hv_register_assoc]) -> Result<()> {
        self.hvcall_set_reg(regs)
    }
    /// Generic hypercall version of set_reg
    fn hvcall_set_reg(&self, reg_assocs: &[hv_register_assoc]) -> Result<()> {
        let input = make_rep_input!(
            hv_input_set_vp_registers {
                vp_index: self.index,
                ..Default::default()
            },
            elements,
            reg_assocs
        );
        let mut args = make_rep_args!(HVCALL_SET_VP_REGISTERS, input);
        self.hvcall(&mut args)?;

        if args.reps as usize != reg_assocs.len() {
            return Err(libc::EINTR.into());
        }

        Ok(())
    }

    /// Sets the vCPU general purpose registers using the IOCTL
    #[cfg(not(target_arch = "aarch64"))]
    fn set_standard_regs_ioctl(&self, regs: &StandardRegisters) -> Result<()> {
        let reg_assocs = [
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RAX,
                value: hv_register_value { reg64: regs.rax },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RBX,
                value: hv_register_value { reg64: regs.rbx },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RCX,
                value: hv_register_value { reg64: regs.rcx },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RDX,
                value: hv_register_value { reg64: regs.rdx },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RSI,
                value: hv_register_value { reg64: regs.rsi },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RDI,
                value: hv_register_value { reg64: regs.rdi },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RSP,
                value: hv_register_value { reg64: regs.rsp },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RBP,
                value: hv_register_value { reg64: regs.rbp },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_R8,
                value: hv_register_value { reg64: regs.r8 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_R9,
                value: hv_register_value { reg64: regs.r9 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_R10,
                value: hv_register_value { reg64: regs.r10 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_R11,
                value: hv_register_value { reg64: regs.r11 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_R12,
                value: hv_register_value { reg64: regs.r12 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_R13,
                value: hv_register_value { reg64: regs.r13 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_R14,
                value: hv_register_value { reg64: regs.r14 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_R15,
                value: hv_register_value { reg64: regs.r15 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RIP,
                value: hv_register_value { reg64: regs.rip },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RFLAGS,
                value: hv_register_value { reg64: regs.rflags },
                ..Default::default()
            },
        ];
        self.set_reg(&reg_assocs)?;
        Ok(())
    }
    /// Sets the vCPU general purpose registers on ARM64
    #[cfg(target_arch = "aarch64")]
    pub fn set_regs(&self, regs: &StandardRegisters) -> Result<()> {
        let mut reg_assocs = Vec::with_capacity(38);

        for i in 0..29 as usize {
            reg_assocs.push(hv_register_assoc {
                name: hv_register_name_HV_ARM64_REGISTER_X0 + i as u32,
                value: hv_register_value {
                    reg64: regs.regs[i],
                },
                ..Default::default()
            });
        }

        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_FP,
            value: hv_register_value {
                reg64: regs.regs[29],
            },
            ..Default::default()
        });

        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_LR,
            value: hv_register_value {
                reg64: regs.regs[30],
            },
            ..Default::default()
        });

        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_SP,
            value: hv_register_value { reg64: regs.sp },
            ..Default::default()
        });

        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_PC,
            value: hv_register_value { reg64: regs.pc },
            ..Default::default()
        });

        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_PSTATE,
            value: hv_register_value { reg64: regs.pstate },
            ..Default::default()
        });

        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_SP_EL1,
            value: hv_register_value { reg64: regs.sp_el1 },
            ..Default::default()
        });

        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_ELR_EL1,
            value: hv_register_value {
                reg64: regs.elr_el1,
            },
            ..Default::default()
        });

        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_FPSR,
            value: hv_register_value { reg64: regs.fpsr },
            ..Default::default()
        });

        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_FPCR,
            value: hv_register_value { reg64: regs.fpcr },
            ..Default::default()
        });

        self.hvcall_set_reg(&reg_assocs)?;
        Ok(())
    }
    /// Sets the vCPU general purpose registers using VP register page
    #[cfg(not(target_arch = "aarch64"))]
    fn set_standard_regs_vp_page(&self, regs: &StandardRegisters) -> Result<()> {
        let vp_reg_page = self.get_vp_reg_page().unwrap().0;
        set_gp_regs_field_ptr!(vp_reg_page, rax, regs.rax);
        set_gp_regs_field_ptr!(vp_reg_page, rbx, regs.rbx);
        set_gp_regs_field_ptr!(vp_reg_page, rcx, regs.rcx);
        set_gp_regs_field_ptr!(vp_reg_page, rdx, regs.rdx);
        set_gp_regs_field_ptr!(vp_reg_page, rsi, regs.rsi);
        set_gp_regs_field_ptr!(vp_reg_page, rdi, regs.rdi);
        set_gp_regs_field_ptr!(vp_reg_page, rsp, regs.rsp);
        set_gp_regs_field_ptr!(vp_reg_page, rbp, regs.rbp);
        set_gp_regs_field_ptr!(vp_reg_page, r8, regs.r8);
        set_gp_regs_field_ptr!(vp_reg_page, r9, regs.r9);
        set_gp_regs_field_ptr!(vp_reg_page, r10, regs.r10);
        set_gp_regs_field_ptr!(vp_reg_page, r11, regs.r11);
        set_gp_regs_field_ptr!(vp_reg_page, r12, regs.r12);
        set_gp_regs_field_ptr!(vp_reg_page, r13, regs.r13);
        set_gp_regs_field_ptr!(vp_reg_page, r14, regs.r14);
        set_gp_regs_field_ptr!(vp_reg_page, r15, regs.r15);

        // SAFETY: access union fields
        unsafe {
            (*vp_reg_page).dirty |= 1 << HV_X64_REGISTER_CLASS_GENERAL;
            (*vp_reg_page).__bindgen_anon_1.__bindgen_anon_1.rip = regs.rip;
            (*vp_reg_page).dirty |= 1 << HV_X64_REGISTER_CLASS_IP;
            (*vp_reg_page).__bindgen_anon_1.__bindgen_anon_1.rflags = regs.rflags;
            (*vp_reg_page).dirty |= 1 << HV_X64_REGISTER_CLASS_FLAGS;
        }
        Ok(())
    }

    /// Sets the vCPU general purpose registers
    #[cfg(not(target_arch = "aarch64"))]
    pub fn set_regs(&self, regs: &StandardRegisters) -> Result<()> {
        if self.is_valid_vp_reg_page() {
            self.set_standard_regs_vp_page(regs)
        } else {
            self.set_standard_regs_ioctl(regs)
        }
    }

    /// Returns the vCPU general purpose registers.
    #[cfg(target_arch = "x86_64")]
    pub fn get_regs(&self) -> Result<StandardRegisters> {
        if self.is_valid_vp_reg_page() {
            self.get_standard_regs_vp_page()
        } else {
            self.get_standard_regs_ioctl()
        }
    }

    /// Returns the vCPU general purpose registers using IOCTL
    #[cfg(not(target_arch = "aarch64"))]
    fn get_standard_regs_ioctl(&self) -> Result<StandardRegisters> {
        let reg_names = [
            hv_register_name_HV_X64_REGISTER_RAX,
            hv_register_name_HV_X64_REGISTER_RBX,
            hv_register_name_HV_X64_REGISTER_RCX,
            hv_register_name_HV_X64_REGISTER_RDX,
            hv_register_name_HV_X64_REGISTER_RSI,
            hv_register_name_HV_X64_REGISTER_RDI,
            hv_register_name_HV_X64_REGISTER_RSP,
            hv_register_name_HV_X64_REGISTER_RBP,
            hv_register_name_HV_X64_REGISTER_R8,
            hv_register_name_HV_X64_REGISTER_R9,
            hv_register_name_HV_X64_REGISTER_R10,
            hv_register_name_HV_X64_REGISTER_R11,
            hv_register_name_HV_X64_REGISTER_R12,
            hv_register_name_HV_X64_REGISTER_R13,
            hv_register_name_HV_X64_REGISTER_R14,
            hv_register_name_HV_X64_REGISTER_R15,
            hv_register_name_HV_X64_REGISTER_RIP,
            hv_register_name_HV_X64_REGISTER_RFLAGS,
        ];

        let mut reg_assocs: [hv_register_assoc; 18] = [hv_register_assoc::default(); 18];
        for (it, elem) in reg_assocs.iter_mut().zip(reg_names) {
            it.name = elem;
        }

        self.get_reg(&mut reg_assocs)?;
        let mut ret_regs = StandardRegisters::default();
        // SAFETY: access union fields
        unsafe {
            ret_regs.rax = reg_assocs[0].value.reg64;
            ret_regs.rbx = reg_assocs[1].value.reg64;
            ret_regs.rcx = reg_assocs[2].value.reg64;
            ret_regs.rdx = reg_assocs[3].value.reg64;
            ret_regs.rsi = reg_assocs[4].value.reg64;
            ret_regs.rdi = reg_assocs[5].value.reg64;
            ret_regs.rsp = reg_assocs[6].value.reg64;
            ret_regs.rbp = reg_assocs[7].value.reg64;
            ret_regs.r8 = reg_assocs[8].value.reg64;
            ret_regs.r9 = reg_assocs[9].value.reg64;
            ret_regs.r10 = reg_assocs[10].value.reg64;
            ret_regs.r11 = reg_assocs[11].value.reg64;
            ret_regs.r12 = reg_assocs[12].value.reg64;
            ret_regs.r13 = reg_assocs[13].value.reg64;
            ret_regs.r14 = reg_assocs[14].value.reg64;
            ret_regs.r15 = reg_assocs[15].value.reg64;
            ret_regs.rip = reg_assocs[16].value.reg64;
            ret_regs.rflags = reg_assocs[17].value.reg64;
        }

        Ok(ret_regs)
    }
    /// Returns the vCPU general purpose registers on ARM64.
    #[cfg(target_arch = "aarch64")]
    pub fn get_regs(&self) -> Result<StandardRegisters> {
        let mut reg_assocs: Vec<hv_register_assoc> = Vec::with_capacity(38);
        for i in 0..29 as usize {
            reg_assocs.push(hv_register_assoc {
                name: hv_register_name_HV_ARM64_REGISTER_X0 + i as u32,
                ..Default::default()
            });
        }
        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_FP,
            ..Default::default()
        });
        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_LR,
            ..Default::default()
        });
        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_SP,
            ..Default::default()
        });
        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_PC,
            ..Default::default()
        });
        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_PSTATE,
            ..Default::default()
        });
        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_SP_EL1,
            ..Default::default()
        });
        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_ELR_EL1,
            ..Default::default()
        });
        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_FPSR,
            ..Default::default()
        });
        reg_assocs.push(hv_register_assoc {
            name: hv_register_name_HV_ARM64_REGISTER_FPCR,
            ..Default::default()
        });

        self.hvcall_get_reg(&mut reg_assocs)?;
        let mut ret_regs = StandardRegisters::default();
        // SAFETY: access union fields
        unsafe {
            for i in 0..31 as usize {
                ret_regs.regs[i] = reg_assocs[i].value.reg64;
            }

            ret_regs.sp = reg_assocs[31].value.reg64;
            ret_regs.pc = reg_assocs[32].value.reg64;
            ret_regs.pstate = reg_assocs[33].value.reg64;
            ret_regs.sp_el1 = reg_assocs[34].value.reg64;
            ret_regs.elr_el1 = reg_assocs[35].value.reg64;
            ret_regs.fpsr = reg_assocs[36].value.reg64;
            ret_regs.fpcr = reg_assocs[37].value.reg64;
        }
        Ok(ret_regs)
    }

    /// Returns the vCPU general purpose registers using VP register page
    #[cfg(not(target_arch = "aarch64"))]
    pub fn get_standard_regs_vp_page(&self) -> Result<StandardRegisters> {
        let vp_reg_page = self.get_vp_reg_page().unwrap().0;
        let mut ret_regs = StandardRegisters::default();
        // SAFETY: access union fields
        unsafe {
            ret_regs.rax = get_gp_regs_field_ptr!(vp_reg_page, rax);
            ret_regs.rbx = get_gp_regs_field_ptr!(vp_reg_page, rbx);
            ret_regs.rcx = get_gp_regs_field_ptr!(vp_reg_page, rcx);
            ret_regs.rdx = get_gp_regs_field_ptr!(vp_reg_page, rdx);
            ret_regs.rsi = get_gp_regs_field_ptr!(vp_reg_page, rsi);
            ret_regs.rdi = get_gp_regs_field_ptr!(vp_reg_page, rdi);
            ret_regs.rsp = get_gp_regs_field_ptr!(vp_reg_page, rsp);
            ret_regs.rbp = get_gp_regs_field_ptr!(vp_reg_page, rbp);
            ret_regs.r8 = get_gp_regs_field_ptr!(vp_reg_page, r8);
            ret_regs.r9 = get_gp_regs_field_ptr!(vp_reg_page, r9);
            ret_regs.r10 = get_gp_regs_field_ptr!(vp_reg_page, r10);
            ret_regs.r11 = get_gp_regs_field_ptr!(vp_reg_page, r11);
            ret_regs.r12 = get_gp_regs_field_ptr!(vp_reg_page, r12);
            ret_regs.r13 = get_gp_regs_field_ptr!(vp_reg_page, r13);
            ret_regs.r14 = get_gp_regs_field_ptr!(vp_reg_page, r14);
            ret_regs.r15 = get_gp_regs_field_ptr!(vp_reg_page, r15);
            ret_regs.rip = (*vp_reg_page).__bindgen_anon_1.__bindgen_anon_1.rip;
            ret_regs.rflags = (*vp_reg_page).__bindgen_anon_1.__bindgen_anon_1.rflags;
        }

        Ok(ret_regs)
    }

    /// Returns the vCPU special registers using VP register page
    #[cfg(not(target_arch = "aarch64"))]
    fn get_special_regs_vp_page(&self) -> Result<SpecialRegisters> {
        let vp_reg_page = match self.get_vp_reg_page() {
            Some(page) => page.0,
            None => return Err(libc::EINVAL.into()),
        };
        let mut ret_regs = SpecialRegisters::default();
        // SAFETY: access union fields
        unsafe {
            ret_regs.cr0 = (*vp_reg_page).cr0;
            ret_regs.cr3 = (*vp_reg_page).cr3;
            ret_regs.cr4 = (*vp_reg_page).cr4;
            ret_regs.cr8 = (*vp_reg_page).cr8;
            ret_regs.cs = (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.cs.into();
            ret_regs.ds = (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.ds.into();
            ret_regs.es = (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.es.into();
            ret_regs.fs = (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.fs.into();
            ret_regs.gs = (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.gs.into();
            ret_regs.ss = (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.ss.into();
            ret_regs.efer = (*vp_reg_page).efer;
        }

        let mut reg_assocs: [hv_register_assoc; 7] = [hv_register_assoc::default(); 7];
        for (it, elem) in reg_assocs.iter_mut().zip(NON_VP_PAGE_SP_REGS) {
            it.name = *elem;
        }
        self.get_reg(&mut reg_assocs)?;
        // SAFETY: access union fields
        unsafe {
            ret_regs.tr = SegmentRegister::from(reg_assocs[0].value.segment);
            ret_regs.ldt = SegmentRegister::from(reg_assocs[1].value.segment);
            ret_regs.gdt = TableRegister::from(reg_assocs[2].value.table);
            ret_regs.idt = TableRegister::from(reg_assocs[3].value.table);
            ret_regs.cr2 = reg_assocs[4].value.reg64;
            ret_regs.apic_base = reg_assocs[5].value.reg64;
            // A VMM can use the bitmap to reassert a pending external interrupt
            // for a live migrating guest.
            update_interrupt_bitmap(
                &mut ret_regs,
                reg_assocs[6].value.pending_interruption.as_uint64,
            );
        }
        Ok(ret_regs)
    }

    /// Returns the vCPU special registers using IOCTL
    #[cfg(not(target_arch = "aarch64"))]
    fn get_special_regs_ioctl(&self) -> Result<SpecialRegisters> {
        let mut reg_names: [::std::os::raw::c_uint; 18] = [0u32; 18];
        reg_names[..11].copy_from_slice(VP_PAGE_SP_REGS);
        reg_names[11..].copy_from_slice(NON_VP_PAGE_SP_REGS);
        let mut reg_assocs: [hv_register_assoc; 18] = [hv_register_assoc::default(); 18];
        for (it, elem) in reg_assocs.iter_mut().zip(reg_names) {
            it.name = elem;
        }

        self.get_reg(&mut reg_assocs)?;
        let mut ret_regs = SpecialRegisters::default();
        /*
            ** Sequence of the register names;

            0: hv_register_name_HV_X64_REGISTER_CS,
            1: hv_register_name_HV_X64_REGISTER_DS,
            2: hv_register_name_HV_X64_REGISTER_ES,
            3: hv_register_name_HV_X64_REGISTER_FS,
            4: hv_register_name_HV_X64_REGISTER_GS,
            5: hv_register_name_HV_X64_REGISTER_SS,
            6: hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR0,
            7: hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR3,
            8: hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR4,
            9: hv_register_name_HV_X64_REGISTER_CR8,
            10: hv_register_name_HV_X64_REGISTER_EFER,
            11: hv_register_name_HV_X64_REGISTER_TR,
            12: hv_register_name_HV_X64_REGISTER_LDTR,
            13: hv_register_name_HV_X64_REGISTER_GDTR,
            14: hv_register_name_HV_X64_REGISTER_IDTR,
            15: hv_register_name_HV_X64_REGISTER_CR2,
            16: hv_register_name_HV_X64_REGISTER_APIC_BASE,
            17: hv_register_name_HV_REGISTER_PENDING_INTERRUPTION,
        */
        // SAFETY: access union fields
        unsafe {
            ret_regs.cs = SegmentRegister::from(reg_assocs[0].value.segment);
            ret_regs.ds = SegmentRegister::from(reg_assocs[1].value.segment);
            ret_regs.es = SegmentRegister::from(reg_assocs[2].value.segment);
            ret_regs.fs = SegmentRegister::from(reg_assocs[3].value.segment);
            ret_regs.gs = SegmentRegister::from(reg_assocs[4].value.segment);
            ret_regs.ss = SegmentRegister::from(reg_assocs[5].value.segment);
            ret_regs.tr = SegmentRegister::from(reg_assocs[11].value.segment);
            ret_regs.ldt = SegmentRegister::from(reg_assocs[12].value.segment);
            ret_regs.gdt = TableRegister::from(reg_assocs[13].value.table);
            ret_regs.idt = TableRegister::from(reg_assocs[14].value.table);
            ret_regs.cr0 = reg_assocs[6].value.reg64;
            ret_regs.cr2 = reg_assocs[15].value.reg64;
            ret_regs.cr3 = reg_assocs[7].value.reg64;
            ret_regs.cr4 = reg_assocs[8].value.reg64;
            ret_regs.cr8 = reg_assocs[9].value.reg64;
            ret_regs.efer = reg_assocs[10].value.reg64;
            ret_regs.apic_base = reg_assocs[16].value.reg64;
            // A VMM can use the bitmap to reassert a pending external interrupt
            // for a live migrating guest.
            update_interrupt_bitmap(
                &mut ret_regs,
                reg_assocs[17].value.pending_interruption.as_uint64,
            );
        };

        Ok(ret_regs)
    }

    /// Returns the vCPU special registers.
    #[cfg(not(target_arch = "aarch64"))]
    pub fn get_sregs(&self) -> Result<SpecialRegisters> {
        if self.is_valid_vp_reg_page() {
            self.get_special_regs_vp_page()
        } else {
            self.get_special_regs_ioctl()
        }
    }

    /// Sets the vCPU special registers using VP register page
    #[cfg(not(target_arch = "aarch64"))]
    fn set_special_regs_vp_page(&self, sregs: &SpecialRegisters) -> Result<()> {
        let vp_reg_page = self.get_vp_reg_page().unwrap().0;
        unsafe {
            (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.cs = sregs.cs.into();
            (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.ds = sregs.ds.into();
            (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.es = sregs.es.into();
            (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.fs = sregs.fs.into();
            (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.gs = sregs.gs.into();
            (*vp_reg_page).__bindgen_anon_3.__bindgen_anon_1.ss = sregs.ss.into();
            // Update dirty bits
            (*vp_reg_page).dirty |= 1 << HV_X64_REGISTER_CLASS_SEGMENT;
        }
        let reg_assocs = [
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_TR,
                value: hv_register_value {
                    segment: sregs.tr.into(),
                },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_LDTR,
                value: hv_register_value {
                    segment: sregs.ldt.into(),
                },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_GDTR,
                value: hv_register_value {
                    table: sregs.gdt.into(),
                },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_IDTR,
                value: hv_register_value {
                    table: sregs.idt.into(),
                },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR0,
                value: hv_register_value { reg64: sregs.cr0 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_CR2,
                value: hv_register_value { reg64: sregs.cr2 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR3,
                value: hv_register_value { reg64: sregs.cr3 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR4,
                value: hv_register_value { reg64: sregs.cr4 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_CR8,
                value: hv_register_value { reg64: sregs.cr8 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_EFER,
                value: hv_register_value { reg64: sregs.efer },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_APIC_BASE,
                value: hv_register_value {
                    reg64: sregs.apic_base,
                },
                ..Default::default()
            },
        ];

        self.set_reg(&reg_assocs)?;
        Ok(())
    }

    /// Sets the vCPU special registers using IOCTL
    #[cfg(not(target_arch = "aarch64"))]
    fn set_special_regs_ioctl(&self, sregs: &SpecialRegisters) -> Result<()> {
        let reg_names: [hv_register_name; 17] = [
            hv_register_name_HV_X64_REGISTER_CS,
            hv_register_name_HV_X64_REGISTER_DS,
            hv_register_name_HV_X64_REGISTER_ES,
            hv_register_name_HV_X64_REGISTER_FS,
            hv_register_name_HV_X64_REGISTER_GS,
            hv_register_name_HV_X64_REGISTER_SS,
            hv_register_name_HV_X64_REGISTER_TR,
            hv_register_name_HV_X64_REGISTER_LDTR,
            hv_register_name_HV_X64_REGISTER_GDTR,
            hv_register_name_HV_X64_REGISTER_IDTR,
            hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR0,
            hv_register_name_HV_X64_REGISTER_CR2,
            hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR3,
            hv_register_name_HV_X64_REGISTER_INTERMEDIATE_CR4,
            hv_register_name_HV_X64_REGISTER_CR8,
            hv_register_name_HV_X64_REGISTER_EFER,
            hv_register_name_HV_X64_REGISTER_APIC_BASE,
        ];
        let reg_values: [hv_register_value; 17] = [
            hv_register_value {
                segment: sregs.cs.into(),
            },
            hv_register_value {
                segment: sregs.ds.into(),
            },
            hv_register_value {
                segment: sregs.es.into(),
            },
            hv_register_value {
                segment: sregs.fs.into(),
            },
            hv_register_value {
                segment: sregs.gs.into(),
            },
            hv_register_value {
                segment: sregs.ss.into(),
            },
            hv_register_value {
                segment: sregs.tr.into(),
            },
            hv_register_value {
                segment: sregs.ldt.into(),
            },
            hv_register_value {
                table: sregs.gdt.into(),
            },
            hv_register_value {
                table: sregs.idt.into(),
            },
            hv_register_value { reg64: sregs.cr0 },
            hv_register_value { reg64: sregs.cr2 },
            hv_register_value { reg64: sregs.cr3 },
            hv_register_value { reg64: sregs.cr4 },
            hv_register_value { reg64: sregs.cr8 },
            hv_register_value { reg64: sregs.efer },
            hv_register_value {
                reg64: sregs.apic_base,
            },
        ];

        let reg_assocs: Vec<hv_register_assoc> = reg_names
            .iter()
            .zip(reg_values.iter())
            .map(|t| hv_register_assoc {
                name: *t.0,
                value: *t.1,
                ..Default::default()
            })
            .collect();
        self.set_reg(&reg_assocs)?;
        Ok(())
    }

    /// Public API to set the vCPU special registers
    #[cfg(not(target_arch = "aarch64"))]
    pub fn set_sregs(&self, sregs: &SpecialRegisters) -> Result<()> {
        if self.is_valid_vp_reg_page() {
            self.set_special_regs_vp_page(sregs)
        } else {
            self.set_special_regs_ioctl(sregs)
        }
    }

    #[cfg(not(target_arch = "aarch64"))]
    fn fpu_registers() -> [hv_register_name; 26] {
        [
            hv_register_name_HV_X64_REGISTER_XMM0,
            hv_register_name_HV_X64_REGISTER_XMM1,
            hv_register_name_HV_X64_REGISTER_XMM2,
            hv_register_name_HV_X64_REGISTER_XMM3,
            hv_register_name_HV_X64_REGISTER_XMM4,
            hv_register_name_HV_X64_REGISTER_XMM5,
            hv_register_name_HV_X64_REGISTER_XMM6,
            hv_register_name_HV_X64_REGISTER_XMM7,
            hv_register_name_HV_X64_REGISTER_XMM8,
            hv_register_name_HV_X64_REGISTER_XMM9,
            hv_register_name_HV_X64_REGISTER_XMM10,
            hv_register_name_HV_X64_REGISTER_XMM11,
            hv_register_name_HV_X64_REGISTER_XMM12,
            hv_register_name_HV_X64_REGISTER_XMM13,
            hv_register_name_HV_X64_REGISTER_XMM14,
            hv_register_name_HV_X64_REGISTER_XMM15,
            hv_register_name_HV_X64_REGISTER_FP_MMX0,
            hv_register_name_HV_X64_REGISTER_FP_MMX1,
            hv_register_name_HV_X64_REGISTER_FP_MMX2,
            hv_register_name_HV_X64_REGISTER_FP_MMX3,
            hv_register_name_HV_X64_REGISTER_FP_MMX4,
            hv_register_name_HV_X64_REGISTER_FP_MMX5,
            hv_register_name_HV_X64_REGISTER_FP_MMX6,
            hv_register_name_HV_X64_REGISTER_FP_MMX7,
            hv_register_name_HV_X64_REGISTER_FP_CONTROL_STATUS,
            hv_register_name_HV_X64_REGISTER_XMM_CONTROL_STATUS,
        ]
    }

    /// Sets the vCPU floating point registers
    #[cfg(not(target_arch = "aarch64"))]
    pub fn set_fpu(&self, fpu: &FloatingPointUnit) -> Result<()> {
        let reg_names = Self::fpu_registers();
        let mut reg_values: [hv_register_value; 26] = [hv_register_value { reg64: 0 }; 26];
        // First 16 registers are XMM registers.
        for (i, reg) in reg_values.iter_mut().enumerate().take(16) {
            // SAFETY: we're sure the underlying bit pattern is valid
            unsafe {
                *reg = hv_register_value {
                    reg128: std::mem::transmute::<[u8; 16usize], hv_u128>(fpu.xmm[i]),
                };
            }
        }
        // The next 8 registers are FP registers.
        for (i, reg) in reg_values.iter_mut().enumerate().take(24).skip(16) {
            let fp_i = i - 16;
            // SAFETY: we're sure the underlying bit pattern is valid
            unsafe {
                *reg = hv_register_value {
                    fp: hv_x64_fp_register {
                        as_uint128: std::mem::transmute::<[u8; 16usize], hv_u128>(fpu.fpr[fp_i]),
                    },
                };
            }
        }
        reg_values[24] = hv_register_value {
            fp_control_status: hv_x64_fp_control_status_register {
                __bindgen_anon_1: hv_x64_fp_control_status_register__bindgen_ty_1 {
                    fp_control: fpu.fcw,
                    fp_status: fpu.fsw,
                    fp_tag: fpu.ftwx,
                    reserved: 0x0,
                    last_fp_op: fpu.last_opcode,
                    __bindgen_anon_1:
                        hv_x64_fp_control_status_register__bindgen_ty_1__bindgen_ty_1 {
                            last_fp_rip: fpu.last_ip,
                        },
                },
            },
        };
        reg_values[25] = hv_register_value {
            xmm_control_status: hv_x64_xmm_control_status_register {
                __bindgen_anon_1: hv_x64_xmm_control_status_register__bindgen_ty_1 {
                    xmm_status_control: fpu.mxcsr,
                    xmm_status_control_mask: 0x0,
                    __bindgen_anon_1:
                        hv_x64_xmm_control_status_register__bindgen_ty_1__bindgen_ty_1 {
                            last_fp_rdp: fpu.last_dp,
                        },
                },
            },
        };

        let reg_assocs: Vec<hv_register_assoc> = reg_names
            .iter()
            .zip(reg_values.iter())
            .map(|t| hv_register_assoc {
                name: *t.0,
                value: *t.1,
                ..Default::default()
            })
            .collect();

        self.set_reg(&reg_assocs)?;
        Ok(())
    }
    /// Returns the floating point state (FPU) from the vCPU.
    #[cfg(not(target_arch = "aarch64"))]
    pub fn get_fpu(&self) -> Result<FloatingPointUnit> {
        let reg_names = Self::fpu_registers();
        let mut reg_assocs: Vec<hv_register_assoc> = reg_names
            .iter()
            .map(|name| hv_register_assoc {
                name: *name,
                ..Default::default()
            })
            .collect();
        self.get_reg(&mut reg_assocs)?;

        // SAFETY: access union fields
        let fp_control_status: hv_x64_fp_control_status_register__bindgen_ty_1 =
            unsafe { reg_assocs[24].value.fp_control_status.__bindgen_anon_1 };
        let xmm_control_status: hv_x64_xmm_control_status_register__bindgen_ty_1 =
            unsafe { reg_assocs[25].value.xmm_control_status.__bindgen_anon_1 };
        let mut ret_regs = unsafe {
            FloatingPointUnit {
                fpr: [[0x0; 16usize]; 8usize],
                fcw: fp_control_status.fp_control,
                fsw: fp_control_status.fp_status,
                ftwx: fp_control_status.fp_tag,
                pad1: 0x0,
                last_opcode: fp_control_status.last_fp_op,
                last_ip: fp_control_status.__bindgen_anon_1.last_fp_rip,
                last_dp: xmm_control_status.__bindgen_anon_1.last_fp_rdp,
                xmm: [[0; 16usize]; 16usize],
                mxcsr: xmm_control_status.xmm_status_control,
                pad2: 0x0,
            }
        };

        // First 16 registers are XMM registers.
        for (i, reg) in reg_assocs.iter().enumerate().take(16) {
            // SAFETY: we trust the hypervisor returns the expected data type.
            unsafe {
                ret_regs.xmm[i] = std::mem::transmute::<hv_u128, [u8; 16usize]>(reg.value.reg128);
            }
        }
        // The next 8 registers are FP registers.
        for (i, reg) in reg_assocs.iter().enumerate().take(24).skip(16) {
            let fp_i = i - 16;
            // SAFETY: we trust the hypervisor returns the expected data type.
            unsafe {
                ret_regs.fpr[fp_i] =
                    std::mem::transmute::<hv_u128, [u8; 16usize]>(reg.value.fp.as_uint128);
            }
        }

        Ok(ret_regs)
    }
    /// X86 specific call that returns the vcpu's current "debug registers".
    #[cfg(not(target_arch = "aarch64"))]
    pub fn get_debug_regs(&self) -> Result<DebugRegisters> {
        let reg_names: [hv_register_name; 6] = [
            hv_register_name_HV_X64_REGISTER_DR0,
            hv_register_name_HV_X64_REGISTER_DR1,
            hv_register_name_HV_X64_REGISTER_DR2,
            hv_register_name_HV_X64_REGISTER_DR3,
            hv_register_name_HV_X64_REGISTER_DR6,
            hv_register_name_HV_X64_REGISTER_DR7,
        ];

        let mut reg_assocs: Vec<hv_register_assoc> = reg_names
            .iter()
            .map(|name| hv_register_assoc {
                name: *name,
                ..Default::default()
            })
            .collect();

        self.get_reg(&mut reg_assocs)?;

        let ret_regs = unsafe {
            DebugRegisters {
                dr0: reg_assocs[0].value.reg64,
                dr1: reg_assocs[1].value.reg64,
                dr2: reg_assocs[2].value.reg64,
                dr3: reg_assocs[3].value.reg64,
                dr6: reg_assocs[4].value.reg64,
                dr7: reg_assocs[5].value.reg64,
            }
        };

        Ok(ret_regs)
    }
    /// X86 specific call that sets the vcpu's current "debug registers".
    #[cfg(not(target_arch = "aarch64"))]
    pub fn set_debug_regs(&self, d_regs: &DebugRegisters) -> Result<()> {
        let reg_names = [
            hv_register_name_HV_X64_REGISTER_DR0,
            hv_register_name_HV_X64_REGISTER_DR1,
            hv_register_name_HV_X64_REGISTER_DR2,
            hv_register_name_HV_X64_REGISTER_DR3,
            hv_register_name_HV_X64_REGISTER_DR6,
            hv_register_name_HV_X64_REGISTER_DR7,
        ];
        let reg_values = [
            hv_register_value { reg64: d_regs.dr0 },
            hv_register_value { reg64: d_regs.dr1 },
            hv_register_value { reg64: d_regs.dr2 },
            hv_register_value { reg64: d_regs.dr3 },
            hv_register_value { reg64: d_regs.dr6 },
            hv_register_value { reg64: d_regs.dr7 },
        ];

        let reg_assocs: Vec<hv_register_assoc> = reg_names
            .iter()
            .zip(reg_values.iter())
            .map(|t| hv_register_assoc {
                name: *t.0,
                value: *t.1,
                ..Default::default()
            })
            .collect();

        self.set_reg(&reg_assocs)?;
        Ok(())
    }
    /// Returns the machine-specific registers (MSR) for this vCPU.
    #[cfg(not(target_arch = "aarch64"))]
    pub fn get_msrs(&self, msrs: &mut Msrs) -> Result<usize> {
        let nmsrs = msrs.as_fam_struct_ref().nmsrs as usize;
        let mut reg_assocs: Vec<hv_register_assoc> = Vec::with_capacity(nmsrs);

        for i in 0..nmsrs {
            let name = match msr_to_hv_reg_name(msrs.as_slice()[i].index) {
                Ok(n) => n,
                Err(_) => return Err(libc::EINVAL.into()),
            };
            reg_assocs.push(hv_register_assoc {
                name,
                ..Default::default()
            });
        }

        self.get_reg(&mut reg_assocs)?;

        for (i, reg) in reg_assocs.iter().enumerate().take(nmsrs) {
            // SAFETY: access union fields requires unsafe. The values are initialized by get_reg
            // call.
            unsafe {
                msrs.as_mut_slice()[i].data = reg.value.reg64;
            }
        }

        Ok(nmsrs)
    }
    /// Setup the model-specific registers (MSR) for this vCPU.
    /// Returns the number of MSR entries actually written.
    #[cfg(not(target_arch = "aarch64"))]
    pub fn set_msrs(&self, msrs: &Msrs) -> Result<usize> {
        let nmsrs = msrs.as_fam_struct_ref().nmsrs as usize;
        let mut reg_assocs: Vec<hv_register_assoc> = Vec::with_capacity(nmsrs);

        for i in 0..nmsrs {
            let name = match msr_to_hv_reg_name(msrs.as_slice()[i].index) {
                Ok(n) => n,
                Err(_) => return Err(libc::EINVAL.into()),
            };
            reg_assocs.push(hv_register_assoc {
                name,
                value: hv_register_value {
                    reg64: msrs.as_slice()[i].data,
                },
                ..Default::default()
            });
        }

        self.set_reg(&reg_assocs)?;
        Ok(0_usize)
    }
    ///  Triggers the running of the current virtual CPU returning an exit reason.
    pub fn run(&self) -> Result<hv_message> {
        let mut msg = hv_message::default();
        // SAFETY: we know that our file is a vCPU fd and we verify the return result.
        let ret = unsafe { ioctl_with_mut_ref(self, MSHV_RUN_VP(), &mut msg) };
        if ret != 0 {
            return Err(errno::Error::last().into());
        }
        Ok(msg)
    }
    /// Returns currently pending exceptions, interrupts, and NMIs as well as related
    /// states of the vcpu.
    #[cfg(not(target_arch = "aarch64"))]
    pub fn get_vcpu_events(&self) -> Result<VcpuEvents> {
        let reg_names: [hv_register_name; 5] = [
            hv_register_name_HV_REGISTER_PENDING_INTERRUPTION,
            hv_register_name_HV_REGISTER_INTERRUPT_STATE,
            hv_register_name_HV_REGISTER_INTERNAL_ACTIVITY_STATE,
            hv_register_name_HV_REGISTER_PENDING_EVENT0,
            hv_register_name_HV_REGISTER_PENDING_EVENT1,
        ];
        let mut reg_assocs: Vec<hv_register_assoc> = reg_names
            .iter()
            .map(|name| hv_register_assoc {
                name: *name,
                ..Default::default()
            })
            .collect();
        self.get_reg(&mut reg_assocs)?;
        let mut ret_regs = VcpuEvents::default();
        // SAFETY: access union fields
        unsafe {
            ret_regs.pending_interruption = reg_assocs[0].value.reg64;
            ret_regs.interrupt_state = reg_assocs[1].value.reg64;
            ret_regs.internal_activity_state = reg_assocs[2].value.reg64;
            ret_regs.pending_event0 =
                std::mem::transmute::<hv_u128, [u8; 16usize]>(reg_assocs[3].value.reg128);
            ret_regs.pending_event1 =
                std::mem::transmute::<hv_u128, [u8; 16usize]>(reg_assocs[4].value.reg128);
        }
        Ok(ret_regs)
    }
    /// Sets pending exceptions, interrupts, and NMIs as well as related states of the vcpu.
    #[cfg(not(target_arch = "aarch64"))]
    pub fn set_vcpu_events(&self, events: &VcpuEvents) -> Result<()> {
        let reg_names: [hv_register_name; 5] = [
            hv_register_name_HV_REGISTER_PENDING_INTERRUPTION,
            hv_register_name_HV_REGISTER_INTERRUPT_STATE,
            hv_register_name_HV_REGISTER_INTERNAL_ACTIVITY_STATE,
            hv_register_name_HV_REGISTER_PENDING_EVENT0,
            hv_register_name_HV_REGISTER_PENDING_EVENT1,
        ];
        // SAFETY: access union fields requires unsafe. For transmuting values we're sure
        // the types and bit patterns are correct.
        let reg_values: [hv_register_value; 5] = unsafe {
            [
                hv_register_value {
                    reg64: events.pending_interruption,
                },
                hv_register_value {
                    reg64: events.interrupt_state,
                },
                hv_register_value {
                    reg64: events.internal_activity_state,
                },
                hv_register_value {
                    reg128: std::mem::transmute::<[u8; 16usize], hv_u128>(events.pending_event0),
                },
                hv_register_value {
                    reg128: std::mem::transmute::<[u8; 16usize], hv_u128>(events.pending_event1),
                },
            ]
        };

        let reg_assocs: Vec<hv_register_assoc> = reg_names
            .iter()
            .zip(reg_values.iter())
            .map(|t| hv_register_assoc {
                name: *t.0,
                value: *t.1,
                ..Default::default()
            })
            .collect();
        self.set_reg(&reg_assocs)?;
        Ok(())
    }

    /// X86 specific call that returns the vcpu's current "xcrs" using VP register page
    /// This is a private API for internal use, not publicly available
    #[cfg(not(target_arch = "aarch64"))]
    fn get_xcrs_vp_page(&self) -> Result<Xcrs> {
        let vp_reg_page = self.get_vp_reg_page().unwrap().0;
        // SAFETY: access union fields
        let ret_regs = unsafe {
            Xcrs {
                xcr0: (*vp_reg_page).xfem,
            }
        };

        Ok(ret_regs)
    }

    /// X86 specific call that returns the vcpu's current "xcrs" using IOCTL
    #[cfg(not(target_arch = "aarch64"))]
    fn get_xcrs_ioctl(&self) -> Result<Xcrs> {
        let mut reg_assocs: [hv_register_assoc; 1] = [hv_register_assoc {
            name: hv_register_name_HV_X64_REGISTER_XFEM,
            ..Default::default()
        }];
        self.get_reg(&mut reg_assocs)?;

        // SAFETY: access union fields
        let ret_regs = unsafe {
            Xcrs {
                xcr0: reg_assocs[0].value.reg64,
            }
        };

        Ok(ret_regs)
    }

    /// X86 specific call that returns the vcpu's current "xcrs".
    #[cfg(not(target_arch = "aarch64"))]
    pub fn get_xcrs(&self) -> Result<Xcrs> {
        if self.is_valid_vp_reg_page() {
            self.get_xcrs_vp_page()
        } else {
            self.get_xcrs_ioctl()
        }
    }

    /// X86 specific call to set XCRs
    #[cfg(not(target_arch = "aarch64"))]
    pub fn set_xcrs(&self, xcrs: &Xcrs) -> Result<()> {
        self.set_reg(&[hv_register_assoc {
            name: hv_register_name_HV_X64_REGISTER_XFEM,
            value: hv_register_value { reg64: xcrs.xcr0 },
            ..Default::default()
        }])
    }
    /// X86 specific call that returns the vcpu's current "misc registers".
    #[cfg(not(target_arch = "aarch64"))]
    pub fn get_misc_regs(&self) -> Result<MiscRegs> {
        let mut reg_assocs: [hv_register_assoc; 1] = [hv_register_assoc {
            name: hv_register_name_HV_X64_REGISTER_HYPERCALL,
            ..Default::default()
        }];
        self.get_reg(&mut reg_assocs)?;

        // SAFETY: access union fields
        let mut ret_regs = unsafe {
            MiscRegs {
                hypercall: reg_assocs[0].value.reg64,
                ..Default::default()
            }
        };
        if let Some(vp_page) = self.get_vp_reg_page() {
            let vp_reg_page = vp_page.0;
            // SAFETY: access union fields
            unsafe {
                ret_regs.int_vec = (*vp_reg_page).interrupt_vectors.as_uint64;
            }
        }
        Ok(ret_regs)
    }
    /// X86 specific call that sets the vcpu's current "misc registers".
    #[cfg(not(target_arch = "aarch64"))]
    pub fn set_misc_regs(&self, misc: &MiscRegs) -> Result<()> {
        if let Some(vp_page) = self.get_vp_reg_page() {
            let vp_reg_page = vp_page.0;
            // SAFETY: access union fields
            unsafe {
                (*vp_reg_page).interrupt_vectors.as_uint64 = misc.int_vec;
            }
        }

        self.set_reg(&[hv_register_assoc {
            name: hv_register_name_HV_X64_REGISTER_HYPERCALL,
            value: hv_register_value {
                reg64: misc.hypercall,
            },
            ..Default::default()
        }])
    }
    #[cfg(target_arch = "x86_64")]
    /// Returns the VCpu state. This IOCTLs can be used to get XSave and LAPIC state.
    pub fn get_vp_state_ioctl(&self, state: &mut mshv_get_set_vp_state) -> Result<()> {
        // SAFETY: we know that our file is a vCPU fd and we verify the return result.
        let ret = unsafe { ioctl_with_mut_ref(self, MSHV_GET_VP_STATE(), state) };
        if ret != 0 {
            return Err(errno::Error::last().into());
        }
        Ok(())
    }
    #[cfg(target_arch = "x86_64")]
    /// Set vp states (LAPIC, XSave etc)
    /// Test code already covered by get/set_lapic/xsave
    pub fn set_vp_state_ioctl(&self, state: &mshv_get_set_vp_state) -> Result<()> {
        // SAFETY: IOCTL call with correct types
        let ret = unsafe { ioctl_with_ref(self, MSHV_SET_VP_STATE(), state) };
        if ret != 0 {
            return Err(errno::Error::last().into());
        }
        Ok(())
    }
    #[cfg(target_arch = "x86_64")]
    /// Get the state of the LAPIC (Local Advanced Programmable Interrupt Controller).
    pub fn get_lapic(&self) -> Result<LapicState> {
        let buffer = Buffer::new(HV_PAGE_SIZE, HV_PAGE_SIZE)?;
        let mut vp_state = mshv_get_set_vp_state {
            buf_ptr: buffer.buf as u64,
            buf_sz: buffer.size() as u32,
            type_: MSHV_VP_STATE_LAPIC as u8,
            ..Default::default()
        };
        self.get_vp_state_ioctl(&mut vp_state)?;
        Ok(LapicState::try_from(buffer)?)
    }
    #[cfg(target_arch = "x86_64")]
    /// Sets the state of the LAPIC (Local Advanced Programmable Interrupt Controller).
    pub fn set_lapic(&self, lapic_state: &LapicState) -> Result<()> {
        let buffer = Buffer::try_from(lapic_state)?;
        let vp_state = mshv_get_set_vp_state {
            type_: MSHV_VP_STATE_LAPIC as u8,
            buf_sz: buffer.size() as u32,
            buf_ptr: buffer.buf as u64,
            ..Default::default()
        };
        self.set_vp_state_ioctl(&vp_state)
    }
    #[cfg(target_arch = "x86_64")]
    /// Returns the xsave data
    pub fn get_xsave(&self) -> Result<XSave> {
        let buffer = Buffer::new(HV_PAGE_SIZE, HV_PAGE_SIZE)?;
        let mut vp_state = mshv_get_set_vp_state {
            buf_ptr: buffer.buf as u64,
            buf_sz: buffer.size() as u32,
            type_: MSHV_VP_STATE_XSAVE as u8,
            ..Default::default()
        };
        self.get_vp_state_ioctl(&mut vp_state)?;
        Ok(XSave::try_from(buffer)?)
    }
    #[cfg(target_arch = "x86_64")]
    /// Set the xsave data
    pub fn set_xsave(&self, data: &XSave) -> Result<()> {
        let buffer = Buffer::try_from(data)?;
        let vp_state = mshv_get_set_vp_state {
            type_: MSHV_VP_STATE_XSAVE as u8,
            buf_sz: buffer.size() as u32,
            buf_ptr: buffer.buf as u64,
            ..Default::default()
        };
        self.set_vp_state_ioctl(&vp_state)
    }
    /// Translate guest virtual address to guest physical address.
    ///
    /// Tries the dedicated `MSHV_VP_TRANSLATE_GVA` ioctl first (which handles
    /// transient GPA access failures by faulting pages back in). Falls back
    /// to the generic hypercall passthrough if the kernel does not support
    /// the new ioctl (`ENOTTY`).
    pub fn translate_gva(&self, gva: u64, flags: u64) -> Result<(u64, hv_translate_gva_result)> {
        match self.ioctl_translate_gva(gva, flags) {
            Err(MshvError::Errno(e)) if e.errno() == libc::ENOTTY => {
                self.hvcall_translate_gva(gva, flags)
            }
            other => other,
        }
    }
    /// Dedicated ioctl for GVA translation with transparent fault-in
    /// handling for movable memory regions.
    fn ioctl_translate_gva(&self, gva: u64, flags: u64) -> Result<(u64, hv_translate_gva_result)> {
        let mut result = hv_translate_gva_result::default();
        let mut gpa: u64 = 0;
        let mut args = mshv_translate_gva {
            gva,
            flags,
            result: &mut result,
            gpa: &mut gpa,
        };
        // SAFETY: IOCTL with correct types, result and gpa pointers are valid
        let ret = unsafe { ioctl_with_mut_ref(self, MSHV_VP_TRANSLATE_GVA(), &mut args) };
        if ret != 0 {
            return Err(errno::Error::last().into());
        }
        Ok((gpa, result))
    }
    /// Generic hvcall version of translate guest virtual address
    fn hvcall_translate_gva(&self, gva: u64, flags: u64) -> Result<(u64, hv_translate_gva_result)> {
        let input = hv_input_translate_virtual_address {
            vp_index: self.index,
            control_flags: flags,
            gva_page: gva >> HV_HYP_PAGE_SHIFT,
            ..Default::default() // NOTE: Kernel will populate partition_id field
        };
        let mut output = hv_output_translate_virtual_address {
            ..Default::default()
        };
        let mut args = make_args!(HVCALL_TRANSLATE_VIRTUAL_ADDRESS, input, output);
        self.hvcall(&mut args)?;

        let gpa = (output.gpa_page << HV_HYP_PAGE_SHIFT) | (gva & !(HV_HYP_PAGE_MASK as u64));

        Ok((gpa, output.translation_result))
    }

    /// X86 specific call that returns the vcpu's current "suspend registers".
    #[cfg(not(target_arch = "aarch64"))]
    pub fn get_suspend_regs(&self) -> Result<SuspendRegisters> {
        let reg_names: [hv_register_name; 2] = [
            hv_register_name_HV_REGISTER_EXPLICIT_SUSPEND,
            hv_register_name_HV_REGISTER_INTERCEPT_SUSPEND,
        ];

        let mut reg_assocs: Vec<hv_register_assoc> = reg_names
            .iter()
            .map(|name| hv_register_assoc {
                name: *name,
                ..Default::default()
            })
            .collect();

        self.get_reg(&mut reg_assocs)?;

        // SAFETY: access union fields
        let ret_regs = unsafe {
            SuspendRegisters {
                explicit_register: reg_assocs[0].value.reg64,
                intercept_register: reg_assocs[1].value.reg64,
            }
        };

        Ok(ret_regs)
    }
    #[cfg(target_arch = "x86_64")]
    /// Register override CPUID values for one leaf.
    pub fn register_intercept_result_cpuid_entry(
        &self,
        entry: &hv_cpuid_entry,
        always_override: Option<u8>,
        subleaf_specific: Option<u8>,
    ) -> Result<()> {
        self.hvcall_register_intercept_result_cpuid_entry(entry, always_override, subleaf_specific)
    }

    #[cfg(target_arch = "x86_64")]
    /// Register override CPUID values for one leaf.
    fn hvcall_register_intercept_result_cpuid_entry(
        &self,
        entry: &hv_cpuid_entry,
        always_override: Option<u8>,
        subleaf_specific: Option<u8>,
    ) -> Result<()> {
        let mshv_cpuid = hv_register_x64_cpuid_result_parameters {
            input: hv_register_x64_cpuid_result_parameters__bindgen_ty_1 {
                eax: entry.function,
                // Subleaf index, default is 0. Further subleafs can be
                // overwritten by a repeated call to this function with a desired
                // index passed. Refer to the Intel Dev Manual for a particular
                // EAX input for the further details.
                ecx: entry.index,
                // Whether the intercept result is to be applied to all
                // the subleafs (0) or just to the specific subleaf (1).
                subleaf_specific: subleaf_specific.unwrap_or(0),
                // Override even if the hypervisor computed value is zero.
                // If set to 1, the registered result will be still applied.
                always_override: always_override.unwrap_or(1),
                // Not relevant, bindgen specific struct padding.
                padding: 0,
            },
            // With regard to masks - these are to specify bits to be overwritten.
            // The current CpuidEntry structure wouldn't allow to carry the masks
            // in addition to the actual register values. For this reason, the
            // masks are set to the exact values of the corresponding register bits
            // to be registered for an overwrite. To view resulting values the
            // hypervisor would return, HvCallGetVpCpuidValues hypercall can be used.
            result: hv_register_x64_cpuid_result_parameters__bindgen_ty_2 {
                eax: entry.eax,
                eax_mask: entry.eax,
                ebx: entry.ebx,
                ebx_mask: entry.ebx,
                ecx: entry.ecx,
                ecx_mask: entry.ecx,
                edx: entry.edx,
                edx_mask: entry.edx,
            },
        };
        let input = hv_input_register_intercept_result {
            vp_index: self.index,
            intercept_type: hv_intercept_type_HV_INTERCEPT_TYPE_X64_CPUID,
            parameters: hv_register_intercept_result_parameters { cpuid: mshv_cpuid },
            ..Default::default() // NOTE: Kernel will populate partition_id field
        };
        let mut args = make_args!(HVCALL_REGISTER_INTERCEPT_RESULT, input);
        self.hvcall(&mut args)?;

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    /// Extend CPUID values delivered by hypervisor.
    pub fn register_intercept_result_cpuid(&self, cpuid: &CpuId) -> Result<()> {
        let mut ret = Ok(());

        for entry in cpuid.as_slice().iter() {
            let mut override_arg = None;
            let mut subleaf_specific = None;

            match entry.function {
                // Intel
                // 0xb - Extended Topology Enumeration Leaf
                // 0x1f - V2 Extended Topology Enumeration Leaf
                // AMD
                // 0x8000_001e - Processor Topology Information
                // 0x8000_0026 - Extended CPU Topology
                0xb | 0x1f | 0x8000_001e | 0x8000_0026 => {
                    subleaf_specific = Some(1);
                    override_arg = None;
                }
                0x0000_0001 | 0x8000_0000 | 0x8000_0001 | 0x8000_0008 => {
                    subleaf_specific = None;
                    override_arg = Some(1);
                }
                _ => {}
            }
            let eret =
                self.register_intercept_result_cpuid_entry(entry, override_arg, subleaf_specific);
            if eret.is_err() && ret.is_ok() {
                ret = eret;
            }
        }

        ret
    }
    /// X86 specific call that retrieves the values of the specified CPUID
    /// leaf as observed on the virtual processor.
    #[cfg(not(target_arch = "aarch64"))]
    pub fn get_cpuid_values(&self, eax: u32, ecx: u32, xfem: u64, xss: u64) -> Result<[u32; 4]> {
        self.hvcall_get_cpuid_values(eax, ecx, xfem, xss)
    }
    /// Generic hvcall version of get cpuid values
    #[cfg(not(target_arch = "aarch64"))]
    fn hvcall_get_cpuid_values(&self, eax: u32, ecx: u32, xfem: u64, xss: u64) -> Result<[u32; 4]> {
        let mut input = make_rep_input!(
            hv_input_get_vp_cpuid_values {
                vp_index: self.index,
                ..Default::default() // NOTE: Kernel will populate partition_id field
            },
            cpuid_leaf_info,
            [hv_cpuid_leaf_info {
                eax,
                ecx,
                xfem,
                xss,
            }]
        );
        unsafe {
            input
                .as_mut_struct_ref()
                .flags
                .__bindgen_anon_1
                .set_use_vp_xfem_xss(1);
            input
                .as_mut_struct_ref()
                .flags
                .__bindgen_anon_1
                .set_apply_registered_values(1);
        }
        let mut output_arr: [hv_output_get_vp_cpuid_values; 1] = [Default::default()];
        let mut args = make_rep_args!(HVCALL_GET_VP_CPUID_VALUES, input, output_arr);
        self.hvcall(&mut args)?;

        // SAFETY: The hvcall succeeded, and both fields of the union are
        // equivalent. Just return the array instead of taking eax, ebx, etc...
        Ok(unsafe { output_arr[0].as_uint32 })
    }
    /// Read GPA
    pub fn gpa_read(&self, input: &mut mshv_read_write_gpa) -> Result<mshv_read_write_gpa> {
        let flags = hv_access_gpa_control_flags {
            as_uint64: input.flags as u64,
        };
        let res = self.hvcall_gpa_read(input.byte_count, input.base_gpa, flags)?;
        input.data = res.data;
        Ok(*input)
    }

    /// Generic hvcall version of gpa_read
    fn hvcall_gpa_read(
        &self,
        byte_count: u32,
        gpa: u64,
        flags: hv_access_gpa_control_flags,
    ) -> Result<hv_output_read_gpa> {
        let input = hv_input_read_gpa {
            vp_index: self.index,
            byte_count,
            base_gpa: gpa,
            control_flags: flags,
            ..Default::default() // NOTE: Kernel will populate partition_id field
        };
        let mut output = hv_output_read_gpa::default();
        let mut args = make_args!(HVCALL_READ_GPA, input, output);
        self.hvcall(&mut args)?;

        Ok(output)
    }

    /// Write GPA
    pub fn gpa_write(&self, input: &mut mshv_read_write_gpa) -> Result<mshv_read_write_gpa> {
        let flags = hv_access_gpa_control_flags {
            as_uint64: input.flags as u64,
        };
        // The old ioctl just drops the access result on the floor, so we do the same.
        self.hvcall_gpa_write(input.byte_count, input.base_gpa, flags, input.data)?;
        Ok(*input)
    }
    /// Generic hvcall version of gpa_write
    fn hvcall_gpa_write(
        &self,
        byte_count: u32,
        gpa: u64,
        flags: hv_access_gpa_control_flags,
        data: [__u8; 16usize],
    ) -> Result<hv_output_write_gpa> {
        let input = hv_input_write_gpa {
            vp_index: self.index,
            byte_count,
            base_gpa: gpa,
            control_flags: flags,
            data,
            ..Default::default() // NOTE: Kernel will populate partition_id field
        };
        let mut output = hv_output_write_gpa::default();
        let mut args = make_args!(HVCALL_WRITE_GPA, input, output);
        self.hvcall(&mut args)?;

        Ok(output)
    }

    /// Sets the sev control register
    #[cfg(not(target_arch = "aarch64"))]
    pub fn set_sev_control_register(&self, reg: u64) -> Result<()> {
        let reg_assocs = [hv_register_assoc {
            name: hv_register_name_HV_X64_REGISTER_SEV_CONTROL,
            value: hv_register_value { reg64: reg },
            ..Default::default()
        }];

        self.set_reg(&reg_assocs)?;
        Ok(())
    }

    /// Gets the VP state components
    #[cfg(not(target_arch = "aarch64"))]
    pub fn get_all_vp_state_components(&self) -> Result<AllVpStateComponents> {
        let mut states: AllVpStateComponents = AllVpStateComponents::default();
        let mut buffer = Buffer::new(HV_PAGE_SIZE, HV_PAGE_SIZE)?;

        for i in 0..MSHV_VP_STATE_COUNT {
            buffer.zero_out_buf();
            let mut vp_state = mshv_get_set_vp_state {
                buf_ptr: buffer.buf as u64,
                buf_sz: buffer.size() as u32,
                type_: i as u8,
                ..Default::default()
            };
            self.get_vp_state_ioctl(&mut vp_state)?;
            states.copy_to_or_from_buffer(i as usize, &mut buffer, false);
        }
        Ok(states)
    }

    /// Sets the VP state components
    #[cfg(not(target_arch = "aarch64"))]
    pub fn set_all_vp_state_components(&self, states: &mut AllVpStateComponents) -> Result<()> {
        let mut buffer = Buffer::new(HV_PAGE_SIZE, HV_PAGE_SIZE)?;

        for i in 0..MSHV_VP_STATE_COUNT {
            buffer.zero_out_buf();
            states.copy_to_or_from_buffer(i as usize, &mut buffer, true);
            let vp_state = mshv_get_set_vp_state {
                type_: i as u8,
                buf_sz: buffer.size() as u32,
                buf_ptr: buffer.buf as u64,
                ..Default::default()
            };
            self.set_vp_state_ioctl(&vp_state)?;
        }
        Ok(())
    }

    /// Execute a hypercall for this vp
    pub fn hvcall(&self, args: &mut mshv_root_hvcall) -> Result<()> {
        // SAFETY: IOCTL with correct types
        let ret = unsafe { ioctl_with_mut_ref(self, MSHV_ROOT_HVCALL(), args) };
        if ret == 0 {
            Ok(())
        } else {
            Err(MshvError::from_hvcall(errno::Error::last(), *args))
        }
    }

    /// Get the list of system/synthetic registers supported by MSHV.
    #[cfg(target_arch = "aarch64")]
    pub fn get_reg_list(&self) -> Result<MshvRegList> {
        let mut reg_list = MshvRegList::default();
        reg_list.reg_list = vec![
            hv_register_name_HV_ARM64_REGISTER_CNTVCT_EL0,
            hv_register_name_HV_ARM64_REGISTER_PAR_EL1,
            hv_register_name_HV_ARM64_REGISTER_SPSR_EL1,
            hv_register_name_HV_ARM64_REGISTER_MPIDR_EL1,
            hv_register_name_HV_ARM64_REGISTER_MIDR_EL1,
            hv_register_name_HV_ARM64_REGISTER_SCTLR_EL1,
            hv_register_name_HV_ARM64_REGISTER_ACTLR_EL1,
            hv_register_name_HV_ARM64_REGISTER_TCR_EL1,
            hv_register_name_HV_ARM64_REGISTER_MAIR_EL1,
            hv_register_name_HV_ARM64_REGISTER_TPIDR_EL1,
            hv_register_name_HV_ARM64_REGISTER_AMAIR_EL1,
            hv_register_name_HV_ARM64_REGISTER_TPIDRRO_EL0,
            hv_register_name_HV_ARM64_REGISTER_TPIDR_EL0,
            hv_register_name_HV_ARM64_REGISTER_CONTEXTIDR_EL1,
            hv_register_name_HV_ARM64_REGISTER_CPACR_EL1,
            hv_register_name_HV_ARM64_REGISTER_CSSELR_EL1,
            hv_register_name_HV_ARM64_REGISTER_CNTKCTL_EL1,
            hv_register_name_HV_ARM64_REGISTER_CNTV_CTL_EL0,
            hv_register_name_HV_ARM64_REGISTER_CNTV_CVAL_EL0,
            hv_register_name_HV_ARM64_REGISTER_TTBR0_EL1,
            hv_register_name_HV_ARM64_REGISTER_TTBR1_EL1,
            hv_register_name_HV_ARM64_REGISTER_VBAR_EL1,
            hv_register_name_HV_ARM64_REGISTER_ESR_EL1,
            hv_register_name_HV_ARM64_REGISTER_FAR_EL1,
            hv_register_name_HV_ARM64_REGISTER_PAR_EL1,
            hv_register_name_HV_ARM64_REGISTER_SP_EL0,
            hv_register_name_HV_ARM64_REGISTER_SP_EL1,
            hv_register_name_HV_ARM64_REGISTER_AFSR0_EL1,
            hv_register_name_HV_ARM64_REGISTER_AFSR1_EL1,
            hv_register_name_HV_ARM64_REGISTER_SYNTHETIC_VBAR_EL1,
            hv_register_name_HV_REGISTER_PENDING_EVENT0,
            hv_register_name_HV_REGISTER_PENDING_EVENT1,
            hv_register_name_HV_REGISTER_DELIVERABILITY_NOTIFICATIONS,
            hv_register_name_HV_REGISTER_INTERNAL_ACTIVITY_STATE,
        ];
        Ok(reg_list)
    }
}

impl Drop for VcpuFd {
    fn drop(&mut self) {
        if let Some(vp_page) = &self.vp_page {
            // SAFETY: because we ensured vp_page is valid in create_vcpu.
            unsafe {
                let _ = libc::munmap(vp_page.0 as *mut libc::c_void, HV_PAGE_SIZE);
            }
        }
    }
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use crate::ioctls::system::Mshv;

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_get_regs() {
        let set_reg_assocs: [hv_register_assoc; 2] = [
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RIP,
                value: hv_register_value { reg64: 0x1000 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RFLAGS,
                value: hv_register_value { reg64: 0x2 },
                ..Default::default()
            },
        ];
        let get_reg_assocs: [hv_register_assoc; 2] = [
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RIP,
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RFLAGS,
                ..Default::default()
            },
        ];

        for i in [0, 1] {
            let hv = Mshv::new().unwrap();
            let vm = hv.create_vm().unwrap();
            vm.initialize().unwrap();
            let vcpu = vm.create_vcpu(0).unwrap();

            if i == 0 {
                vcpu.set_reg(&set_reg_assocs).unwrap();
            } else {
                vcpu.hvcall_set_reg(&set_reg_assocs).unwrap();
            }

            let mut get_regs: [hv_register_assoc; 2] = get_reg_assocs;

            if i == 0 {
                vcpu.get_reg(&mut get_regs).unwrap();
            } else {
                vcpu.hvcall_get_reg(&mut get_regs).unwrap();
            }

            // SAFETY: access union fields
            unsafe {
                assert!(get_regs[0].value.reg64 == 0x1000);
                assert!(get_regs[1].value.reg64 == 0x2);
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn test_set_get_regs() {
        let set_regs_assocs = [
            hv_register_assoc {
                name: hv_register_name_HV_ARM64_REGISTER_PC,
                value: hv_register_value { reg64: 0x1000 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_ARM64_REGISTER_ELR_EL1,
                value: hv_register_value { reg64: 0x2 },
                ..Default::default()
            },
        ];

        let get_reg_assocs = [
            hv_register_assoc {
                name: hv_register_name_HV_ARM64_REGISTER_PC,
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_ARM64_REGISTER_ELR_EL1,
                ..Default::default()
            },
        ];

        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();

        vcpu.hvcall_set_reg(&set_regs_assocs).unwrap();

        let mut get_regs = get_reg_assocs;
        vcpu.hvcall_get_reg(&mut get_regs).unwrap();

        // SAFETY: access union fields
        unsafe {
            assert!(get_regs[0].value.reg64 == 0x1000);
            assert!(get_regs[1].value.reg64 == 0x2);
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_get_sregs() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();
        let s_sregs = vcpu.get_sregs().unwrap();
        vcpu.set_sregs(&s_sregs).unwrap();
        let g_sregs = vcpu.get_sregs().unwrap();
        assert!(g_sregs.cr0 == s_sregs.cr0);
        assert!(g_sregs.cr2 == s_sregs.cr2);
        assert!(g_sregs.cr3 == s_sregs.cr3);
        assert!(g_sregs.cr4 == s_sregs.cr4);
        assert!(g_sregs.cr8 == s_sregs.cr8);
        assert!(g_sregs.cr8 == s_sregs.cr8);
        assert!(g_sregs.apic_base == s_sregs.apic_base);
        assert!(g_sregs.efer == s_sregs.efer);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_sregs_interrupt_bitmap() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();
        let mut sregs = vcpu.get_sregs().unwrap();
        sregs.interrupt_bitmap[0] = 1;
        vcpu.set_sregs(&sregs).unwrap();
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_get_standard_registers() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();

        let s_regs = vcpu.get_regs().unwrap();
        vcpu.set_regs(&s_regs).unwrap();
        let g_regs = vcpu.get_regs().unwrap();
        assert!(g_regs.rax == s_regs.rax);
        assert!(g_regs.rbx == s_regs.rbx);
        assert!(g_regs.rcx == s_regs.rcx);
        assert!(g_regs.rdx == s_regs.rdx);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_get_debug_registers() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();

        let s_regs = vcpu.get_debug_regs().unwrap();
        vcpu.set_debug_regs(&s_regs).unwrap();
        let g_regs = vcpu.get_debug_regs().unwrap();
        assert!(g_regs.dr0 == s_regs.dr0);
        assert!(g_regs.dr1 == s_regs.dr1);
        assert!(g_regs.dr2 == s_regs.dr2);
        assert!(g_regs.dr3 == s_regs.dr3);
        assert!(g_regs.dr6 == s_regs.dr6);
        assert!(g_regs.dr7 == s_regs.dr7);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_get_fpu() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();

        let s_regs = vcpu.get_fpu().unwrap();
        vcpu.set_fpu(&s_regs).unwrap();
        let g_regs = vcpu.get_fpu().unwrap();
        for i in 0..16 {
            for j in 0..16 {
                assert!(g_regs.xmm[i][j] == s_regs.xmm[i][j]);
            }
        }
        for i in 0..8 {
            for j in 0..16 {
                assert!(g_regs.fpr[i][j] == s_regs.fpr[i][j]);
            }
        }
        assert!(g_regs.fcw == s_regs.fcw);
        assert!(g_regs.fsw == s_regs.fsw);
        assert!(g_regs.ftwx == s_regs.ftwx);
        assert!(g_regs.last_opcode == s_regs.last_opcode);
        assert!(g_regs.last_ip == s_regs.last_ip);
        assert!(g_regs.last_dp == s_regs.last_dp);
        assert!(g_regs.mxcsr == s_regs.mxcsr);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_run_code() {
        use libc::c_void;

        use super::*;
        use crate::ioctls::system::Mshv;
        use crate::set_bits;
        use std::io::Write;

        let mshv = Mshv::new().unwrap();
        let vm = mshv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();
        // This example is based on https://lwn.net/Articles/658511/
        #[rustfmt::skip]
        let code:[u8;11] = [
            0xba, 0xf8, 0x03,  /* mov $0x3f8, %dx */
            0x00, 0xd8,         /* add %bl, %al */
            0x04, b'0',         /* add $'0', %al */
            0xee,               /* out %al, (%dx) */
            /* send a 0 to indicate we're done */
            0xb0, b'\0',        /* mov $'\0', %al */
            0xee,               /* out %al, (%dx) */
        ];

        let mem_size = 0x4000;
        let load_addr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                mem_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_SHARED | libc::MAP_NORESERVE,
                -1,
                0,
            )
        } as *mut u8;
        let mem_region = mshv_user_mem_region {
            flags: set_bits!(u8, MSHV_SET_MEM_BIT_WRITABLE, MSHV_SET_MEM_BIT_EXECUTABLE),
            guest_pfn: 0x1,
            size: 0x1000,
            userspace_addr: load_addr as u64,
            ..Default::default()
        };

        vm.map_user_memory(mem_region).unwrap();

        unsafe {
            // Get a mutable slice of `mem_size` from `load_addr`.
            // This is safe because we mapped it before.
            let mut slice = slice::from_raw_parts_mut(load_addr, mem_size);
            slice.write_all(&code).unwrap();
        }

        //Get CS Register
        let mut cs_reg = hv_register_assoc {
            name: hv_register_name_HV_X64_REGISTER_CS,
            ..Default::default()
        };
        vcpu.get_reg(slice::from_mut(&mut cs_reg)).unwrap();

        unsafe {
            assert_ne!({ cs_reg.value.segment.base }, 0);
            assert_ne!({ cs_reg.value.segment.selector }, 0);
        };

        cs_reg.value.segment.base = 0;
        cs_reg.value.segment.selector = 0;

        vcpu.set_reg(&[
            cs_reg,
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RAX,
                value: hv_register_value { reg64: 2 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RBX,
                value: hv_register_value { reg64: 2 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RIP,
                value: hv_register_value { reg64: 0x1000 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RFLAGS,
                value: hv_register_value { reg64: 0x2 },
                ..Default::default()
            },
        ])
        .unwrap();

        let mut done = false;
        loop {
            let ret_hv_message = vcpu.run().unwrap();
            match ret_hv_message.header.message_type {
                hv_message_type_HVMSG_X64_HALT => {
                    println!("VM Halted!");
                    break;
                }
                hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT => {
                    let io_message = ret_hv_message.to_ioport_info().unwrap();

                    if !done {
                        assert!(io_message.rax == b'4' as u64);
                        assert!(io_message.port_number == 0x3f8);
                        unsafe {
                            assert!(io_message.access_info.__bindgen_anon_1.string_op() == 0);
                            assert!(io_message.access_info.__bindgen_anon_1.access_size() == 1);
                        }
                        assert!(
                            io_message.header.intercept_access_type == /*HV_INTERCEPT_ACCESS_WRITE*/ 1_u8
                        );
                        done = true;
                        /* Advance rip */
                        vcpu.set_reg(&[hv_register_assoc {
                            name: hv_register_name_HV_X64_REGISTER_RIP,
                            value: hv_register_value {
                                reg64: io_message.header.rip + 1,
                            },
                            ..Default::default()
                        }])
                        .unwrap();
                    } else {
                        assert!(io_message.rax == b'\0' as u64);
                        assert!(io_message.port_number == 0x3f8);
                        unsafe {
                            assert!(io_message.access_info.__bindgen_anon_1.string_op() == 0);
                            assert!(io_message.access_info.__bindgen_anon_1.access_size() == 1);
                        }
                        assert!(
                            io_message.header.intercept_access_type == /*HV_INTERCEPT_ACCESS_WRITE*/ 1_u8
                        );
                        break;
                    }
                }
                _ => {
                    println!("Message type: 0x{:x?}", {
                        ret_hv_message.header.message_type
                    });
                    panic!("Unexpected Exit Type");
                }
            };
        }
        assert!(done);
        vm.unmap_user_memory(mem_region).unwrap();
        unsafe { libc::munmap(load_addr as *mut c_void, mem_size) };
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_run_code_mmap() {
        use super::*;
        use crate::ioctls::system::Mshv;
        use crate::set_bits;
        use libc::c_void;
        use std::io::Write;

        let mshv = Mshv::new().unwrap();
        let vm = mshv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();
        // This example is based on https://lwn.net/Articles/658511/
        #[rustfmt::skip]
        let code:[u8;11] = [
            0xba, 0xf8, 0x03,  /* mov $0x3f8, %dx */
            0x00, 0xd8,         /* add %bl, %al */
            0x04, b'0',         /* add $'0', %al */
            0xee,               /* out %al, (%dx) */
            /* send a 0 to indicate we're done */
            0xb0, b'\0',        /* mov $'\0', %al */
            0xee,               /* out %al, (%dx) */
        ];

        let mem_size = 0x4000;
        let load_addr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                mem_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_SHARED | libc::MAP_NORESERVE,
                -1,
                0,
            )
        } as *mut u8;
        let mem_region = mshv_user_mem_region {
            flags: set_bits!(u8, MSHV_SET_MEM_BIT_WRITABLE, MSHV_SET_MEM_BIT_EXECUTABLE),
            guest_pfn: 0x1,
            size: 0x1000,
            userspace_addr: load_addr as u64,
            ..Default::default()
        };

        let registers_addr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                0x1000,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                vcpu.as_raw_fd(),
                MSHV_VP_MMAP_OFFSET_REGISTERS as i64 * libc::sysconf(libc::_SC_PAGE_SIZE),
            )
        } as *mut u8;

        if registers_addr as *mut c_void == libc::MAP_FAILED {
            panic!(
                "Could not mmap register page, error:{}",
                std::io::Error::last_os_error()
            );
        }

        let hv_msg_addr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                0x1000,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                vcpu.as_raw_fd(),
                MSHV_VP_MMAP_OFFSET_INTERCEPT_MESSAGE as i64 * libc::sysconf(libc::_SC_PAGE_SIZE),
            )
        } as *mut u8;

        if hv_msg_addr as *mut c_void == libc::MAP_FAILED {
            panic!(
                "Could not mmap HV page, error:{}",
                std::io::Error::last_os_error()
            );
        }

        vm.map_user_memory(mem_region).unwrap();

        let reg_page: *mut hv_vp_register_page = registers_addr as *mut hv_vp_register_page;
        let hv_msg_page: *mut hv_message = hv_msg_addr as *mut hv_message;
        let mut done = false;

        unsafe {
            // Get a mutable slice of `mem_size` from `load_addr`.
            // This is safe because we mapped it before.
            let mut slice = slice::from_raw_parts_mut(load_addr, mem_size);
            slice.write_all(&code).unwrap();
        }

        // Get CS Register
        let mut cs_reg = hv_register_assoc {
            name: hv_register_name_HV_X64_REGISTER_CS,
            ..Default::default()
        };
        vcpu.get_reg(slice::from_mut(&mut cs_reg)).unwrap();

        unsafe {
            assert_ne!({ cs_reg.value.segment.base }, 0);
            assert_ne!({ cs_reg.value.segment.selector }, 0);
        };

        cs_reg.value.segment.base = 0;
        cs_reg.value.segment.selector = 0;

        vcpu.set_reg(&[
            cs_reg,
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RAX,
                value: hv_register_value { reg64: 2 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RBX,
                value: hv_register_value { reg64: 2 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RIP,
                value: hv_register_value { reg64: 0x1000 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RFLAGS,
                value: hv_register_value { reg64: 0x2 },
                ..Default::default()
            },
        ])
        .unwrap();

        unsafe {
            assert!((*reg_page).version == HV_VP_REGISTER_PAGE_VERSION_1 as u16);
            assert!((*reg_page).isvalid == 1);
            assert!((*reg_page).dirty == 0);
        }

        loop {
            vcpu.run().unwrap();
            let msg_header = unsafe { (*hv_msg_page).header };
            match msg_header.message_type {
                hv_message_type_HVMSG_X64_HALT => {
                    println!("VM Halted!");
                    break;
                }
                hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT => {
                    let io_message = unsafe { (*hv_msg_page).to_ioport_info().unwrap() };
                    assert!(io_message.port_number == 0x3f8);
                    unsafe {
                        assert!(io_message.access_info.__bindgen_anon_1.string_op() == 0);
                        assert!(io_message.access_info.__bindgen_anon_1.access_size() == 1);
                    }
                    if !done {
                        assert!(io_message.rax == b'4' as u64);
                        assert!(io_message.port_number == 0x3f8);
                        unsafe {
                            assert!(io_message.access_info.__bindgen_anon_1.string_op() == 0);
                            assert!(io_message.access_info.__bindgen_anon_1.access_size() == 1);
                        }
                        assert!(
                            io_message.header.intercept_access_type == /*HV_INTERCEPT_ACCESS_WRITE*/ 1_u8
                        );
                        done = true;
                        /* Advance rip */
                        unsafe {
                            (*reg_page).__bindgen_anon_1.__bindgen_anon_1.rip =
                                io_message.header.rip + 1;
                        }
                        unsafe {
                            (*reg_page).dirty = 1 << HV_X64_REGISTER_CLASS_IP;
                        }
                    } else {
                        assert!(io_message.rax == b'\0' as u64);
                        assert!(
                            io_message.header.intercept_access_type == /*HV_INTERCEPT_ACCESS_WRITE*/ 1_u8
                        );
                        break;
                    }
                }
                _ => {
                    println!("Message type: 0x{:x?}", { msg_header.message_type });
                    panic!("Unexpected Exit Type");
                }
            };
        }
        assert!(done);

        vm.unmap_user_memory(mem_region).unwrap();
        unsafe { libc::munmap(load_addr as *mut c_void, mem_size) };
        unsafe { libc::munmap(registers_addr as *mut c_void, 0x1000) };
        unsafe { libc::munmap(hv_msg_addr as *mut c_void, 0x1000) };
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_get_msrs() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();

        let s_regs = Msrs::from_entries(&[
            msr_entry {
                index: IA32_MSR_SYSENTER_CS,
                data: 0x1,
                ..Default::default()
            },
            msr_entry {
                index: IA32_MSR_SYSENTER_ESP,
                data: 0x2,
                ..Default::default()
            },
        ])
        .unwrap();
        let mut g_regs = Msrs::from_entries(&[
            msr_entry {
                index: IA32_MSR_SYSENTER_CS,
                ..Default::default()
            },
            msr_entry {
                index: IA32_MSR_SYSENTER_ESP,
                ..Default::default()
            },
        ])
        .unwrap();
        vcpu.set_msrs(&s_regs).unwrap();
        vcpu.get_msrs(&mut g_regs).unwrap();
        assert!(g_regs.as_fam_struct_ref().nmsrs == s_regs.as_fam_struct_ref().nmsrs);
        assert!(g_regs.as_slice()[0].data == s_regs.as_slice()[0].data);
        assert!(g_regs.as_slice()[1].data == s_regs.as_slice()[1].data);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_get_vcpu_events() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();

        let s_regs = vcpu.get_vcpu_events().unwrap();
        vcpu.set_vcpu_events(&s_regs).unwrap();
        let g_regs = vcpu.get_vcpu_events().unwrap();
        assert!(g_regs.pending_interruption == s_regs.pending_interruption);
        assert!(g_regs.interrupt_state == s_regs.interrupt_state);
        assert!(g_regs.internal_activity_state == s_regs.internal_activity_state);
        for i in 0..16 {
            assert!(g_regs.pending_event0[i] == s_regs.pending_event0[i]);
            assert!(g_regs.pending_event1[i] == s_regs.pending_event1[i]);
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_get_xcrs() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();

        let s_regs = vcpu.get_xcrs().unwrap();
        vcpu.set_xcrs(&s_regs).unwrap();
        let g_regs = vcpu.get_xcrs().unwrap();
        assert!(g_regs.xcr0 == s_regs.xcr0);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_get_lapic() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();

        let state = vcpu.get_lapic().unwrap();
        vcpu.set_lapic(&state).unwrap();
        let g_state = vcpu.get_lapic().unwrap();
        for i in 0..1024 {
            assert!(state.regs[i] == g_state.regs[i]);
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_registers_64() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();
        let arr_reg_name_value = [
            (hv_register_name_HV_X64_REGISTER_RIP, 0x1000),
            (hv_register_name_HV_X64_REGISTER_RFLAGS, 0x2),
        ];
        set_registers_64!(vcpu, &arr_reg_name_value).unwrap();
        let mut get_regs: [hv_register_assoc; 2] = [
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RIP,
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_RFLAGS,
                ..Default::default()
            },
        ];

        vcpu.get_reg(&mut get_regs).unwrap();

        unsafe {
            /* use returned regs */
            assert!(get_regs[0].value.reg64 == 0x1000);
            assert!(get_regs[1].value.reg64 == 0x2);
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_get_set_xsave() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();

        let state = vcpu.get_xsave().unwrap();

        vcpu.set_xsave(&state).unwrap();
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_get_suspend_regs() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();

        let regs = vcpu.get_suspend_regs().unwrap();
        // Verify the returned values
        assert!(regs.explicit_register == 0x1);
        assert!(regs.intercept_register == 0x0);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_set_get_misc_regs() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();

        let s_regs = vcpu.get_misc_regs().unwrap();
        vcpu.set_misc_regs(&s_regs).unwrap();
        let g_regs = vcpu.get_misc_regs().unwrap();
        assert!(g_regs.hypercall == s_regs.hypercall);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_get_cpuid_values() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();
        let res_0 = vcpu.get_cpuid_values(0, 0, 0, 0).unwrap();
        let max_function = res_0[0];
        assert!(max_function >= 1);
        let res_1 = vcpu.hvcall_get_cpuid_values(0, 0, 0, 0).unwrap();
        assert!(res_1[0] >= 1);
        assert!(res_0[0] == res_1[0]);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_get_set_vp_state_components() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm().unwrap();
        vm.initialize().unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();
        let mut states = vcpu.get_all_vp_state_components().unwrap();
        vcpu.set_all_vp_state_components(&mut states).unwrap();
        let ret_states = vcpu.get_all_vp_state_components().unwrap();
        assert!(states
            .buffer
            .iter()
            .zip(ret_states.buffer)
            .all(|(a, b)| *a == b));
    }
}
