// Copyright © 2024 Institute of Software, CAS. All rights reserved.
// SPDX-License-Identifier: BSD-3-Clause

//! Miscellaneous macros related to align addresses

/// Aligns the given address downwards to the nearest boundary specified by
/// `alignment`.
///
/// This macro ensures that the result is the largest address less than or equal
/// to `addr` that is aligned to `alignment`.
///
/// # Arguments
///
/// * `$addr` - The address or value to align downwards.
/// * `$alignment` - The alignment boundary, which must be a power of two.
///
/// # Panics
///
/// The macro will panic if `$alignment` is not a power of two.
///
/// # Example
///
/// ```rust
/// use vmm_sys_util::align_downwards;
///
/// let addr: usize = 10;
/// let alignment: usize = 4; // Must be a power of two
/// let aligned = align_downwards!(addr, alignment);
/// assert_eq!(aligned, 8);
/// ```
#[macro_export]
macro_rules! align_downwards {
    ($addr:expr, $alignment:expr) => {{
        assert!(
            $alignment.is_power_of_two(),
            "alignment must be a power of two and non-zero"
        );
        $addr & !($alignment - 1)
    }};
}

/// Aligns the given address upwards to the nearest boundary specified by `alignment`.
///
/// This macro ensures that the result is the smallest address greater than or equal to `addr`
/// that is aligned to `alignment`.
///
/// # Arguments
///
/// * `$addr` - The address or value to align upwards.
/// * `$alignment` - The alignment boundary, which must be a power of two.
///
/// # Panics
///
/// The macro will panic if `$alignment` is not a power of two.
///
/// # Example
///
/// ```rust
/// use vmm_sys_util::align_upwards;
///
/// let addr: usize = 10;
/// let alignment: usize = 4; // Must be a power of two
/// let aligned = align_upwards!(addr, alignment);
/// assert_eq!(aligned, 12);
/// ```
#[macro_export]
macro_rules! align_upwards {
    ($addr:expr, $alignment:expr) => {{
        assert!(
            $alignment.is_power_of_two(),
            "alignment must be a power of two and non-zero"
        );
        $addr
            .checked_add($alignment - 1)
            .expect("address overflow when aligning address upwards")
            & !($alignment - 1)
    }};
}
