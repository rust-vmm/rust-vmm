/* SPDX-License-Identifier: GPL-2.0 */
/*
 * Type definitions for the hypervisor guest interface.
 */
#ifndef _HVGDK_H
#define _HVGDK_H

#include "hvgdk_mini.h"

#define HVGDK_H_VERSION			(25125)

#if defined(__x86_64__)

enum hv_unimplemented_msr_action {
	HV_UNIMPLEMENTED_MSR_ACTION_FAULT = 0,
	HV_UNIMPLEMENTED_MSR_ACTION_IGNORE_WRITE_READ_ZERO = 1,
	HV_UNIMPLEMENTED_MSR_ACTION_COUNT = 2,
};

#endif

#endif /* _HVGDK_H */
