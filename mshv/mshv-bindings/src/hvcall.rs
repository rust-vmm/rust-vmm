// Copyright © 2024, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//

use crate::bindings::*;
use std::mem::size_of;
use std::vec::Vec;

/// This file contains helper functions for the MSHV_ROOT_HVCALL ioctl.
/// MSHV_ROOT_HVCALL is basically a 'passthrough' hypercall. The kernel makes a
/// hypercall on behalf of the VMM without interpreting the arguments or result
/// or changing any state in the kernel.
///
/// RepInput<T> wraps a buffer containing the input for a "rep"[1] hypercall.
/// Rep hypercalls have rep-eated data, i.e. a variable length array as part of
/// the input structure e.g.:
/// ```
/// use mshv_bindings::bindings::*;
/// #[repr(C, packed)]
/// struct hv_input_foo {
///    some_field: __u64,
///    variable_array_field: __IncompleteArrayField<__u64>,
/// }
/// ```
/// The struct cannot be used as-is because it can't store anything in the
/// __IncompleteArrayField<T> field.
///
/// RepInput<T> abstracts a rep hypercall input by wrapping a Vec<T>, where T
/// is the hv_input_* struct type. The buffer backing the Vec<T> has enough
/// space to store both the hv_input_* struct (at index 0), and the rep data
/// immediately following it.
///
/// Note also that the length of the variable length array field is not stored in
/// this struct. Rather, it is passed to the hypercall in the 'rep count' field
/// of the hypercall args (mshv_root_hvcall.reps). RepInput<T> stores this count,
/// along with the size of the entire input data.
///
/// RepInput<T> is intended to be created with make_rep_input!() and used with
/// make_rep_args!() below.
///
/// [1] HyperV TLFS describing the hypercall interface and rep hypercalls:
///   https://learn.microsoft.com/en-us/virtualization/hyper-v-on-windows/tlfs/hypercall-interface
///
pub struct RepInput<T> {
    vec: Vec<T>,
    size: usize,
    rep_count: usize,
}

impl<T: Default> RepInput<T> {
    /// Create a RepInput<T> for a rep hypercall
    ///
    /// # Arguments
    ///
    /// * `vec` - Vec<T> from input_with_arr_field_as_vec(). T is hv_input_* struct
    /// * `size` - Size of the hypercall input, including the rep data
    /// * `rep_count` - number of reps
    pub fn new(vec: Vec<T>, size: usize, rep_count: usize) -> Self {
        Self {
            vec,
            size,
            rep_count,
        }
    }
    pub fn as_mut_struct_ref(&mut self) -> &mut T {
        &mut self.vec[0]
    }
    pub fn as_struct_ptr(&self) -> *const T {
        &self.vec[0]
    }
    pub fn rep_count(&self) -> usize {
        self.rep_count
    }
    pub fn size(&self) -> usize {
        self.size
    }
    /// Make `Vec<T>` with at least enough space for `count` entries of
    /// `entry_size`, plus one additional entry
    /// Populate the first element of the Vec with the T. The rest will hold
    /// elements of size `entry_size` (note the Vec cannot be used normally to
    /// modify these since size_of::<T>() isn't necessarily the same as entry_size)
    pub fn input_with_arr_field_as_vec(t: T, entry_size: usize, count: usize) -> Vec<T> {
        let element_space = count * entry_size;
        let vec_size_bytes = size_of::<T>() + element_space;
        let rounded_size = vec_size_bytes.div_ceil(size_of::<T>());
        let mut v = Vec::with_capacity(rounded_size);
        v.resize_with(rounded_size, T::default);
        v[0] = t;
        v
    }
}

impl<U> __IncompleteArrayField<U> {
    /// Utility for casting __IncompleteArrayField<T> as the interior type
    pub fn as_entry_ptr_mut(ptr: *mut __IncompleteArrayField<U>) -> *mut U {
        ptr as *mut U
    }
    /// Utility for getting the size of the interior type
    /// Note we must use a raw pointer rather than a reference here, because the
    /// compiler thinks the field itself may be unaligned due to the struct being packed
    pub fn entry_size(_: *const __IncompleteArrayField<U>) -> usize {
        size_of::<U>()
    }
}

/// Assemble a RepInput<T> from a hypercall input struct and an array of rep data
/// Arguments:
///     1. The hv_input_* struct with the input data
///     2. Name of the __IncompleteArrayField<T> in the struct
///     3. An array or slice containing the rep data
#[macro_export]
macro_rules! make_rep_input {
    ($struct_expr:expr, $field_ident:ident, $arr_expr:expr) => {{
        let s = $struct_expr;
        let a = $arr_expr;
        let el_size = __IncompleteArrayField::entry_size(std::ptr::addr_of!(s.$field_ident));
        let struct_size = std::mem::size_of_val(&s);
        let mut vec = RepInput::input_with_arr_field_as_vec(s, el_size, a.len());
        let ptr =
            __IncompleteArrayField::as_entry_ptr_mut(std::ptr::addr_of_mut!(vec[0].$field_ident));
        for (i, el) in a.iter().enumerate() {
            // SAFETY: we know the vector is large enough to hold the data
            unsafe {
                let mut p = ptr.add(i);
                p.write_unaligned(*el);
            };
        }
        RepInput::new(vec, struct_size + el_size * a.len(), a.len())
    }};
}

/// Create a mshv_root_hvcall populated with rep hypercall parameters
/// Arguments:
///     1. hypercall code
///     2. RepInput<T> structure, where T is hv_input_*. See make_rep_input!()
///     3. Slice of the correct type for output data (optional)
#[macro_export]
macro_rules! make_rep_args {
    ($code_expr:expr, $input_ident:ident, $output_slice_ident:ident) => {{
        mshv_root_hvcall {
            code: $code_expr as u16,
            reps: $input_ident.rep_count() as u16,
            in_sz: $input_ident.size() as u16,
            out_sz: (std::mem::size_of_val(&$output_slice_ident[0]) * $output_slice_ident.len())
                as u16,
            in_ptr: $input_ident.as_struct_ptr() as u64,
            out_ptr: std::ptr::addr_of_mut!($output_slice_ident[0]) as u64,
            ..Default::default()
        }
    }};
    ($code_expr:expr, $input_ident:ident) => {{
        mshv_root_hvcall {
            code: $code_expr as u16,
            reps: $input_ident.rep_count() as u16,
            in_sz: $input_ident.size() as u16,
            in_ptr: $input_ident.as_struct_ptr() as u64,
            ..Default::default()
        }
    }};
}

/// Create a mshv_root_hvcall populated with hypercall parameters
/// Arguments:
///     1. hypercall code
///     2. hv_input_* structure
///     3. hv_output_* structure (optional)
#[macro_export]
macro_rules! make_args {
    ($code_expr:expr, $input_expr:expr, $output_expr:expr) => {{
        mshv_root_hvcall {
            code: $code_expr as u16,
            in_sz: std::mem::size_of_val(&($input_expr)) as u16,
            out_sz: std::mem::size_of_val(&($output_expr)) as u16,
            in_ptr: std::ptr::addr_of!($input_expr) as u64,
            out_ptr: std::ptr::addr_of_mut!($output_expr) as u64,
            ..Default::default()
        }
    }};
    ($code_expr:expr, $input_ident:ident) => {{
        mshv_root_hvcall {
            code: $code_expr as u16,
            in_sz: std::mem::size_of_val(&$input_ident) as u16,
            in_ptr: std::ptr::addr_of!($input_ident) as u64,
            ..Default::default()
        }
    }};
}
