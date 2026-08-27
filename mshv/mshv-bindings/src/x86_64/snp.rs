// Copyright © 2023, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//
// Definitions are derived from AMD GHCB spec
// https://www.amd.com/system/files/TechDocs/56421-guest-hypervisor-communication-block-standardization.pdf
// https://www.amd.com/system/files/TechDocs/24593.pdf
//
#![allow(
    clippy::too_many_arguments,
    clippy::missing_safety_doc,
    clippy::useless_transmute,
    clippy::unnecessary_cast,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]
use crate::bindings::*;
use vmm_sys_util::errno;

type Result<T> = std::result::Result<T, errno::Error>;

pub const GHCB_PROTOCOL_VERSION_MIN: u32 = 1;
pub const GHCB_PROTOCOL_VERSION_MAX: u32 = 2;

pub const SVM_EXITCODE_HV_DOORBELL_PAGE: u32 = 2147483668;
pub const SVM_EXITCODE_SNP_GUEST_REQUEST: u32 = 2147483665;
pub const SVM_EXITCODE_SNP_EXTENDED_GUEST_REQUEST: u32 = 2147483666;
pub const SVM_EXITCODE_SNP_AP_CREATION: u32 = 2147483667;
pub const SVM_EXITCODE_IOIO_PROT: u32 = 123;
pub const SVM_EXITCODE_MMIO_READ: u32 = 2147483649;
pub const SVM_EXITCODE_MMIO_WRITE: u32 = 2147483650;
pub const SVM_NAE_HV_DOORBELL_PAGE_GET_PREFERRED: u32 = 0;
pub const SVM_NAE_HV_DOORBELL_PAGE_SET: u32 = 1;
pub const SVM_NAE_HV_DOORBELL_PAGE_QUERY: u32 = 2;
pub const SVM_NAE_HV_DOORBELL_PAGE_CLEAR: u32 = 3;

pub const GHCB_INFO_BIT_WIDTH: u32 = 12;
pub const GHCB_INFO_MASK: u32 = 4095;
pub const GHCB_DATA_MASK: u64 = 4503599627370495;
pub const GHCB_INFO_NORMAL: u32 = 0;
pub const GHCB_INFO_SEV_INFO_RESPONSE: u32 = 1;
pub const GHCB_INFO_SEV_INFO_REQUEST: u32 = 2;
pub const GHCB_INFO_AP_JUMP_TABLE: u32 = 3;
pub const GHCB_INFO_CPUID_REQUEST: u32 = 4;
pub const GHCB_INFO_CPUID_RESPONSE: u32 = 5;
pub const GHCB_INFO_PREFERRED_REQUEST: u32 = 16;
pub const GHCB_INFO_PREFERRED_RESPONSE: u32 = 17;
pub const GHCB_INFO_REGISTER_REQUEST: u32 = 18;
pub const GHCB_INFO_REGISTER_RESPONSE: u32 = 19;
pub const GHCB_INFO_PAGE_STATE_CHANGE: u32 = 20;
pub const GHCB_INFO_PAGE_STATE_UPDATED: u32 = 21;
pub const GHCB_INFO_HYP_FEATURE_REQUEST: u32 = 128;
pub const GHCB_INFO_HYP_FEATURE_RESPONSE: u32 = 129;
pub const GHCB_INFO_SPECIAL_HYPERCALL: u32 = 3840;
pub const GHCB_INFO_SPECIAL_FAST_CALL: u32 = 3841;
pub const GHCB_INFO_HYPERCALL_OUTPUT: u32 = 3842;
pub const GHCB_INFO_SPECIAL_DBGPRINT: u32 = 3843;
pub const GHCB_INFO_SHUTDOWN_REQUEST: u32 = 256;

pub const GHCB_HYP_FEATURE_SEV_SNP: u32 = 1 << 0;
pub const GHCB_HYP_FEATURE_SEV_SNP_AP_CREATION: u32 = 1 << 1;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub union svm_ghcb_msr {
    pub as_uint64: __u64,
    pub as_uint16: [__u16; 4usize],
    pub __bindgen_anon_1: svm_ghcb_msr__bindgen_ty_1,
    pub __bindgen_anon_2: svm_ghcb_msr__bindgen_ty_2,
    pub __bindgen_anon_3: svm_ghcb_msr__bindgen_ty_3,
    pub __bindgen_anon_4: svm_ghcb_msr__bindgen_ty_4,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct svm_ghcb_msr__bindgen_ty_1 {
    pub _bitfield_align_1: [u8; 0],
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 8usize]>,
}
#[test]
fn bindgen_test_layout_svm_ghcb_msr__bindgen_ty_1() {
    assert_eq!(
        ::std::mem::size_of::<svm_ghcb_msr__bindgen_ty_1>(),
        8usize,
        concat!("Size of: ", stringify!(svm_ghcb_msr__bindgen_ty_1))
    );
    assert_eq!(
        ::std::mem::align_of::<svm_ghcb_msr__bindgen_ty_1>(),
        1usize,
        concat!("Alignment of ", stringify!(svm_ghcb_msr__bindgen_ty_1))
    );
}
impl svm_ghcb_msr__bindgen_ty_1 {
    #[inline]
    pub fn ghcb_low(&self) -> __u64 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(0usize, 32u8) as u64) }
    }
    #[inline]
    pub fn set_ghcb_low(&mut self, val: __u64) {
        unsafe {
            let val: u64 = ::std::mem::transmute(val);
            self._bitfield_1.set(0usize, 32u8, val as u64)
        }
    }
    #[inline]
    pub fn ghcb_high(&self) -> __u64 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(32usize, 32u8) as u64) }
    }
    #[inline]
    pub fn set_ghcb_high(&mut self, val: __u64) {
        unsafe {
            let val: u64 = ::std::mem::transmute(val);
            self._bitfield_1.set(32usize, 32u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(
        ghcb_low: __u64,
        ghcb_high: __u64,
    ) -> __BindgenBitfieldUnit<[u8; 8usize]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 8usize]> = Default::default();
        __bindgen_bitfield_unit.set(0usize, 32u8, {
            let ghcb_low: u64 = unsafe { ::std::mem::transmute(ghcb_low) };
            ghcb_low as u64
        });
        __bindgen_bitfield_unit.set(32usize, 32u8, {
            let ghcb_high: u64 = unsafe { ::std::mem::transmute(ghcb_high) };
            ghcb_high as u64
        });
        __bindgen_bitfield_unit
    }
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct svm_ghcb_msr__bindgen_ty_2 {
    pub _bitfield_align_1: [u8; 0],
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 8usize]>,
}
#[test]
fn bindgen_test_layout_svm_ghcb_msr__bindgen_ty_2() {
    assert_eq!(
        ::std::mem::size_of::<svm_ghcb_msr__bindgen_ty_2>(),
        8usize,
        concat!("Size of: ", stringify!(svm_ghcb_msr__bindgen_ty_2))
    );
    assert_eq!(
        ::std::mem::align_of::<svm_ghcb_msr__bindgen_ty_2>(),
        1usize,
        concat!("Alignment of ", stringify!(svm_ghcb_msr__bindgen_ty_2))
    );
}
impl svm_ghcb_msr__bindgen_ty_2 {
    #[inline]
    pub fn ghcb_info(&self) -> __u64 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(0usize, 12u8) as u64) }
    }
    #[inline]
    pub fn set_ghcb_info(&mut self, val: __u64) {
        unsafe {
            let val: u64 = ::std::mem::transmute(val);
            self._bitfield_1.set(0usize, 12u8, val as u64)
        }
    }
    #[inline]
    pub fn gpa_page_number(&self) -> __u64 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(12usize, 40u8) as u64) }
    }
    #[inline]
    pub fn set_gpa_page_number(&mut self, val: __u64) {
        unsafe {
            let val: u64 = ::std::mem::transmute(val);
            self._bitfield_1.set(12usize, 40u8, val as u64)
        }
    }
    #[inline]
    pub fn extra_data(&self) -> __u64 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(52usize, 12u8) as u64) }
    }
    #[inline]
    pub fn set_extra_data(&mut self, val: __u64) {
        unsafe {
            let val: u64 = ::std::mem::transmute(val);
            self._bitfield_1.set(52usize, 12u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(
        ghcb_info: __u64,
        gpa_page_number: __u64,
        extra_data: __u64,
    ) -> __BindgenBitfieldUnit<[u8; 8usize]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 8usize]> = Default::default();
        __bindgen_bitfield_unit.set(0usize, 12u8, {
            let ghcb_info: u64 = unsafe { ::std::mem::transmute(ghcb_info) };
            ghcb_info as u64
        });
        __bindgen_bitfield_unit.set(12usize, 40u8, {
            let gpa_page_number: u64 = unsafe { ::std::mem::transmute(gpa_page_number) };
            gpa_page_number as u64
        });
        __bindgen_bitfield_unit.set(52usize, 12u8, {
            let extra_data: u64 = unsafe { ::std::mem::transmute(extra_data) };
            extra_data as u64
        });
        __bindgen_bitfield_unit
    }
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct svm_ghcb_msr__bindgen_ty_3 {
    pub _bitfield_align_1: [u8; 0],
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 8usize]>,
}
#[test]
fn bindgen_test_layout_svm_ghcb_msr__bindgen_ty_3() {
    assert_eq!(
        ::std::mem::size_of::<svm_ghcb_msr__bindgen_ty_3>(),
        8usize,
        concat!("Size of: ", stringify!(svm_ghcb_msr__bindgen_ty_3))
    );
    assert_eq!(
        ::std::mem::align_of::<svm_ghcb_msr__bindgen_ty_3>(),
        1usize,
        concat!("Alignment of ", stringify!(svm_ghcb_msr__bindgen_ty_3))
    );
}
impl svm_ghcb_msr__bindgen_ty_3 {
    #[inline]
    pub fn reserved(&self) -> __u64 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(12usize, 18u8) as u64) }
    }
    #[inline]
    pub fn set_reserved(&mut self, val: __u64) {
        unsafe {
            let val: u64 = ::std::mem::transmute(val);
            self._bitfield_1.set(12usize, 18u8, val as u64)
        }
    }
    #[inline]
    pub fn cpuid_register(&self) -> __u64 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(30usize, 2u8) as u64) }
    }
    #[inline]
    pub fn set_cpuid_register(&mut self, val: __u64) {
        unsafe {
            let val: u64 = ::std::mem::transmute(val);
            self._bitfield_1.set(30usize, 2u8, val as u64)
        }
    }
    #[inline]
    pub fn cpuid_function(&self) -> __u64 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(32usize, 32u8) as u64) }
    }
    #[inline]
    pub fn set_cpuid_function(&mut self, val: __u64) {
        unsafe {
            let val: u64 = ::std::mem::transmute(val);
            self._bitfield_1.set(32usize, 32u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(
        reserved: __u64,
        cpuid_register: __u64,
        cpuid_function: __u64,
    ) -> __BindgenBitfieldUnit<[u8; 8usize]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 8usize]> = Default::default();
        __bindgen_bitfield_unit.set(12usize, 18u8, {
            let reserved: u64 = unsafe { ::std::mem::transmute(reserved) };
            reserved as u64
        });
        __bindgen_bitfield_unit.set(30usize, 2u8, {
            let cpuid_register: u64 = unsafe { ::std::mem::transmute(cpuid_register) };
            cpuid_register as u64
        });
        __bindgen_bitfield_unit.set(32usize, 32u8, {
            let cpuid_function: u64 = unsafe { ::std::mem::transmute(cpuid_function) };
            cpuid_function as u64
        });
        __bindgen_bitfield_unit
    }
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct svm_ghcb_msr__bindgen_ty_4 {
    pub _bitfield_align_1: [u8; 0],
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 8usize]>,
}
#[test]
fn bindgen_test_layout_svm_ghcb_msr__bindgen_ty_4() {
    assert_eq!(
        ::std::mem::size_of::<svm_ghcb_msr__bindgen_ty_4>(),
        8usize,
        concat!("Size of: ", stringify!(svm_ghcb_msr__bindgen_ty_4))
    );
    assert_eq!(
        ::std::mem::align_of::<svm_ghcb_msr__bindgen_ty_4>(),
        1usize,
        concat!("Alignment of ", stringify!(svm_ghcb_msr__bindgen_ty_4))
    );
}
impl svm_ghcb_msr__bindgen_ty_4 {
    #[inline]
    pub fn features(&self) -> __u64 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(12usize, 52u8) as u64) }
    }
    #[inline]
    pub fn set_features(&mut self, val: __u64) {
        unsafe {
            let val: u64 = ::std::mem::transmute(val);
            self._bitfield_1.set(12usize, 52u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(features: __u64) -> __BindgenBitfieldUnit<[u8; 8usize]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 8usize]> = Default::default();
        __bindgen_bitfield_unit.set(12usize, 52u8, {
            let features: u64 = unsafe { ::std::mem::transmute(features) };
            features as u64
        });
        __bindgen_bitfield_unit
    }
}
#[test]
fn bindgen_test_layout_svm_ghcb_msr() {
    const UNINIT: ::std::mem::MaybeUninit<svm_ghcb_msr> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<svm_ghcb_msr>(),
        8usize,
        concat!("Size of: ", stringify!(svm_ghcb_msr))
    );
    assert_eq!(
        ::std::mem::align_of::<svm_ghcb_msr>(),
        1usize,
        concat!("Alignment of ", stringify!(svm_ghcb_msr))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).as_uint64) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_msr),
            "::",
            stringify!(as_uint64)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).as_uint16) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_msr),
            "::",
            stringify!(as_uint16)
        )
    );
}
impl Default for svm_ghcb_msr {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union hv_sev_vmgexit_port_info {
    pub as_uint32: __u32,
    pub __bindgen_anon_1: hv_sev_vmgexit_port_info__bindgen_ty_1,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct hv_sev_vmgexit_port_info__bindgen_ty_1 {
    pub _bitfield_align_1: [u8; 0],
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 4usize]>,
}
#[test]
fn bindgen_test_layout_hv_sev_vmgexit_port_info__bindgen_ty_1() {
    assert_eq!(
        ::std::mem::size_of::<hv_sev_vmgexit_port_info__bindgen_ty_1>(),
        4usize,
        concat!(
            "Size of: ",
            stringify!(hv_sev_vmgexit_port_info__bindgen_ty_1)
        )
    );
    assert_eq!(
        ::std::mem::align_of::<hv_sev_vmgexit_port_info__bindgen_ty_1>(),
        1usize,
        concat!(
            "Alignment of ",
            stringify!(hv_sev_vmgexit_port_info__bindgen_ty_1)
        )
    );
}

impl hv_sev_vmgexit_port_info__bindgen_ty_1 {
    #[inline]
    pub fn access_type(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(0usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_access_type(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(0usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn reserved1(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(1usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_reserved1(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(1usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn string_based_port_access(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(2usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_string_based_port_access(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(2usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn repeated_port_access(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(3usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_repeated_port_access(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(3usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn operand_size_8bit(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(4usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_operand_size_8bit(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(4usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn operand_size_16bit(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(5usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_operand_size_16bit(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(5usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn operand_size_32bit(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(6usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_operand_size_32bit(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(6usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn address_16bit(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(7usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_address_16bit(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(7usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn address_32bit(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(8usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_address_32bit(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(8usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn address_64bit(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(9usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_address_64bit(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(9usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn effective_segment_number(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(10usize, 3u8) as u32) }
    }
    #[inline]
    pub fn set_effective_segment_number(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(10usize, 3u8, val as u64)
        }
    }
    #[inline]
    pub fn reserved2(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(13usize, 3u8) as u32) }
    }
    #[inline]
    pub fn set_reserved2(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(13usize, 3u8, val as u64)
        }
    }
    #[inline]
    pub fn intercepted_port(&self) -> __u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(16usize, 16u8) as u32) }
    }
    #[inline]
    pub fn set_intercepted_port(&mut self, val: __u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(16usize, 16u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(
        access_type: __u32,
        reserved1: __u32,
        string_based_port_access: __u32,
        repeated_port_access: __u32,
        operand_size_8bit: __u32,
        operand_size_16bit: __u32,
        operand_size_32bit: __u32,
        address_16bit: __u32,
        address_32bit: __u32,
        address_64bit: __u32,
        effective_segment_number: __u32,
        reserved2: __u32,
        intercepted_port: __u32,
    ) -> __BindgenBitfieldUnit<[u8; 4usize]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 4usize]> = Default::default();
        __bindgen_bitfield_unit.set(0usize, 1u8, {
            let access_type: u32 = unsafe { ::std::mem::transmute(access_type) };
            access_type as u64
        });
        __bindgen_bitfield_unit.set(1usize, 1u8, {
            let reserved1: u32 = unsafe { ::std::mem::transmute(reserved1) };
            reserved1 as u64
        });
        __bindgen_bitfield_unit.set(2usize, 1u8, {
            let string_based_port_access: u32 =
                unsafe { ::std::mem::transmute(string_based_port_access) };
            string_based_port_access as u64
        });
        __bindgen_bitfield_unit.set(3usize, 1u8, {
            let repeated_port_access: u32 = unsafe { ::std::mem::transmute(repeated_port_access) };
            repeated_port_access as u64
        });
        __bindgen_bitfield_unit.set(4usize, 1u8, {
            let operand_size_8bit: u32 = unsafe { ::std::mem::transmute(operand_size_8bit) };
            operand_size_8bit as u64
        });
        __bindgen_bitfield_unit.set(5usize, 1u8, {
            let operand_size_16bit: u32 = unsafe { ::std::mem::transmute(operand_size_16bit) };
            operand_size_16bit as u64
        });
        __bindgen_bitfield_unit.set(6usize, 1u8, {
            let operand_size_32bit: u32 = unsafe { ::std::mem::transmute(operand_size_32bit) };
            operand_size_32bit as u64
        });
        __bindgen_bitfield_unit.set(7usize, 1u8, {
            let address_16bit: u32 = unsafe { ::std::mem::transmute(address_16bit) };
            address_16bit as u64
        });
        __bindgen_bitfield_unit.set(8usize, 1u8, {
            let address_32bit: u32 = unsafe { ::std::mem::transmute(address_32bit) };
            address_32bit as u64
        });
        __bindgen_bitfield_unit.set(9usize, 1u8, {
            let address_64bit: u32 = unsafe { ::std::mem::transmute(address_64bit) };
            address_64bit as u64
        });
        __bindgen_bitfield_unit.set(10usize, 3u8, {
            let effective_segment_number: u32 =
                unsafe { ::std::mem::transmute(effective_segment_number) };
            effective_segment_number as u64
        });
        __bindgen_bitfield_unit.set(13usize, 3u8, {
            let reserved2: u32 = unsafe { ::std::mem::transmute(reserved2) };
            reserved2 as u64
        });
        __bindgen_bitfield_unit.set(16usize, 16u8, {
            let intercepted_port: u32 = unsafe { ::std::mem::transmute(intercepted_port) };
            intercepted_port as u64
        });
        __bindgen_bitfield_unit
    }
}
#[test]
fn bindgen_test_layout_hv_sev_vmgexit_port_info() {
    const UNINIT: ::std::mem::MaybeUninit<hv_sev_vmgexit_port_info> =
        ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<hv_sev_vmgexit_port_info>(),
        4usize,
        concat!("Size of: ", stringify!(hv_sev_vmgexit_port_info))
    );
    assert_eq!(
        ::std::mem::align_of::<hv_sev_vmgexit_port_info>(),
        4usize,
        concat!("Alignment of ", stringify!(hv_sev_vmgexit_port_info))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).as_uint32) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(hv_sev_vmgexit_port_info),
            "::",
            stringify!(as_uint32)
        )
    );
}
impl Default for hv_sev_vmgexit_port_info {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}

///
/// Get default VMGEXIT offload features supported by
/// Microsoft Hypervisor.
///
pub fn get_default_vmgexit_offload_features() -> hv_sev_vmgexit_offload {
    let mut offload_feature = hv_sev_vmgexit_offload::default();

    unsafe {
        offload_feature.__bindgen_anon_1.set_nae_rdtsc(1);
        offload_feature.__bindgen_anon_1.set_nae_cpuid(1);
        offload_feature.__bindgen_anon_1.set_nae_rdmsr(1);
        offload_feature.__bindgen_anon_1.set_nae_wrmsr(1);
        offload_feature.__bindgen_anon_1.set_nae_vmmcall(1);
        offload_feature.__bindgen_anon_1.set_nae_wbinvd(1);
        offload_feature
            .__bindgen_anon_1
            .set_nae_snp_page_state_change(1);
        offload_feature.__bindgen_anon_1.set_msr_cpuid(1);
        offload_feature
            .__bindgen_anon_1
            .set_msr_snp_page_state_change(1);
    }

    offload_feature
}

///
/// Helper function to parse the GPA range
///
pub fn parse_gpa_range(range: hv_gpa_page_range) -> Result<(u64, u64)> {
    let gpa_page_start;
    let gpa_page_count;

    // SAFETY: access union field
    unsafe {
        if range.page.largepage() > 0 {
            return Err(errno::Error::new(libc::EINVAL));
        } else {
            gpa_page_start = range.page.basepfn();
            gpa_page_count = 1 + range.page.additional_pages();
        }
    }
    Ok((gpa_page_start, gpa_page_count))
}

///
/// Get default SEV-SNP guest policy supported by
/// Microsoft Hypervisor.
///
pub fn get_default_snp_guest_policy() -> hv_snp_guest_policy {
    let mut snp_policy = hv_snp_guest_policy { as_uint64: 0_u64 };

    // SAFETY: access union field
    unsafe {
        snp_policy.__bindgen_anon_1.set_minor_version(0x1f);
        snp_policy.__bindgen_anon_1.set_major_version(0x00);
        snp_policy.__bindgen_anon_1.set_smt_allowed(1);
        snp_policy.__bindgen_anon_1.set_vmpls_required(1);
        snp_policy.__bindgen_anon_1.set_migration_agent_allowed(0);
        snp_policy.__bindgen_anon_1.set_debug_allowed(0);
    }

    snp_policy
}

///
/// Helper function to get sev control register
/// for a given VMSA PFN.
///
pub fn get_sev_control_register(vmsa_pfn: u64) -> u64 {
    let mut sev_control = hv_x64_register_sev_control { as_uint64: 0_u64 };

    // SAFETY: access union field
    unsafe {
        sev_control.__bindgen_anon_1.set_enable_encrypted_state(1);
        sev_control
            .__bindgen_anon_1
            .set_vmsa_gpa_page_number(vmsa_pfn);
        sev_control.as_uint64
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct svm_ghcb_base {
    pub unused: [__u8; 203usize],
    pub cpl: __u8,
    pub unused2: [__u8; 116usize],
    pub xss: __u64,
    pub unused3: [__u8; 24usize],
    pub dr7: __u64,
    pub unused4: [__u8; 144usize],
    pub rax: __u64,
    pub unused5: [__u8; 264usize],
    pub rcx: __u64,
    pub rdx: __u64,
    pub rbx: __u64,
    pub unused6: __u64,
    pub rbp: __u64,
    pub rsi: __u64,
    pub rdi: __u64,
    pub r8: __u64,
    pub r9: __u64,
    pub r10: __u64,
    pub r11: __u64,
    pub r12: __u64,
    pub r13: __u64,
    pub r14: __u64,
    pub r15: __u64,
    pub unused7: [__u8; 16usize],
    pub exit_code: __u64,
    pub exit_info1: __u64,
    pub exit_info2: __u64,
    pub scratch: __u64,
    pub unused8: [__u8; 56usize],
    pub xfem: __u64,
    pub valid_bitmap: [__u64; 2usize],
    pub x87_state_gpa: __u64,
    pub reserved: [__u64; 127usize],
    pub shared: [__u64; 254usize],
}

// SAFETY: struct is based on GHCB page in the hypervisor,
// safe to send across threads
unsafe impl Send for svm_ghcb_base {}

#[test]
fn bindgen_test_layout_svm_ghcb_base() {
    const UNINIT: ::std::mem::MaybeUninit<svm_ghcb_base> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<svm_ghcb_base>(),
        4080usize,
        concat!("Size of: ", stringify!(svm_ghcb_base))
    );
    assert_eq!(
        ::std::mem::align_of::<svm_ghcb_base>(),
        1usize,
        concat!("Alignment of ", stringify!(svm_ghcb_base))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).unused) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(unused)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).cpl) as usize - ptr as usize },
        203usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(cpl)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).unused2) as usize - ptr as usize },
        204usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(unused2)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).xss) as usize - ptr as usize },
        320usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(xss)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).unused3) as usize - ptr as usize },
        328usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(unused3)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).dr7) as usize - ptr as usize },
        352usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(dr7)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).unused4) as usize - ptr as usize },
        360usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(unused4)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).rax) as usize - ptr as usize },
        504usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(rax)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).unused5) as usize - ptr as usize },
        512usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(unused5)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).rcx) as usize - ptr as usize },
        776usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(rcx)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).rdx) as usize - ptr as usize },
        784usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(rdx)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).rbx) as usize - ptr as usize },
        792usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(rbx)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).unused6) as usize - ptr as usize },
        800usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(unused6)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).rbp) as usize - ptr as usize },
        808usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(rbp)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).rsi) as usize - ptr as usize },
        816usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(rsi)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).rdi) as usize - ptr as usize },
        824usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(rdi)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).r8) as usize - ptr as usize },
        832usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(r8)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).r9) as usize - ptr as usize },
        840usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(r9)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).r10) as usize - ptr as usize },
        848usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(r10)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).r11) as usize - ptr as usize },
        856usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(r11)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).r12) as usize - ptr as usize },
        864usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(r12)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).r13) as usize - ptr as usize },
        872usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(r13)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).r14) as usize - ptr as usize },
        880usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(r14)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).r15) as usize - ptr as usize },
        888usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(r15)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).unused7) as usize - ptr as usize },
        896usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(unused7)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).exit_code) as usize - ptr as usize },
        912usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(rxit_code)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).exit_info1) as usize - ptr as usize },
        920usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(exit_info1)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).exit_info2) as usize - ptr as usize },
        928usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(exit_info2)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).scratch) as usize - ptr as usize },
        936usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(scratch)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).unused8) as usize - ptr as usize },
        944usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(unused8)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).xfem) as usize - ptr as usize },
        1000usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(xfem)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).valid_bitmap) as usize - ptr as usize },
        1008usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(valid_bitmap)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).x87_state_gpa) as usize - ptr as usize },
        1024usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(x87_state_gpa)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).reserved) as usize - ptr as usize },
        1032usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(reserved)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).shared) as usize - ptr as usize },
        2048usize,
        concat!(
            "Offset of field: ",
            stringify!(svm_ghcb_base),
            "::",
            stringify!(shared)
        )
    );
}
impl Default for svm_ghcb_base {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! set_svm_field_u64_ptr {
    ($this: ident, $name: ident, $value: expr) => {
        #[allow(clippy::macro_metavars_in_unsafe)]
        unsafe {
            (*$this).$name = $value
        };
        let qword_offset = std::mem::offset_of!(svm_ghcb_base, $name) / 8;
        let arr_idx = qword_offset / 64;
        let bit_index = qword_offset % 64;
        let mut bitmap = unsafe { (*$this).valid_bitmap };
        bitmap[arr_idx] |= (1 << bit_index);
        unsafe { (*$this).valid_bitmap = bitmap };
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svm_macro() {
        let mut st: svm_ghcb_base = svm_ghcb_base::default();
        println!("{}", std::mem::size_of::<svm_ghcb_base>());
        let pptr: *mut svm_ghcb_base = core::ptr::addr_of_mut!(st);
        set_svm_field_u64_ptr!(pptr, cpl, 100);
        let mut val = st.cpl as u64;
        let mut bitmap = st.valid_bitmap;
        assert!(100 == val);
        assert!((33554432 == bitmap[0]));
        set_svm_field_u64_ptr!(pptr, xfem, 0x100);
        val = st.xfem;
        assert!(0x100 == val);
        bitmap = st.valid_bitmap;
        assert!(0x2000000000000000 == bitmap[1]);
    }
}
