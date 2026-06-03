/* SPDX-License-Identifier: Apache-2.0 */

/*
 * Pre-included into bindgen, see CONTRIBUTING.md.
 *
 * bindgen turns __DECLARE_FLEX_ARRAY()'s portable C expansion into a nested
 * __bindgen_ty_* struct, while vmm-sys-util's generate_fam_struct_impl!
 * needs the array as a direct field. Use the C++ expansion here rather than
 * defining __cplusplus, which would flip every other C++ conditional in
 * these headers.
 *
 * The include must stay first. It claims the guard, which makes linux/kvm.h's
 * later inclusion a no-op and keeps the redefinition below in effect. Without
 * it, the header's own #define wins.
 */
#include <linux/stddef.h>

#undef __DECLARE_FLEX_ARRAY
#define __DECLARE_FLEX_ARRAY(T, member) T member[0]
