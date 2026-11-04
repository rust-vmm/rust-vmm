/* SPDX-License-Identifier: GPL-2.0 */
/*
 * Type definitions for the hypervisor host interface to kernel.
 */
#ifndef _HVHDK_MINI_H
#define _HVHDK_MINI_H

#include "hvgdk_mini.h"

#define HVHVK_MINI_VERSION		(25294)

/* Each generic set contains 64 elements */
#define HV_GENERIC_SET_SHIFT		(6)
#define HV_GENERIC_SET_MASK		(63)

enum hv_generic_set_format {
	HV_GENERIC_SET_SPARSE_4K,
	HV_GENERIC_SET_ALL,
};

enum hv_partition_property_code {
	/* Privilege properties */
    HV_PARTITION_PROPERTY_PRIVILEGE_FLAGS			= 0x00010000,
    HV_PARTITION_PROPERTY_SYNTHETIC_PROC_FEATURES		= 0x00010001,

    /* Scheduling properties */
    HV_PARTITION_PROPERTY_SUSPEND				= 0x00020000,
    HV_PARTITION_PROPERTY_CPU_RESERVE				= 0x00020001,
    HV_PARTITION_PROPERTY_CPU_CAP				= 0x00020002,
    HV_PARTITION_PROPERTY_CPU_WEIGHT				= 0x00020003,
    HV_PARTITION_PROPERTY_CPU_GROUP_ID				= 0x00020004,

    /* Time properties */
    HV_PARTITION_PROPERTY_TIME_FREEZE				= 0x00030003,
    HV_PARTITION_PROPERTY_REFERENCE_TIME			= 0x00030005,

    /* Debugging properties */
    HV_PARTITION_PROPERTY_DEBUG_CHANNEL_ID			= 0x00040000,

    /* Resource properties */
    HV_PARTITION_PROPERTY_VIRTUAL_TLB_PAGE_COUNT		= 0x00050000,
    HV_PARTITION_PROPERTY_VSM_CONFIG				= 0x00050001,
    HV_PARTITION_PROPERTY_ZERO_MEMORY_ON_RESET			= 0x00050002,
    HV_PARTITION_PROPERTY_PROCESSORS_PER_SOCKET			= 0x00050003,
    HV_PARTITION_PROPERTY_NESTED_TLB_SIZE			= 0x00050004,
    HV_PARTITION_PROPERTY_GPA_PAGE_ACCESS_TRACKING		= 0x00050005,
    HV_PARTITION_PROPERTY_VSM_PERMISSIONS_DIRTY_SINCE_LAST_QUERY = 0x00050006,
    HV_PARTITION_PROPERTY_SGX_LAUNCH_CONTROL_CONFIG		= 0x00050007,
    HV_PARTITION_PROPERTY_DEFAULT_SGX_LAUNCH_CONTROL0		= 0x00050008,
    HV_PARTITION_PROPERTY_DEFAULT_SGX_LAUNCH_CONTROL1		= 0x00050009,
    HV_PARTITION_PROPERTY_DEFAULT_SGX_LAUNCH_CONTROL2		= 0x0005000a,
    HV_PARTITION_PROPERTY_DEFAULT_SGX_LAUNCH_CONTROL3		= 0x0005000b,
    HV_PARTITION_PROPERTY_ISOLATION_STATE			= 0x0005000c,
    HV_PARTITION_PROPERTY_ISOLATION_CONTROL			= 0x0005000d,
    HV_PARTITION_PROPERTY_ALLOCATION_ID				= 0x0005000e,
    HV_PARTITION_PROPERTY_MONITORING_ID				= 0x0005000f,
    HV_PARTITION_PROPERTY_IMPLEMENTED_PHYSICAL_ADDRESS_BITS	= 0x00050010,
    HV_PARTITION_PROPERTY_NON_ARCHITECTURAL_CORE_SHARING	= 0x00050011,
    HV_PARTITION_PROPERTY_HYPERCALL_DOORBELL_PAGE		= 0x00050012,
    HV_PARTITION_PROPERTY_ISOLATION_POLICY			= 0x00050014,
    HV_PARTITION_PROPERTY_UNIMPLEMENTED_MSR_ACTION		= 0x00050017,
    HV_PARTITION_PROPERTY_SEV_VMGEXIT_OFFLOADS			= 0x00050022,
    HV_PARTITION_PROPERTY_PARTITION_DIAG_BUFFER_CONFIG  = 0x00050026,
    HV_PARTITION_PROPERTY_GICD_BASE_ADDRESS             = 0x00050028,
    HV_PARTITION_PROPERTY_GITS_TRANSLATER_BASE_ADDRESS  = 0x00050029,
    HV_PARTITION_PROPERTY_GIC_LPI_INT_ID_BITS                       = 0x0005002A,
    HV_PARTITION_PROPERTY_GIC_PPI_OVERFLOW_INTERRUPT_FROM_CNTV      = 0x0005002B,
    HV_PARTITION_PROPERTY_GIC_PPI_OVERFLOW_INTERRUPT_FROM_CNTP      = 0x0005002C,
    HV_PARTITION_PROPERTY_GIC_PPI_PERFORMANCE_MONITORS_INTERRUPT    = 0x0005002D,
    HV_PARTITION_PROPERTY_GIC_PPI_PMBIRQ                            = 0x0005002E,

    /* Compatibility properties */
    HV_PARTITION_PROPERTY_PROCESSOR_VENDOR			= 0x00060000,
    HV_PARTITION_PROPERTY_PROCESSOR_FEATURES_DEPRECATED		= 0x00060001,
    HV_PARTITION_PROPERTY_PROCESSOR_XSAVE_FEATURES		= 0x00060002,
    HV_PARTITION_PROPERTY_PROCESSOR_CL_FLUSH_SIZE		= 0x00060003,
    HV_PARTITION_PROPERTY_ENLIGHTENMENT_MODIFICATIONS		= 0x00060004,
    HV_PARTITION_PROPERTY_COMPATIBILITY_VERSION			= 0x00060005,
    HV_PARTITION_PROPERTY_PHYSICAL_ADDRESS_WIDTH		= 0x00060006,
    HV_PARTITION_PROPERTY_XSAVE_STATES				= 0x00060007,
    HV_PARTITION_PROPERTY_MAX_XSAVE_DATA_SIZE			= 0x00060008,
    HV_PARTITION_PROPERTY_PROCESSOR_CLOCK_FREQUENCY		= 0x00060009,
    HV_PARTITION_PROPERTY_PROCESSOR_FEATURES0			= 0x0006000a,
    HV_PARTITION_PROPERTY_PROCESSOR_FEATURES1			= 0x0006000b,

    /* Guest software properties */
    HV_PARTITION_PROPERTY_GUEST_OS_ID				= 0x00070000,

    /* Nested virtualization properties */
    HV_PARTITION_PROPERTY_PROCESSOR_VIRTUALIZATION_FEATURES	= 0x00080000,

    /* Extended properties with larger property values */
    HV_PARTITION_PROPERTY_VMM_CAPABILITIES				= 0x00090007,
    HV_PARTITION_PROPERTY_ASSIGNABLE_SYNTHETIC_PROC_FEATURES	= 0x00090009,
};

#define HV_PARTITION_VMM_CAPABILITIES_BANK_COUNT	1
#define HV_PARTITION_VMM_CAPABILITIES_RESERVED_BITFIELD_COUNT	59

/* HV Map GPA (Guest Physical Address) Flags */
#define HV_MAP_GPA_PERMISSIONS_NONE	       0x0
#define HV_MAP_GPA_READABLE		       0x1
#define HV_MAP_GPA_WRITABLE		       0x2
#define HV_MAP_GPA_KERNEL_EXECUTABLE	       0x4
#define HV_MAP_GPA_USER_EXECUTABLE	       0x8
#define HV_MAP_GPA_EXECUTABLE		       0xC
#define HV_MAP_GPA_PERMISSIONS_MASK	       0xF
#define HV_MAP_GPA_ADJUSTABLE		    0x8000
#define HV_MAP_GPA_NO_ACCESS		   0x10000
#define HV_MAP_GPA_NOT_CACHED		  0x200000
#define HV_MAP_GPA_LARGE_PAGE		0x80000000

#define HV_PFN_RNG_PAGEBITS 24	/* HV_SPA_PAGE_RANGE_ADDITIONAL_PAGES_BITS */
union hv_pfn_range {		/* HV_SPA_PAGE_RANGE */
	__u64 as_uint64;
	struct {
		/* 39:0: base pfn.  63:40: additional pages */
		__u64 base_pfn : 64 - HV_PFN_RNG_PAGEBITS;
		__u64 add_pfns : HV_PFN_RNG_PAGEBITS;
	} __packed;
};

union hv_snp_guest_policy {
	struct {
		__u64 minor_version : 8;
		__u64 major_version : 8;
		__u64 smt_allowed : 1;
		__u64 vmpls_required : 1;
		__u64 migration_agent_allowed : 1;
		__u64 debug_allowed : 1;
		__u64 reserved : 44;
	} __packed;
	__u64 as_uint64;
};

struct hv_snp_id_block {
	__u8 launch_digest[48];
	__u8 family_id[16];
	__u8 image_id[16];
	__u32 version;
	__u32 guest_svn;
	union hv_snp_guest_policy policy;
} __packed;

struct hv_snp_id_auth_info {
	__u32 id_key_algorithm;
	__u32 auth_key_algorithm;
	__u8 reserved0[56];
	__u8 id_block_signature[512];
	__u8 id_key[1028];
	__u8 reserved1[60];
	__u8 id_key_signature[512];
	__u8 author_key[1028];
} __packed;

struct hv_psp_launch_finish_data {
	struct hv_snp_id_block id_block;
	struct hv_snp_id_auth_info id_auth_info;
	__u8 host_data[32];
	__u8 id_block_enabled;
	__u8 author_key_enabled;
} __packed;

union hv_partition_complete_isolated_import_data {
	__u64 reserved;
	struct hv_psp_launch_finish_data psp_parameters;
} __packed;

struct hv_input_complete_isolated_import {
	__u64 partition_id;
	union hv_partition_complete_isolated_import_data import_data;
} __packed;

struct hv_partition_property_vmm_capabilities {
	__u16 bank_count;
	__u16 reserved[3];
	union {
		__u64 as_uint64[HV_PARTITION_VMM_CAPABILITIES_BANK_COUNT];
		struct {
			__u64 map_gpa_preserve_adjustable : 1;
			__u64 vmm_can_provide_overlay_gpfn : 1;
			__u64 vp_affinity_property : 1;
#if defined(__aarch64__)
			__u64 vmm_can_provide_gic_overlay_locations : 1;
#else
			__u64 reservedbit3 : 1;
#endif
			__u64 assignable_synthetic_proc_features : 1;
			__u64 reserved0 : HV_PARTITION_VMM_CAPABILITIES_RESERVED_BITFIELD_COUNT;
		} __packed;
	};
} __packed;

#endif /* _HVHDK_MINI_H */
