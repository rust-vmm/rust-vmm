// Copyright © 2020, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//
#![allow(missing_docs)]

use mshv_bindings::*;

// /dev/mshv fd
ioctl_iow_nr!(
    MSHV_CREATE_PARTITION,
    MSHV_IOCTL,
    0x00,
    mshv_create_partition
);
ioctl_iowr_nr!(
    MSHV_GET_HOST_PARTITION_PROPERTY,
    MSHV_IOCTL,
    0x01,
    mshv_partition_property
);

// partition fd
ioctl_io_nr!(MSHV_INITIALIZE_PARTITION, MSHV_IOCTL, 0x00);
ioctl_iow_nr!(MSHV_CREATE_VP, MSHV_IOCTL, 0x01, mshv_create_vp);
ioctl_iow_nr!(
    MSHV_SET_GUEST_MEMORY,
    MSHV_IOCTL,
    0x02,
    mshv_user_mem_region
);
ioctl_iow_nr!(MSHV_IRQFD, MSHV_IOCTL, 0x03, mshv_user_irqfd);
ioctl_iow_nr!(MSHV_IOEVENTFD, MSHV_IOCTL, 0x4, mshv_user_ioeventfd);
ioctl_iow_nr!(MSHV_SET_MSI_ROUTING, MSHV_IOCTL, 0x05, mshv_user_irq_table);
ioctl_iowr_nr!(
    MSHV_GET_GPAP_ACCESS_BITMAP,
    MSHV_IOCTL,
    0x06,
    mshv_gpap_access_bitmap
);
ioctl_iowr_nr!(MSHV_ROOT_HVCALL, MSHV_IOCTL, 0x07, mshv_root_hvcall);
ioctl_iowr_nr!(MSHV_CREATE_DEVICE, MSHV_IOCTL, 0x08, mshv_create_device);
ioctl_iow_nr!(
    MSHV_MODIFY_GPA_HOST_ACCESS,
    MSHV_IOCTL,
    0x09,
    mshv_modify_gpa_host_access
);
ioctl_iow_nr!(
    MSHV_IMPORT_ISOLATED_PAGES,
    MSHV_IOCTL,
    0x0A,
    mshv_import_isolated_pages
);

// deprecated
ioctl_iow_nr!(
    MSHV_INSTALL_INTERCEPT,
    MSHV_IOCTL,
    0xF0,
    mshv_install_intercept
);
ioctl_iow_nr!(
    MSHV_ASSERT_INTERRUPT,
    MSHV_IOCTL,
    0xF1,
    mshv_assert_interrupt
);
ioctl_iow_nr!(
    MSHV_SET_PARTITION_PROPERTY,
    MSHV_IOCTL,
    0xF2,
    mshv_partition_property
);
ioctl_iowr_nr!(
    MSHV_GET_PARTITION_PROPERTY,
    MSHV_IOCTL,
    0xF3,
    mshv_partition_property
);
ioctl_iow_nr!(
    MSHV_COMPLETE_ISOLATED_IMPORT,
    MSHV_IOCTL,
    0xF4,
    mshv_complete_isolated_import
);
ioctl_iow_nr!(
    MSHV_ISSUE_PSP_GUEST_REQUEST,
    MSHV_IOCTL,
    0xF5,
    mshv_issue_psp_guest_request
);
ioctl_iow_nr!(
    MSHV_SEV_SNP_AP_CREATE,
    MSHV_IOCTL,
    0xF6,
    mshv_sev_snp_ap_create
);
ioctl_iowr_nr!(
    MSHV_SIGNAL_EVENT_DIRECT,
    MSHV_IOCTL,
    0xF7,
    mshv_signal_event_direct
);
ioctl_iow_nr!(
    MSHV_POST_MESSAGE_DIRECT,
    MSHV_IOCTL,
    0xF8,
    mshv_post_message_direct
);
ioctl_iow_nr!(
    MSHV_REGISTER_DELIVERABILITY_NOTIFICATIONS,
    MSHV_IOCTL,
    0xF9,
    mshv_register_deliverabilty_notifications
);

// VP fd
ioctl_ior_nr!(MSHV_RUN_VP, MSHV_IOCTL, 0x00, mshv_run_vp);
#[cfg(target_arch = "x86_64")]
ioctl_iowr_nr!(MSHV_GET_VP_STATE, MSHV_IOCTL, 0x01, mshv_get_set_vp_state);
#[cfg(target_arch = "x86_64")]
ioctl_iowr_nr!(MSHV_SET_VP_STATE, MSHV_IOCTL, 0x02, mshv_get_set_vp_state);
// NOTE: defined above, also used with VP fd:
// ioctl_iowr_nr!(MSHV_ROOT_HVCALL, MSHV_IOCTL, 0x07, mshv_root_hvcall);

// deprecated
ioctl_iowr_nr!(MSHV_GET_VP_REGISTERS, MSHV_IOCTL, 0xF0, mshv_vp_registers);
ioctl_iow_nr!(MSHV_SET_VP_REGISTERS, MSHV_IOCTL, 0xF1, mshv_vp_registers);
ioctl_iowr_nr!(MSHV_VP_TRANSLATE_GVA, MSHV_IOCTL, 0xF2, mshv_translate_gva);
#[cfg(target_arch = "x86_64")]
ioctl_iow_nr!(
    MSHV_VP_REGISTER_INTERCEPT_RESULT,
    MSHV_IOCTL,
    0xF3,
    mshv_register_intercept_result
);
ioctl_iowr_nr!(
    MSHV_GET_VP_CPUID_VALUES,
    MSHV_IOCTL,
    0xF4,
    mshv_get_vp_cpuid_values
);
ioctl_iowr_nr!(MSHV_READ_GPA, MSHV_IOCTL, 0xF5, mshv_read_write_gpa);
ioctl_iow_nr!(MSHV_WRITE_GPA, MSHV_IOCTL, 0xF6, mshv_read_write_gpa);

// device fd
ioctl_iow_nr!(MSHV_SET_DEVICE_ATTR, MSHV_IOCTL, 0x00, mshv_device_attr);
ioctl_iow_nr!(MSHV_GET_DEVICE_ATTR, MSHV_IOCTL, 0x01, mshv_device_attr);
ioctl_iow_nr!(MSHV_HAS_DEVICE_ATTR, MSHV_IOCTL, 0x02, mshv_device_attr);
