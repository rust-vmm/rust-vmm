// Copyright © 2019 Intel Corporation. All Rights Reserved.
// SPDX-License-Identifier: (Apache-2.0 OR BSD-3-Clause)

//! System level device management.
//!
//! [`IoManager`] is responsible for managing
//! all devices of virtual machine, registering IO resources callback,
//! deregistering devices and helping VM IO exit handling.
//! It defines two buses, one for PIO and one for MMIO, and provides default
//! implementations of [`PioManager`] and [`MmioManager`].
//!
//! The VMM must first allocate unique resources (such as bus ranges), and then
//! call into the vm-device interface to register the devices with their
//! corresponding resources.
//!
//! # Examples
//!
//! Registering a new device can be done using the register methods of [`PioManager`]
//! and [`MmioManager`] with an appropriate bus range
//! ([`PioRange`](../bus/type.PioRange.html) or [`MmioRange`](../bus/type.MmioRange.html)).
//! ```
//! # use std::sync::Arc;
//! # use vm_device::bus::{PioAddress, PioAddressOffset, PioRange};
//! # use vm_device::bus::{MmioAddress, MmioAddressOffset, MmioRange};
//! # use vm_device::device_manager::{IoManager, PioManager, MmioManager};
//! # use vm_device::{DevicePio, DeviceMmio};
//! struct NoopDevice {}
//!
//! impl DevicePio for NoopDevice {
//!     fn pio_read(&self, base: PioAddress, offset: PioAddressOffset, data: &mut [u8]) {}
//!     fn pio_write(&self, base: PioAddress, offset: PioAddressOffset, data: &[u8]) {}
//! }
//!
//! impl DeviceMmio for NoopDevice {
//!     fn mmio_read(&self, base: MmioAddress, offset: MmioAddressOffset, data: &mut [u8]) {}
//!     fn mmio_write(&self, base: MmioAddress, offset: MmioAddressOffset, data: &[u8]) {}
//! }
//!
//! // IoManager implements both PioManager and MmioManager.
//! let mut manager = IoManager::new();
//!
//! // Register the device on the PIO bus.
//! let pio_range = PioRange::new(PioAddress(0), 10).unwrap();
//! manager
//!     .register_pio(pio_range, Arc::new(NoopDevice {}))
//!     .unwrap();
//!
//! // Register the device on the MMIO bus.
//! let mmio_range = MmioRange::new(MmioAddress(0), 10).unwrap();
//! manager
//!     .register_mmio(mmio_range, Arc::new(NoopDevice {}))
//!     .unwrap();
//!
//! // Dispatch I/O on the PIO bus.
//! manager.pio_write(PioAddress(0), &vec![b'o', b'k']).unwrap();
//!
//! // Dispatch I/O on the MMIO bus.
//! manager
//!     .mmio_write(MmioAddress(0), &vec![b'o', b'k'])
//!     .unwrap();
//! ```
//!
//! An alternative way would be to use [`resources`](../resources/index.html) and the
//! resources registration methods of [`IoManager`]:
//! * [`register_pio_resources`](struct.IoManager.html#method.register_pio_resources)
//! * [`register_mmio_resources`](struct.IoManager.html#method.register_mmio_resources)
//! * or generic [`register_resources`](struct.IoManager.html#method.register_resources)
//! ```
//! # use std::sync::Arc;
//! # use vm_device::bus::{PioAddress, PioAddressOffset, PioRange};
//! # use vm_device::bus::{MmioAddress, MmioAddressOffset, MmioRange};
//! # use vm_device::device_manager::{IoManager, PioManager, MmioManager};
//! # use vm_device::{DevicePio, DeviceMmio};
//! # use vm_device::resources::Resource;
//! # struct NoopDevice {}
//! #
//! # impl DevicePio for NoopDevice {
//! #    fn pio_read(&self, base: PioAddress, offset: PioAddressOffset, data: &mut [u8]) {}
//! #    fn pio_write(&self, base: PioAddress, offset: PioAddressOffset, data: &[u8]) {}
//! # }
//! #
//! # impl DeviceMmio for NoopDevice {
//! #    fn mmio_read(&self, base: MmioAddress, offset: MmioAddressOffset, data: &mut [u8]) {}
//! #    fn mmio_write(&self, base: MmioAddress, offset: MmioAddressOffset, data: &[u8]) {}
//! # }
//! // Use the same NoopDevice defined above.
//!
//! let mut manager = IoManager::new();
//!
//! // Define a PIO address range resource.
//! let pio = Resource::PioAddressRange {
//!    base: 0,
//!    size: 10,
//! };
//!
//! // Define a MMIO address range resource.
//! let mmio = Resource::MmioAddressRange {
//!    base: 0,
//!    size: 10,
//! };
//!
//! // Register the PIO resource.
//! manager
//!     .register_pio_resources(Arc::new(NoopDevice {}), &vec![pio])
//!     .unwrap();
//!
//! // Register the MMIO resource.
//! manager
//!     .register_mmio_resources(Arc::new(NoopDevice {}), &vec![mmio])
//!     .unwrap();
//!
//! // Dispatching I/O is the same.
//! manager.pio_write(PioAddress(0), &vec![b'o', b'k']).unwrap();
//! manager.mmio_write(MmioAddress(0), &vec![b'o', b'k']).unwrap();
//! ```

use std::fmt::{Display, Formatter};
use std::result::Result;
use std::sync::Arc;

use crate::bus::{self, BusManager, MmioAddress, MmioBus, MmioRange, PioAddress, PioBus, PioRange};
use crate::resources::Resource;
use crate::{DeviceMmio, DevicePio};

/// Error type for [IoManager] usage.
#[derive(Debug)]
pub enum Error {
    /// Error during bus operation.
    Bus(bus::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Bus(_) => write!(f, "device_manager: bus error"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Bus(e) => Some(e),
        }
    }
}

/// Represents an object that provides PIO manager operations.
pub trait PioManager {
    /// Type of the objects that can be registered with this `PioManager`.
    type D: DevicePio;

    /// Return a reference to the device registered at `addr`, together with the associated
    /// range, if available.
    fn pio_device(&self, addr: PioAddress) -> Option<(&PioRange, &Self::D)>;

    /// Dispatch a read operation to the device registered at `addr`.
    fn pio_read(&self, addr: PioAddress, data: &mut [u8]) -> Result<(), bus::Error>;

    /// Dispatch a write operation to the device registered at `addr`.
    fn pio_write(&self, addr: PioAddress, data: &[u8]) -> Result<(), bus::Error>;

    /// Register the provided device with the specified range.
    fn register_pio(&mut self, range: PioRange, device: Self::D) -> Result<(), bus::Error>;

    /// Deregister the device currently registered at `addr` together with the
    /// associated range.
    fn deregister_pio(&mut self, addr: PioAddress) -> Option<(PioRange, Self::D)>;
}

// This automatically provides a `PioManager` implementation for types that already implement
// `BusManager<PioAddress>` if their inner associated type implements `DevicePio` as well.
impl<T> PioManager for T
where
    T: BusManager<PioAddress>,
    T::D: DevicePio,
{
    type D = <Self as BusManager<PioAddress>>::D;

    fn pio_device(&self, addr: PioAddress) -> Option<(&PioRange, &Self::D)> {
        self.bus().device(addr)
    }

    fn pio_read(&self, addr: PioAddress, data: &mut [u8]) -> Result<(), bus::Error> {
        self.bus()
            .check_access(addr, data.len())
            .map(|(range, device)| device.pio_read(range.base(), addr - range.base(), data))
    }

    fn pio_write(&self, addr: PioAddress, data: &[u8]) -> Result<(), bus::Error> {
        self.bus()
            .check_access(addr, data.len())
            .map(|(range, device)| device.pio_write(range.base(), addr - range.base(), data))
    }

    fn register_pio(&mut self, range: PioRange, device: Self::D) -> Result<(), bus::Error> {
        self.bus_mut().register(range, device)
    }

    fn deregister_pio(&mut self, addr: PioAddress) -> Option<(PioRange, Self::D)> {
        self.bus_mut().deregister(addr)
    }
}

/// Represents an object that provides MMIO manager operations.
pub trait MmioManager {
    /// Type of the objects that can be registered with this `MmioManager`.
    type D: DeviceMmio;

    /// Return a reference to the device registered at `addr`, together with the associated
    /// range, if available.
    fn mmio_device(&self, addr: MmioAddress) -> Option<(&MmioRange, &Self::D)>;

    /// Dispatch a read operation to the device registered at `addr`.
    fn mmio_read(&self, addr: MmioAddress, data: &mut [u8]) -> Result<(), bus::Error>;

    /// Dispatch a write operation to the device registered at `addr`.
    fn mmio_write(&self, addr: MmioAddress, data: &[u8]) -> Result<(), bus::Error>;

    /// Register the provided device with the specified range.
    fn register_mmio(&mut self, range: MmioRange, device: Self::D) -> Result<(), bus::Error>;

    /// Deregister the device currently registered at `addr` together with the
    /// associated range.
    fn deregister_mmio(&mut self, addr: MmioAddress) -> Option<(MmioRange, Self::D)>;
}

// This automatically provides a `MmioManager` implementation for types that already implement
// `BusManager<MmioAddress>` if their inner associated type implements `DeviceMmio` as well.
impl<T> MmioManager for T
where
    T: BusManager<MmioAddress>,
    T::D: DeviceMmio,
{
    type D = <Self as BusManager<MmioAddress>>::D;

    fn mmio_device(&self, addr: MmioAddress) -> Option<(&MmioRange, &Self::D)> {
        self.bus().device(addr)
    }

    fn mmio_read(&self, addr: MmioAddress, data: &mut [u8]) -> Result<(), bus::Error> {
        self.bus()
            .check_access(addr, data.len())
            .map(|(range, device)| device.mmio_read(range.base(), addr - range.base(), data))
    }

    fn mmio_write(&self, addr: MmioAddress, data: &[u8]) -> Result<(), bus::Error> {
        self.bus()
            .check_access(addr, data.len())
            .map(|(range, device)| device.mmio_write(range.base(), addr - range.base(), data))
    }

    fn register_mmio(&mut self, range: MmioRange, device: Self::D) -> Result<(), bus::Error> {
        self.bus_mut().register(range, device)
    }

    fn deregister_mmio(&mut self, addr: MmioAddress) -> Option<(MmioRange, Self::D)> {
        self.bus_mut().deregister(addr)
    }
}

/// System IO manager serving for all devices management and VM exit handling.
#[derive(Default)]
pub struct IoManager {
    // Range mapping for VM exit pio operations.
    pio_bus: PioBus<Arc<dyn DevicePio + Send + Sync>>,
    // Range mapping for VM exit mmio operations.
    mmio_bus: MmioBus<Arc<dyn DeviceMmio + Send + Sync>>,
}

// Enables the automatic implementation of `PioManager` for `IoManager`.
impl BusManager<PioAddress> for IoManager {
    type D = Arc<dyn DevicePio + Send + Sync>;

    fn bus(&self) -> &PioBus<Arc<dyn DevicePio + Send + Sync>> {
        &self.pio_bus
    }

    fn bus_mut(&mut self) -> &mut PioBus<Arc<dyn DevicePio + Send + Sync>> {
        &mut self.pio_bus
    }
}

// Enables the automatic implementation of `MmioManager` for `IoManager`.
impl BusManager<MmioAddress> for IoManager {
    type D = Arc<dyn DeviceMmio + Send + Sync>;

    fn bus(&self) -> &MmioBus<Arc<dyn DeviceMmio + Send + Sync>> {
        &self.mmio_bus
    }

    fn bus_mut(&mut self) -> &mut MmioBus<Arc<dyn DeviceMmio + Send + Sync>> {
        &mut self.mmio_bus
    }
}

impl IoManager {
    /// Create an default IoManager with empty IO member.
    pub fn new() -> Self {
        IoManager::default()
    }

    /// Register a new MMIO device with its allocated resources.
    /// VMM is responsible for providing the allocated resources to virtual device.
    ///
    /// # Arguments
    ///
    /// * `device`: device instance object to be registered
    /// * `resources`: resources that this device owns, might include
    ///   port I/O and memory-mapped I/O ranges, irq number, etc.
    pub fn register_mmio_resources(
        &mut self,
        device: Arc<dyn DeviceMmio + Send + Sync>,
        resources: &[Resource],
    ) -> Result<(), Error> {
        // Register and mark device resources
        // The resources addresses being registered are sucessfully allocated before.
        for res in resources.iter() {
            match *res {
                Resource::MmioAddressRange { base, size } => {
                    self.register_mmio(
                        MmioRange::new(MmioAddress(base), size).unwrap(),
                        device.clone(),
                    )
                    .map_err(Error::Bus)?;
                }
                _ => continue,
            }
        }
        Ok(())
    }

    /// Register a new PIO device with its allocated resources.
    /// VMM is responsible for providing the allocated resources to virtual device.
    ///
    /// # Arguments
    ///
    /// * `device`: device instance object to be registered
    /// * `resources`: resources that this device owns, might include
    ///   port I/O and memory-mapped I/O ranges, irq number, etc.
    pub fn register_pio_resources(
        &mut self,
        device: Arc<dyn DevicePio + Send + Sync>,
        resources: &[Resource],
    ) -> Result<(), Error> {
        // Register and mark device resources
        // The resources addresses being registered are sucessfully allocated before.
        for res in resources.iter() {
            match *res {
                Resource::PioAddressRange { base, size } => {
                    self.register_pio(
                        PioRange::new(PioAddress(base), size).unwrap(),
                        device.clone(),
                    )
                    .map_err(Error::Bus)?;
                }
                _ => continue,
            }
        }
        Ok(())
    }

    /// Register a new MMIO + PIO device with its allocated resources.
    /// VMM is responsible for providing the allocated resources to virtual device.
    ///
    /// # Arguments
    ///
    /// * `device`: device instance object to be registered
    /// * `resources`: resources that this device owns, might include
    ///   port I/O and memory-mapped I/O ranges, irq number, etc.
    pub fn register_resources<T: DeviceMmio + DevicePio + 'static + Send + Sync>(
        &mut self,
        device: Arc<T>,
        resources: &[Resource],
    ) -> Result<(), Error> {
        self.register_mmio_resources(device.clone(), resources)?;
        self.register_pio_resources(device, resources)
    }

    /// Deregister a device from `IoManager`, e.g. users specified removing.
    /// VMM pre-fetches the resources e.g. dev.get_assigned_resources()
    /// VMM is responsible for freeing the resources. Returns the number
    /// of deregistered devices.
    ///
    /// # Arguments
    ///
    /// * `resources`: resources that this device owns, might include
    ///   port I/O and memory-mapped I/O ranges, irq number, etc.
    pub fn deregister_resources(&mut self, resources: &[Resource]) -> usize {
        let mut count = 0;
        for res in resources.iter() {
            match *res {
                Resource::PioAddressRange { base, .. } => {
                    if self.deregister_pio(PioAddress(base)).is_some() {
                        count += 1;
                    }
                }
                Resource::MmioAddressRange { base, .. } => {
                    if self.deregister_mmio(MmioAddress(base)).is_some() {
                        count += 1;
                    }
                }
                _ => continue,
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::error::Error;
    use std::sync::Mutex;

    use bus::{MmioAddressOffset, PioAddressOffset};

    const PIO_ADDRESS_SIZE: u16 = 4;
    const PIO_ADDRESS_BASE: u16 = 0x40;
    const MMIO_ADDRESS_SIZE: u64 = 0x8765_4321;
    const MMIO_ADDRESS_BASE: u64 = 0x1234_5678;
    const LEGACY_IRQ: u32 = 4;
    const CONFIG_DATA: u32 = 0x1234;

    struct DummyDevice {
        config: Mutex<u32>,
    }

    impl DummyDevice {
        fn new(config: u32) -> Self {
            DummyDevice {
                config: Mutex::new(config),
            }
        }
    }

    impl DevicePio for DummyDevice {
        fn pio_read(&self, _base: PioAddress, _offset: PioAddressOffset, data: &mut [u8]) {
            if data.len() > 4 {
                return;
            }
            for (idx, iter) in data.iter_mut().enumerate() {
                let config = self.config.lock().expect("failed to acquire lock");
                *iter = (*config >> (idx * 8) & 0xff) as u8;
            }
        }

        fn pio_write(&self, _base: PioAddress, _offset: PioAddressOffset, data: &[u8]) {
            let mut config = self.config.lock().expect("failed to acquire lock");
            *config = u32::from(data[0]) & 0xff;
        }
    }

    impl DeviceMmio for DummyDevice {
        fn mmio_read(&self, _base: MmioAddress, _offset: MmioAddressOffset, data: &mut [u8]) {
            if data.len() > 4 {
                return;
            }
            for (idx, iter) in data.iter_mut().enumerate() {
                let config = self.config.lock().expect("failed to acquire lock");
                *iter = (*config >> (idx * 8) & 0xff) as u8;
            }
        }

        fn mmio_write(&self, _base: MmioAddress, _offset: MmioAddressOffset, data: &[u8]) {
            let mut config = self.config.lock().expect("failed to acquire lock");
            *config = u32::from(data[0]) & 0xff;
        }
    }

    #[test]
    fn test_register_deregister_device_io() {
        let mut io_mgr = IoManager::new();
        let dummy = DummyDevice::new(0);
        let dum = Arc::new(dummy);

        let mut resource: Vec<Resource> = Vec::new();
        let mmio = Resource::MmioAddressRange {
            base: MMIO_ADDRESS_BASE,
            size: MMIO_ADDRESS_SIZE,
        };
        let irq = Resource::LegacyIrq(LEGACY_IRQ);
        let pio = Resource::PioAddressRange {
            base: PIO_ADDRESS_BASE,
            size: PIO_ADDRESS_SIZE,
        };

        resource.push(mmio);
        resource.push(irq);
        resource.push(pio);

        assert!(io_mgr
            .register_mmio_resources(dum.clone(), &resource)
            .is_ok());
        assert!(io_mgr.register_pio_resources(dum, &resource).is_ok());
        assert_eq!(io_mgr.deregister_resources(&resource), 2);
    }

    #[test]
    fn test_mmio_read_write() {
        let mut io_mgr: IoManager = Default::default();
        let dum = Arc::new(DummyDevice::new(CONFIG_DATA));
        let mut resource: Vec<Resource> = Vec::new();

        let mmio = Resource::MmioAddressRange {
            base: MMIO_ADDRESS_BASE,
            size: MMIO_ADDRESS_SIZE,
        };
        resource.push(mmio);
        assert!(io_mgr
            .register_mmio_resources(dum.clone(), &resource)
            .is_ok());

        let mut data = [0; 4];
        assert!(io_mgr
            .mmio_read(MmioAddress(MMIO_ADDRESS_BASE), &mut data)
            .is_ok());
        assert_eq!(data, [0x34, 0x12, 0, 0]);

        assert!(io_mgr
            .mmio_read(
                MmioAddress(MMIO_ADDRESS_BASE + MMIO_ADDRESS_SIZE),
                &mut data
            )
            .is_err());

        data = [0; 4];
        assert!(io_mgr
            .mmio_write(MmioAddress(MMIO_ADDRESS_BASE), &data)
            .is_ok());
        assert_eq!(*dum.config.lock().unwrap(), 0);

        assert!(io_mgr
            .mmio_write(MmioAddress(MMIO_ADDRESS_BASE + MMIO_ADDRESS_SIZE), &data)
            .is_err());
    }

    #[test]
    fn test_pio_read_write() {
        let mut io_mgr: IoManager = Default::default();
        let dum = Arc::new(DummyDevice::new(CONFIG_DATA));
        let mut resource: Vec<Resource> = Vec::new();

        let pio = Resource::PioAddressRange {
            base: PIO_ADDRESS_BASE,
            size: PIO_ADDRESS_SIZE,
        };
        resource.push(pio);
        assert!(io_mgr
            .register_pio_resources(dum.clone(), &resource)
            .is_ok());

        let mut data = [0; 4];
        assert!(io_mgr
            .pio_read(PioAddress(PIO_ADDRESS_BASE), &mut data)
            .is_ok());
        assert_eq!(data, [0x34, 0x12, 0, 0]);

        assert!(io_mgr
            .pio_read(PioAddress(PIO_ADDRESS_BASE + PIO_ADDRESS_SIZE), &mut data)
            .is_err());

        data = [0; 4];
        assert!(io_mgr
            .pio_write(PioAddress(PIO_ADDRESS_BASE), &data)
            .is_ok());
        assert_eq!(*dum.config.lock().unwrap(), 0);

        assert!(io_mgr
            .pio_write(PioAddress(PIO_ADDRESS_BASE + PIO_ADDRESS_SIZE), &data)
            .is_err());
    }

    #[test]
    fn test_error_code() {
        let err = super::Error::Bus(bus::Error::DeviceOverlap);

        assert!(err.source().is_some());
        assert_eq!(format!("{}", err), "device_manager: bus error");
    }
}
