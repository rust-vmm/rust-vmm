// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

use std::cmp::Ordering;

use crate::bus::{BusAddress, Error, MmioAddress, PioAddress};

/// An interval in the address space of a bus.
#[derive(Copy, Clone, Debug)]
pub struct BusRange<A: BusAddress> {
    base: A,
    size: A::V,
}

impl<A: BusAddress> BusRange<A> {
    /// Create a new range while checking for overflow.
    pub fn new(base: A, size: A::V) -> Result<Self, Error> {
        // A zero-length range is not valid.
        if size == 0.into() {
            return Err(Error::InvalidRange);
        }

        // Subtracting one, because a range that ends at the very edge of the address space
        // is still valid.
        base.checked_add(size - 1.into())
            .ok_or(Error::InvalidRange)?;

        Ok(BusRange { base, size })
    }

    /// Create a new unit range (its size equals `1`).
    pub fn unit(base: A) -> Self {
        BusRange {
            base,
            size: 1.into(),
        }
    }

    /// Return the base address of this range.
    pub fn base(&self) -> A {
        self.base
    }

    /// Return the size of the range.
    pub fn size(&self) -> A::V {
        self.size
    }

    /// Return the last bus address that's still part of the range.
    pub fn last(&self) -> A {
        self.base + (self.size - 1.into())
    }

    /// Check whether `self` and `other` overlap as intervals.
    pub fn overlaps(&self, other: &BusRange<A>) -> bool {
        !(self.base > other.last() || self.last() < other.base)
    }
}

// We need to implement the following traits so we can use `BusRange` values with `BTreeMap`s.
// This usage scenario requires treating ranges as if they supported a total order, but that's
// not really possible with intervals, so we write the implementations as if `BusRange`s were
// solely determined by their base addresses, and apply extra checks in the `Bus` logic.

impl<A: BusAddress> PartialEq for BusRange<A> {
    fn eq(&self, other: &BusRange<A>) -> bool {
        self.base == other.base
    }
}

impl<A: BusAddress> Eq for BusRange<A> {}

impl<A: BusAddress> PartialOrd for BusRange<A> {
    fn partial_cmp(&self, other: &BusRange<A>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A: BusAddress> Ord for BusRange<A> {
    fn cmp(&self, other: &BusRange<A>) -> Ordering {
        self.base.cmp(&other.base)
    }
}

/// Represents an MMIO bus range.
pub type MmioRange = BusRange<MmioAddress>;
/// Represents a PIO bus range.
pub type PioRange = BusRange<PioAddress>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_range() {
        let base_zero = MmioAddress(0);
        let value = 5;

        assert_eq!(BusRange::new(base_zero, 0), Err(Error::InvalidRange));

        assert!(BusRange::new(base_zero, u64::MAX).is_ok());
        assert!(BusRange::new(MmioAddress(1), u64::MAX).is_ok());
        assert_eq!(
            BusRange::new(MmioAddress(2), u64::MAX),
            Err(Error::InvalidRange)
        );

        {
            let range = BusRange::new(base_zero, value).unwrap();
            assert_eq!(range.base(), base_zero);
            assert_eq!(range.size(), value);
            assert_eq!(range.last(), MmioAddress(value - 1));
            assert!(range.base() < range.last());
        }

        {
            let range = BusRange::unit(base_zero);
            assert_eq!(range.base(), base_zero);
            assert_eq!(range.last(), range.base());
        }

        // Let's test `BusRange::overlaps`.
        {
            let range = BusRange::new(MmioAddress(10), 10).unwrap();

            let overlaps = |base_value, len_value| {
                range.overlaps(&BusRange::new(MmioAddress(base_value), len_value).unwrap())
            };

            assert!(!overlaps(0, 5));
            assert!(!overlaps(0, 10));
            assert!(!overlaps(5, 5));

            assert!(overlaps(0, 11));
            assert!(overlaps(5, 6));
            assert!(overlaps(5, 10));
            assert!(overlaps(11, 15));
            assert!(overlaps(5, 35));
            assert!(overlaps(19, 1));
            assert!(overlaps(19, 10));

            assert!(!overlaps(20, 1));
            assert!(!overlaps(30, 10));
        }

        // Finally, let's test the `BusRange` trait implementations that we added.
        {
            let base = MmioAddress(10);
            let len = 10;

            let range = BusRange::new(base, len).unwrap();

            assert_eq!(range.cmp(&range), range.partial_cmp(&range).unwrap());
            assert_eq!(range.cmp(&range), Ordering::Equal);

            {
                let other = BusRange::new(base, len + 1).unwrap();

                // Still equal becase we're only comparing `base` values as part
                // of the `eq` implementation.
                assert_eq!(range, other);

                assert_eq!(range.cmp(&other), range.partial_cmp(&other).unwrap());
                assert_eq!(range.cmp(&other), Ordering::Equal);
            }

            {
                let other = BusRange::unit(base.checked_add(1).unwrap());

                // Different due to different base addresses.
                assert_ne!(range, other);

                assert_eq!(range.cmp(&other), range.partial_cmp(&other).unwrap());
                assert_eq!(range.cmp(&other), Ordering::Less);
            }
        }
    }
}
