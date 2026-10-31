// Copyright © 2020, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//
use vmm_sys_util::fam::{FamStruct, FamStructWrapper};

use crate::regs::*;

pub const MAX_MSR_ENTRIES: usize = 256;

generate_fam_struct_impl!(msrs, msr_entry, entries, u32, nmsrs, MAX_MSR_ENTRIES);

pub type Msrs = FamStructWrapper<msrs>;

pub type CpuId = FamStructWrapper<hv_cpuid>;

pub const HV_MAX_CPUID_ENTRIES: usize = 256;
// Implement the FamStruct trait for hv_cpuid.
generate_fam_struct_impl!(
    hv_cpuid,
    hv_cpuid_entry,
    entries,
    u32,
    nent,
    HV_MAX_CPUID_ENTRIES
);

generate_fam_struct_impl!(msr_list, u32, indices, u32, nmsrs, MAX_MSR_ENTRIES);

pub type MsrList = FamStructWrapper<msr_list>;
