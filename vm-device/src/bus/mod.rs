// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

//! Provides abstractions for modelling an I/O bus.
//!
//! A bus is seen here as a mapping between
//! disjoint intervals (ranges) from an address space and objects (devices) associated with them.
//! A single device can be registered with multiple ranges, but no two ranges can overlap,
//! regardless with their device associations.

mod address;
mod range;

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::result::Result;

use address::BusAddress;

pub use address::{MmioAddress, MmioAddressOffset, PioAddress, PioAddressOffset};
pub use range::{BusRange, MmioRange, PioRange};

/// Errors encountered during bus operations.
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    /// No device is associated with the specified address or range.
    DeviceNotFound,
    /// Specified range overlaps an already registered range.
    DeviceOverlap,
    /// Access with invalid length attempted.
    InvalidAccessLength(usize),
    /// Invalid range provided (either zero-sized, or last address overflows).
    InvalidRange,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DeviceNotFound => write!(f, "device not found"),
            Error::DeviceOverlap => write!(f, "range overlaps with existing device"),
            Error::InvalidAccessLength(len) => write!(f, "invalid access length ({})", len),
            Error::InvalidRange => write!(f, "invalid range provided"),
        }
    }
}

impl std::error::Error for Error {}

/// A bus that's agnostic to the range address type and device type.
pub struct Bus<A: BusAddress, D> {
    devices: BTreeMap<BusRange<A>, D>,
}

impl<A: BusAddress, D> Default for Bus<A, D> {
    fn default() -> Self {
        Bus {
            devices: BTreeMap::new(),
        }
    }
}

impl<A: BusAddress, D> Bus<A, D> {
    /// Create an empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the registered range and device associated with `addr`.
    pub fn device(&self, addr: A) -> Option<(&BusRange<A>, &D)> {
        // The range is returned as an optimization because the caller
        // might need both the device and its associated bus range.
        // The same goes for the device_mut() method.
        self.devices
            .range(..=BusRange::unit(addr))
            .nth_back(0)
            .filter(|pair| pair.0.last() >= addr)
    }

    /// Return the registered range and a mutable reference to the device
    /// associated with `addr`.
    pub fn device_mut(&mut self, addr: A) -> Option<(&BusRange<A>, &mut D)> {
        self.devices
            .range_mut(..=BusRange::unit(addr))
            .nth_back(0)
            .filter(|pair| pair.0.last() >= addr)
    }

    /// Register a device with the provided range.
    pub fn register(&mut self, range: BusRange<A>, device: D) -> Result<(), Error> {
        for r in self.devices.keys() {
            if range.overlaps(r) {
                return Err(Error::DeviceOverlap);
            }
        }

        self.devices.insert(range, device);

        Ok(())
    }

    /// Deregister the device associated with `addr`.
    pub fn deregister(&mut self, addr: A) -> Option<(BusRange<A>, D)> {
        let range = self.device(addr).map(|(range, _)| *range)?;
        self.devices.remove(&range).map(|device| (range, device))
    }

    /// Verify whether an access starting at `addr` with length `len` fits within any of
    /// the registered ranges. Return the range and a handle to the device when present.
    pub fn check_access(&self, addr: A, len: usize) -> Result<(&BusRange<A>, &D), Error> {
        let access_range = BusRange::new(
            addr,
            A::V::try_from(len).map_err(|_| Error::InvalidAccessLength(len))?,
        )
        .map_err(|_| Error::InvalidRange)?;
        self.device(addr)
            .filter(|(range, _)| range.last() >= access_range.last())
            .ok_or(Error::DeviceNotFound)
    }
}

/// Represents an MMIO bus.
pub type MmioBus<D> = Bus<MmioAddress, D>;
/// Represents a PIO bus.
pub type PioBus<D> = Bus<PioAddress, D>;

/// Helper trait that can be implemented by types which hold one or more buses.
pub trait BusManager<A: BusAddress> {
    /// Type of the objects held by the bus.
    type D;

    /// Return a reference to the bus.
    fn bus(&self) -> &Bus<A, Self::D>;

    /// Return a mutable reference to the bus.
    fn bus_mut(&mut self) -> &mut Bus<A, Self::D>;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bus() {
        let base = MmioAddress(10);
        let base_prev = MmioAddress(base.value().checked_sub(1).unwrap());
        let len = 10;
        let range = MmioRange::new(base, len).unwrap();
        let range_next = range.last().checked_add(1).unwrap();

        let mut bus = Bus::new();
        // The bus is agnostic to actual device types, so let's just use a numeric type here.
        let device = 1u8;

        assert_eq!(bus.devices.len(), 0);

        bus.register(range, device).unwrap();
        assert_eq!(bus.devices.len(), 1);

        assert!(bus.device(base_prev).is_none());
        assert!(bus.device_mut(base_prev).is_none());
        assert!(bus.device(range_next).is_none());
        assert!(bus.device_mut(range_next).is_none());

        for offset in 0..len {
            let addr = base.checked_add(offset).unwrap();

            {
                let (r, d) = bus.device(addr).unwrap();
                assert_eq!(range, *r);
                assert_eq!(device, *d);
            }

            {
                let (r, d) = bus.device_mut(addr).unwrap();
                assert_eq!(range, *r);
                assert_eq!(device, *d);
            }

            // Let's also check invocations of `Bus::check_access`.
            for start_offset in 0..offset {
                let start_addr = base.checked_add(start_offset).unwrap();

                let (r, d) = bus
                    .check_access(start_addr, usize::try_from(offset - start_offset).unwrap())
                    .unwrap();
                assert_eq!(range, *r);
                assert_eq!(device, *d);
            }
        }

        // Detect double registration with the same range.
        assert_eq!(bus.register(range, device), Err(Error::DeviceOverlap));

        // We detect overlaps even if it's another range associated with the same device (we don't
        // implicitly merge ranges). `check_access` fails if the specified range does not fully
        // fit within a region associated with a particular device.

        {
            let range2 = MmioRange::new(MmioAddress(1), 10).unwrap();
            assert_eq!(bus.register(range2, device), Err(Error::DeviceOverlap));
            assert_eq!(
                bus.check_access(range2.base(), usize::try_from(range2.size()).unwrap()),
                Err(Error::DeviceNotFound)
            );
        }

        {
            let range2 = MmioRange::new(range.last(), 10).unwrap();
            assert_eq!(bus.register(range2, device), Err(Error::DeviceOverlap));
            assert_eq!(
                bus.check_access(range2.base(), usize::try_from(range2.size()).unwrap()),
                Err(Error::DeviceNotFound)
            );
        }

        {
            let range2 = MmioRange::new(MmioAddress(1), range.last().value() + 100).unwrap();
            assert_eq!(bus.register(range2, device), Err(Error::DeviceOverlap));
            assert_eq!(
                bus.check_access(range2.base(), usize::try_from(range2.size()).unwrap()),
                Err(Error::DeviceNotFound)
            );
        }

        {
            // For a completely empty range, `check_access` should still fail, but `insert`
            // will succeed.

            let range2 = MmioRange::new(range.last().checked_add(1).unwrap(), 5).unwrap();

            assert_eq!(
                bus.check_access(range2.base(), usize::try_from(range2.size()).unwrap()),
                Err(Error::DeviceNotFound)
            );

            // Validate registration, and that `deregister` works for all addresses within a range.
            for offset in 0..range2.size() {
                let device2 = device + 1;
                assert!(bus.register(range2, device2).is_ok());
                assert_eq!(bus.devices.len(), 2);

                let addr = range2.base().checked_add(offset).unwrap();
                let (r, d) = bus.deregister(addr).unwrap();
                assert_eq!(bus.devices.len(), 1);
                assert_eq!(r, range2);
                assert_eq!(d, device2);

                // A second deregister should fail.
                assert!(bus.deregister(addr).is_none());
                assert_eq!(bus.devices.len(), 1);
            }

            // Register the previous `device` for `range2`.
            assert!(bus.register(range2, device).is_ok());
            assert_eq!(bus.devices.len(), 2);

            // Even though the new range is associated with the same device, and right after the
            // previous one, accesses across multiple ranges are not allowed for now.
            // TODO: Do we want to support this in the future?
            assert_eq!(
                bus.check_access(range.base(), usize::try_from(range.size() + 1).unwrap()),
                Err(Error::DeviceNotFound)
            );
        }

        // Ensure that bus::check_access() fails when the len argument
        // cannot be safely converted to PioAddressOffset which is u16.
        let pio_base = PioAddress(10);
        let pio_len = 10;
        let pio_range = PioRange::new(pio_base, pio_len).unwrap();
        let mut pio_bus = Bus::new();
        let pio_device = 1u8;
        pio_bus.register(pio_range, pio_device).unwrap();
        assert_eq!(
            pio_bus.check_access(pio_base, usize::MAX),
            Err(Error::InvalidAccessLength(usize::MAX))
        );
    }
}
