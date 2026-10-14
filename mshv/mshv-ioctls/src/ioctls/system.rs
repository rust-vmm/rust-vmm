// Copyright © 2020, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//
use crate::ioctls::vm::{new_vmfd, VmFd, VmType};
use crate::ioctls::Result;
use crate::mshv_ioctls::*;
use crate::*;
use libc::{open, O_CLOEXEC, O_NONBLOCK};
use mshv_bindings::*;
use std::fs::File;
use std::os::raw::c_char;
use std::os::unix::io::{FromRawFd, RawFd};
use vmm_sys_util::errno;
use vmm_sys_util::ioctl::{ioctl_with_mut_ref, ioctl_with_ref};

/// Wrapper over MSHV system ioctls.
#[derive(Debug)]
pub struct Mshv {
    hv: File,
}

impl Mshv {
    /// Opens `/dev/mshv` and returns a `Mshv` object on success.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Result<Self> {
        // Open `/dev/mshv` using `O_CLOEXEC` flag.
        let fd = Self::open_with_cloexec(true)?;
        // SAFETY: we verify that ret is valid and we own the fd.
        let ret = unsafe { Self::new_with_fd_number(fd) };
        Ok(ret)
    }

    /// Creates a new Mshv object assuming `fd` represents an existing open file descriptor
    /// associated with `/dev/mshv`.
    ///
    /// # Safety
    ///
    /// This function is unsafe as the primitives currently returned have the contract that
    /// they are the sole owner of the file descriptor they are wrapping. Usage of this function
    /// could accidentally allow violating this contract which can cause memory unsafety in code
    /// that relies on it being true.
    ///
    /// The caller of this method must make sure the fd is valid and nothing else uses it.
    pub unsafe fn new_with_fd_number(fd: RawFd) -> Self {
        Mshv {
            hv: File::from_raw_fd(fd),
        }
    }

    /// Opens `/dev/mshv` and returns the fd number on success.
    pub fn open_with_cloexec(close_on_exec: bool) -> Result<RawFd> {
        let open_flags = O_NONBLOCK | if close_on_exec { O_CLOEXEC } else { 0 };
        // SAFETY: we give a constant null-terminated string and verify the result.
        let ret = unsafe { open(c"/dev/mshv".as_ptr() as *const c_char, open_flags) };
        if ret < 0 {
            Err(errno::Error::last().into())
        } else {
            Ok(ret)
        }
    }

    /// Helper function to prepare partition creation argument based on host properties
    pub fn make_default_partition_create_arg(&self, vm_type: VmType) -> mshv_create_partition_v2 {
        let pt_flags: u64 = set_bits!(
            u64,
            MSHV_PT_BIT_LAPIC,
            MSHV_PT_BIT_X2APIC,
            MSHV_PT_BIT_GPA_SUPER_PAGES,
            MSHV_PT_BIT_CPU_AND_XSAVE_FEATURES
        );
        let mut pt_isolation: u64 = MSHV_PT_ISOLATION_NONE as u64;

        if vm_type == VmType::Snp {
            pt_isolation = MSHV_PT_ISOLATION_SNP as u64;
        }

        let mut create_args = mshv_create_partition_v2 {
            pt_flags,
            pt_isolation,
            pt_num_cpu_fbanks: MSHV_NUM_CPU_FEATURES_BANKS as u16,
            ..Default::default()
        };

        #[cfg(target_arch = "x86_64")]
        // SAFETY: access union fields
        unsafe {
            let mut disabled_xsave_features = hv_partition_processor_xsave_features::default();
            disabled_xsave_features.as_uint64 = 0xFFFFFFFFFFFFFFFF;

            // Enable default XSave features that are known to be supported
            disabled_xsave_features
                .__bindgen_anon_1
                .set_avx_support(0u64);
            disabled_xsave_features
                .__bindgen_anon_1
                .set_xsave_comp_support(0u64);
            disabled_xsave_features
                .__bindgen_anon_1
                .set_xsave_supervisor_support(0u64);
            disabled_xsave_features
                .__bindgen_anon_1
                .set_xsave_support(0u64);
            disabled_xsave_features
                .__bindgen_anon_1
                .set_xsaveopt_support(0u64);
            disabled_xsave_features
                .__bindgen_anon_1
                .set_avx2_support(0u64);
            disabled_xsave_features
                .__bindgen_anon_1
                .set_fma_support(0u64);
            create_args.pt_disabled_xsave = disabled_xsave_features.as_uint64;
        }

        let mut disabled_cpu_features: hv_partition_processor_features = Default::default();
        let host_proc_features0 = self
            .get_host_partition_property(
                hv_partition_property_code_HV_PARTITION_PROPERTY_PROCESSOR_FEATURES0,
            )
            .unwrap();
        let host_proc_features1 = self
            .get_host_partition_property(
                hv_partition_property_code_HV_PARTITION_PROPERTY_PROCESSOR_FEATURES1,
            )
            .unwrap();
        unsafe {
            // pt_cpu_fbanks expects _disabled_ processor features (bit = 1 means disabled)
            // whereas host_proc_features are _enabled_ features (bit = 1 means enabled).
            // So we invert the bits here.
            disabled_cpu_features.as_uint64[0] = !host_proc_features0;
            disabled_cpu_features.as_uint64[1] = !host_proc_features1;
            // We can get the features from host property but we should not blindly enable
            // the reserved bits(that may accidentaly be enabled with newer hypervisor version or hardware)
            // and may impact live migration.
            #[cfg(target_arch = "x86_64")]
            disabled_cpu_features
                .__bindgen_anon_1
                .set_reserved_bank0(0xFFFFFFFFFFFFFFFF);
            #[cfg(target_arch = "aarch64")]
            disabled_cpu_features
                .__bindgen_anon_1
                .set_reserved_bank1(0xFFFFFFFFFFFFFFFF);
            create_args.pt_cpu_fbanks[0] = disabled_cpu_features.as_uint64[0];
            create_args.pt_cpu_fbanks[1] = disabled_cpu_features.as_uint64[1];
        };

        create_args
    }

    /// Creates a VM fd using the MSHV fd and prepared mshv partition.
    pub fn create_vm_with_args(&self, args: &mshv_create_partition_v2) -> Result<VmFd> {
        // SAFETY: IOCTL call with the correct types.
        let ret = unsafe { ioctl_with_ref(&self.hv, MSHV_CREATE_PARTITION(), args) };
        if ret >= 0 {
            // SAFETY: we verify the value of ret and we are the owners of the fd.
            let vm_file = unsafe { File::from_raw_fd(ret) };
            Ok(new_vmfd(vm_file))
        } else {
            Err(errno::Error::last().into())
        }
    }

    /// Retrieve the host partition property given a property code.
    pub fn get_host_partition_property(&self, property_code: u32) -> Result<u64> {
        let input = hv_input_get_partition_property {
            property_code,
            ..Default::default() // NOTE: Kernel will populate partition_id field
        };

        let mut output = hv_output_get_partition_property {
            ..Default::default()
        };

        let mut args = make_args!(HVCALL_GET_PARTITION_PROPERTY, input, output);
        self.hvcall(&mut args).map(|_| output.property_value)
    }

    /// Helper function to populate synthetic features for the partition to create
    pub fn make_default_synthetic_features_mask(&self) -> u64 {
        let mut features: hv_partition_synthetic_processor_features = Default::default();
        // SAFETY: access union fields
        unsafe {
            let feature_bits = &mut features.__bindgen_anon_1;
            feature_bits.set_hypervisor_present(1);
            feature_bits.set_hv1(1);
            feature_bits.set_access_partition_reference_counter(1);
            feature_bits.set_access_synic_regs(1);
            feature_bits.set_access_synthetic_timer_regs(1);
            feature_bits.set_access_partition_reference_tsc(1);
            feature_bits.set_access_frequency_regs(1);
            feature_bits.set_access_intr_ctrl_regs(1);
            feature_bits.set_access_vp_index(1);
            feature_bits.set_access_hypercall_regs(1);
            #[cfg(not(target_arch = "aarch64"))]
            feature_bits.set_access_guest_idle_reg(1);
            #[cfg(not(target_arch = "aarch64"))]
            feature_bits.set_tb_flush_hypercalls(1);
            feature_bits.set_synthetic_cluster_ipi(1);
            feature_bits.set_direct_synthetic_timers(1);
            feature_bits.set_access_vp_regs(1);
        }

        let vmm_caps = self.get_vmm_caps().unwrap();

        // SAFETY: access union fields
        let assignable_supported = unsafe {
            vmm_caps
                .__bindgen_anon_1
                .__bindgen_anon_1
                .assignable_synthetic_proc_features()
                == 1
        };

        if assignable_supported {
            #[cfg(not(target_arch = "aarch64"))]
            // SAFETY: access union fields
            unsafe {
                features
                    .__bindgen_anon_1
                    .set_flush_guest_physical_address_space(1);
            }
            // Intersect our default mask with the hypervisor-advertised set of
            // assignable synthetic features. The property is in the extended range
            // (0x00090xxx) and must be fetched via HVCALL_GET_PARTITION_PROPERTY_EX.
            // The payload follows the banked property shape used by
            // `hv_partition_property_vmm_capabilities`: u16 bank_count, three u16
            // reserved words, then `bank_count` u64 masks. We use bank 0.
            let assignable_ex = self
                .get_host_partition_property_ex(
                    hv_partition_property_code_HV_PARTITION_PROPERTY_ASSIGNABLE_SYNTHETIC_PROC_FEATURES,
                )
                .unwrap();
            // SAFETY: `hv_partition_property_ex` is a union whose `buffer` variant
            // covers the full 4072-byte payload; reading the first bank u64 at
            // offset 8 is within bounds.
            let assignable_features: u64 = unsafe {
                std::ptr::read_unaligned(assignable_ex.buffer.as_ptr().add(8) as *const u64)
            };
            // SAFETY: access union fields
            unsafe { features.as_uint64[0] & assignable_features }
        } else {
            // SAFETY: access union fields
            unsafe { features.as_uint64[0] }
        }
    }

    /// Helper function to creates a VM fd using the MSHV fd with provided configuration.
    pub fn create_vm_with_type(&self, vm_type: VmType) -> Result<VmFd> {
        let create_args = self.make_default_partition_create_arg(vm_type);

        let vm = self.create_vm_with_args(&create_args)?;

        // This is an 'early' property that must be set between creation and initialization
        vm.set_partition_property(
            hv_partition_property_code_HV_PARTITION_PROPERTY_SYNTHETIC_PROC_FEATURES,
            self.make_default_synthetic_features_mask(),
        )?;

        Ok(vm)
    }

    /// Creates a VM fd using the MSHV fd.
    pub fn create_vm(&self) -> Result<VmFd> {
        self.create_vm_with_type(VmType::Normal)
    }

    /// Execute a hypercall targeting the host partition (self).
    fn hvcall(&self, args: &mut mshv_root_hvcall) -> Result<()> {
        // SAFETY: IOCTL with correct types
        let ret = unsafe { ioctl_with_mut_ref(&self.hv, MSHV_ROOT_HVCALL(), args) };
        if ret == 0 {
            Ok(())
        } else {
            Err(MshvError::from_hvcall(errno::Error::last(), *args))
        }
    }

    fn get_host_partition_property_ex(
        &self,
        property_code: u32,
    ) -> Result<Box<hv_partition_property_ex>> {
        let mut input = hv_input_get_partition_property_ex {
            property_code,
            ..Default::default() // NOTE: Kernel will populate partition_id field
        };

        input.__bindgen_anon_1.arg = 0;

        let mut output = Box::new(hv_partition_property_ex::default());

        let mut args = make_args!(HVCALL_GET_PARTITION_PROPERTY_EX, input, *output);

        self.hvcall(&mut args).map(|_| output)
    }

    /// Get VMM capabilities
    pub fn get_vmm_caps(&self) -> Result<hv_partition_property_vmm_capabilities> {
        match self.get_host_partition_property_ex(
            hv_partition_property_code_HV_PARTITION_PROPERTY_VMM_CAPABILITIES,
        ) {
            Ok(output) => {
                // SAFETY: accessing union fields
                let vmm_caps = unsafe { output.vmm_capabilities };
                Ok(vmm_caps)
            }
            Err(MshvError::Hypercall {
                status: Some(HvError::AccessDenied | HvError::UnknownProperty),
                ..
            }) => {
                // If the hypercall fails with AccessDenied or UnknownProperty, it means the
                // VMM capabilities query is not supported on this MSHV version.
                // As per the hypervisor team, we should treat this case as if
                // none of the capabilities are set. So, return a zeroed out struct.
                Ok(hv_partition_property_vmm_capabilities::default())
            }
            Err(e) => Err(e),
        }
    }

    #[cfg(target_arch = "x86_64")]
    /// X86 specific call to get list of supported MSRs
    pub fn get_msr_index_list(&self) -> Result<Vec<u32>> {
        let mut msrs: Vec<u32> = Vec::new();
        msrs.extend_from_slice(MSRS_COMMON);
        msrs.extend_from_slice(MSRS_CET_SS);
        msrs.extend_from_slice(MSRS_SYNIC);
        msrs.extend_from_slice(MSRS_OTHER);
        Ok(msrs)
    }
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    #[ignore]
    fn test_create_vm() {
        let hv = Mshv::new().unwrap();
        let vm = hv.create_vm();
        assert!(vm.is_ok());
    }

    #[test]
    fn test_get_host_ipa_limit() {
        let hv = Mshv::new().unwrap();
        let host_ipa_limit = hv.get_host_partition_property(
            hv_partition_property_code_HV_PARTITION_PROPERTY_PHYSICAL_ADDRESS_WIDTH,
        );
        assert!(host_ipa_limit.is_ok());
    }

    #[test]
    #[ignore]
    fn test_create_vm_with_default_config() {
        let hv = Mshv::new().unwrap();
        let pr = hv.make_default_partition_create_arg(VmType::Normal);
        let vm = hv.create_vm_with_args(&pr);
        assert!(vm.is_ok());
    }

    #[test]
    fn test_vmm_caps() {
        let hv = Mshv::new().unwrap();
        let vmm_caps = hv.get_vmm_caps();
        assert!(vmm_caps.is_ok());
    }
}
