// SPDX-License-Identifier: Apache-2.0 OR MIT

pub mod types {
    pub const ZEROCOPY: &[&str] = &[
        "kvm_mp_state",
        "kvm_irq_routing_irqchip",
        "kvm_irq_routing_msi",
        "kvm_irq_routing_s390_adapter",
        "kvm_irq_routing_hv_sint",
        "kvm_irq_routing_xen_evtchn",
        "kvm_irq_routing_entry",
        "kvm_irq_routing",
    ];

    pub const ZEROCOPY_IMMUTABLE_FROM_BYTES: &[&str] = &[
        "kvm_irq_routing_msi__bindgen_ty_1",
        "kvm_irq_routing_entry__bindgen_ty_1",
    ];

    pub mod arm64 {
        pub const ZEROCOPY: &[&str] = &[
            "kvm_one_reg",
            "user_pt_regs",
            "user_fpsimd_state",
            "kvm_vcpu_init",
            "kvm_regs",
        ];
    }

    pub mod riscv64 {
        pub const ZEROCOPY: &[&str] = &[
            "kvm_one_reg",
            "user_regs_struct",
            "kvm_riscv_config",
            "kvm_riscv_core",
            "kvm_riscv_csr",
            "kvm_riscv_aia_csr",
            "kvm_riscv_smstateen_csr",
            "kvm_riscv_timer",
            "kvm_riscv_sbi_sta",
        ];
    }

    pub mod x86_64 {
        pub const ZEROCOPY: &[&str] = &[
            "kvm_ioapic_state",
            "kvm_ioapic_state__bindgen_ty_1__bindgen_ty_1",
            "kvm_pic_state",
            "kvm_regs",
            "kvm_lapic_state",
            "kvm_segment",
            "kvm_dtable",
            "kvm_sregs",
            "kvm_msr_entry",
            "kvm_msrs",
            "kvm_cpuid_entry2",
            "kvm_cpuid2",
            "kvm_pit_channel_state",
            "kvm_pit_state2",
            "kvm_vcpu_events",
            "kvm_vcpu_events__bindgen_ty_1",
            "kvm_vcpu_events__bindgen_ty_2",
            "kvm_vcpu_events__bindgen_ty_3",
            "kvm_vcpu_events__bindgen_ty_4",
            "kvm_vcpu_events__bindgen_ty_5",
            "kvm_debugregs",
            "kvm_xsave",
            "kvm_xcr",
            "kvm_xcrs",
            "kvm_vmx_nested_state_hdr",
            "kvm_vmx_nested_state_hdr__bindgen_ty_1",
            "kvm_svm_nested_state_hdr",
            "kvm_irqchip",
            "kvm_clock_data",
        ];
        pub const ZEROCOPY_TRY_FROM_BYTES: &[&str] = &["kvm_irqchip__bindgen_ty_1"];
        pub const ZEROCOPY_IMMUTABLE_FROM_BYTES: &[&str] = &[
            "kvm_ioapic_state__bindgen_ty_1",
            "kvm_nested_state__bindgen_ty_1",
        ];
    }
}

pub mod attributes {
    pub const ZEROCOPY: &str = "#[cfg_attr(
    feature = \"serde\",
    derive(zerocopy::IntoBytes, zerocopy::Immutable, zerocopy::FromBytes)
)]";
    pub const ZEROCOPY_TRY_FROM_BYTES: &str = "#[cfg_attr(
    feature = \"serde\",
    derive(zerocopy::TryFromBytes)
)]";
    pub const ZEROCOPY_IMMUTABLE_FROM_BYTES: &str = "#[cfg_attr(
    feature = \"serde\",
    derive(zerocopy::Immutable, zerocopy::FromBytes)
)]";
}
