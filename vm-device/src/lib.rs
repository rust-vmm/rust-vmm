// Copyright © 2019 Intel Corporation. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

#![deny(missing_docs)]

//! This crate provides:
//! * device traits defining read and write operations on specialized buses
//! * device manager (bus-specific traits and a concrete implementation) for
//!   operating devices and dispatching I/O
//! * abstractions for defining resources and their constraints (e.g. a specific bus
//!   address range, IRQ number, etc)
//!
//! [`MutDevicePio`] and [`MutDeviceMmio`] traits help with composite inner mutability
//! (i.e. if we have a `Mutex` that holds a `T` which implements [`MutDevicePio`],
//! then the `Mutex` can implement [`DevicePio`] based on its inner
//! mutability properties).
//!
//! # Example
//!
//! Implement a simple log PIO device, register it with
//! [`IoManager`](device_manager/struct.IoManager.html)
//! and dispatch a write operation to the device.
//!```
//! use std::sync::{Arc, Mutex};
//! use vm_device::bus::{PioAddress, PioAddressOffset, PioRange};
//! use vm_device::device_manager::{IoManager, PioManager};
//! use vm_device::MutDevicePio;
//!
//! struct LogDevice {}
//!
//! impl MutDevicePio for LogDevice {
//!     fn pio_read(&mut self, base: PioAddress, offset: PioAddressOffset, _data: &mut [u8]) {
//!         println!("mut pio_read: base {:?}, offset {}", base, offset);
//!     }
//!     fn pio_write(&mut self, base: PioAddress, offset: PioAddressOffset, data: &[u8]) {
//!         println!(
//!             "mut pio_write: base {:?}, offset {}, data {:?}",
//!             base, offset, data
//!         );
//!     }
//! }
//!
//! // IoManager implements PioManager trait.
//! let mut manager = IoManager::new();
//! let device = LogDevice {};
//! let bus_range = PioRange::new(PioAddress(0), 10).unwrap();
//! manager
//!     .register_pio(bus_range, Arc::new(Mutex::new(device)))
//!     .unwrap();
//! manager.pio_write(PioAddress(0), &vec![b'o', b'k']).unwrap();
//! ```

pub mod bus;
pub mod device_manager;
pub mod resources;

use std::ops::Deref;
use std::sync::{Arc, Mutex};

use bus::{MmioAddress, MmioAddressOffset, PioAddress, PioAddressOffset};

/// Allows a device to be attached to a
/// [PIO](https://en.wikipedia.org/wiki/Programmed_input%E2%80%93output) bus.
///
/// # Example
/// ```
/// # use std::sync::Mutex;
/// # use vm_device::{DevicePio, bus::{PioAddress, PioAddressOffset}};
/// struct DummyDevice {
///     config: Mutex<u32>,
/// }
///
/// impl DevicePio for DummyDevice {
///     fn pio_read(&self, _base: PioAddress, _offset: PioAddressOffset, data: &mut [u8]) {
///         if data.len() > 4 {
///             return;
///         }
///         for (idx, iter) in data.iter_mut().enumerate() {
///             let config = self.config.lock().expect("failed to acquire lock");
///             *iter = (*config >> (idx * 8) & 0xff) as u8;
///         }
///     }
///
///     fn pio_write(&self, _base: PioAddress, _offset: PioAddressOffset, data: &[u8]) {
///         let mut config = self.config.lock().expect("failed to acquire lock");
///         *config = u32::from(data[0]) & 0xff;
///     }
/// }
/// ```
pub trait DevicePio {
    /// Handle a read operation on the device.
    ///
    /// # Arguments
    ///
    /// * `base`:   base address on a PIO bus
    /// * `offset`: base address' offset
    /// * `data`:   a buffer provided by the caller to store the read data
    fn pio_read(&self, base: PioAddress, offset: PioAddressOffset, data: &mut [u8]);

    /// Handle a write operation to the device.
    ///
    /// # Arguments
    ///
    /// * `base`:   base address on a PIO bus
    /// * `offset`: base address' offset
    /// * `data`:   a buffer provided by the caller holding the data to write
    fn pio_write(&self, base: PioAddress, offset: PioAddressOffset, data: &[u8]);
}

/// Allows a device to be attached to a
/// [MMIO](https://en.wikipedia.org/wiki/Memory-mapped_I/O) bus.
///
/// # Example
/// ```
/// # use std::sync::Mutex;
/// # use vm_device::{DeviceMmio, bus::{MmioAddress, MmioAddressOffset}};
/// struct DummyDevice {
///     config: Mutex<u32>,
/// }
///
/// impl DeviceMmio for DummyDevice {
///     fn mmio_read(&self, _base: MmioAddress, _offset: MmioAddressOffset, data: &mut [u8]) {
///         if data.len() > 4 {
///             return;
///         }
///         for (idx, iter) in data.iter_mut().enumerate() {
///             let config = self.config.lock().expect("failed to acquire lock");
///             *iter = (*config >> (idx * 8) & 0xff) as u8;
///         }
///     }
///
///     fn mmio_write(&self, _base: MmioAddress, _offset: MmioAddressOffset, data: &[u8]) {
///         let mut config = self.config.lock().expect("failed to acquire lock");
///         *config = u32::from(data[0]) & 0xff;
///     }
/// }
/// ```
pub trait DeviceMmio {
    /// Handle a read operation on the device.
    ///
    /// # Arguments
    ///
    /// * `base`:   base address on a MMIO bus
    /// * `offset`: base address' offset
    /// * `data`:   a buffer provided by the caller to store the read data
    fn mmio_read(&self, base: MmioAddress, offset: MmioAddressOffset, data: &mut [u8]);

    /// Handle a write operation to the device.
    ///
    /// # Arguments
    ///
    /// * `base`:   base address on a MMIO bus
    /// * `offset`: base address' offset
    /// * `data`:   a buffer provided by the caller holding the data to write
    fn mmio_write(&self, base: MmioAddress, offset: MmioAddressOffset, data: &[u8]);
}

/// Same as [DevicePio] but the methods are invoked with a mutable self borrow.
///
/// # Example
/// ```
/// # use vm_device::{MutDevicePio, bus::{PioAddress, PioAddressOffset}};
/// struct DummyDevice {
///     config: u32,
/// }
///
/// impl MutDevicePio for DummyDevice {
///     fn pio_read(&mut self, _base: PioAddress, _offset: PioAddressOffset, data: &mut [u8]) {
///         if data.len() > 4 {
///             return;
///         }
///         for (idx, iter) in data.iter_mut().enumerate() {
///             *iter = (self.config >> (idx * 8) & 0xff) as u8;
///         }
///     }
///
///     fn pio_write(&mut self, _base: PioAddress, _offset: PioAddressOffset, data: &[u8]) {
///         self.config = u32::from(data[0]) & 0xff;
///     }
/// }
/// ```
pub trait MutDevicePio {
    /// Handle a read operation on the device.
    ///
    /// # Arguments
    ///
    /// * `base`:   base address on a PIO bus
    /// * `offset`: base address' offset
    /// * `data`:   a buffer provided by the caller to store the read data
    fn pio_read(&mut self, base: PioAddress, offset: PioAddressOffset, data: &mut [u8]);

    /// Handle a write operation to the device.
    ///
    /// # Arguments
    ///
    /// * `base`:   base address on a PIO bus
    /// * `offset`: base address' offset
    /// * `data`:   a buffer provided by the caller holding the data to write
    fn pio_write(&mut self, base: PioAddress, offset: PioAddressOffset, data: &[u8]);
}

/// Same as [DeviceMmio] but the methods are invoked with a mutable self borrow.
/// # Example
/// ```
/// # use vm_device::{MutDeviceMmio, bus::{MmioAddress, MmioAddressOffset}};
/// struct DummyDevice {
///     config: u32,
/// }
///
/// impl MutDeviceMmio for DummyDevice {
///     fn mmio_read(&mut self, _base: MmioAddress, _offset: MmioAddressOffset, data: &mut [u8]) {
///         if data.len() > 4 {
///             return;
///         }
///         for (idx, iter) in data.iter_mut().enumerate() {
///             *iter = (self.config >> (idx * 8) & 0xff) as u8;
///         }
///     }
///
///     fn mmio_write(&mut self, _base: MmioAddress, _offset: MmioAddressOffset, data: &[u8]) {
///         self.config = u32::from(data[0]) & 0xff;
///     }
/// }
/// ```
pub trait MutDeviceMmio {
    /// Handle a read operation on the device.
    ///
    /// # Arguments
    ///
    /// * `base`:   base address on a MMIO bus
    /// * `offset`: base address' offset
    /// * `data`:   a buffer provided by the caller to store the read data
    fn mmio_read(&mut self, base: MmioAddress, offset: MmioAddressOffset, data: &mut [u8]);

    /// Handle a write operation to the device.
    ///
    /// # Arguments
    ///
    /// * `base`:   base address on a MMIO bus
    /// * `offset`: base address' offset
    /// * `data`:   a buffer provided by the caller holding the data to write
    fn mmio_write(&mut self, base: MmioAddress, offset: MmioAddressOffset, data: &[u8]);
}

// Blanket implementations for Arc<T>.

impl<T: DeviceMmio + ?Sized> DeviceMmio for Arc<T> {
    fn mmio_read(&self, base: MmioAddress, offset: MmioAddressOffset, data: &mut [u8]) {
        self.deref().mmio_read(base, offset, data);
    }

    fn mmio_write(&self, base: MmioAddress, offset: MmioAddressOffset, data: &[u8]) {
        self.deref().mmio_write(base, offset, data);
    }
}

impl<T: DevicePio + ?Sized> DevicePio for Arc<T> {
    fn pio_read(&self, base: PioAddress, offset: PioAddressOffset, data: &mut [u8]) {
        self.deref().pio_read(base, offset, data);
    }

    fn pio_write(&self, base: PioAddress, offset: PioAddressOffset, data: &[u8]) {
        self.deref().pio_write(base, offset, data);
    }
}

// Blanket implementations for Mutex<T>.

impl<T: MutDeviceMmio + ?Sized> DeviceMmio for Mutex<T> {
    fn mmio_read(&self, base: MmioAddress, offset: MmioAddressOffset, data: &mut [u8]) {
        self.lock().unwrap().mmio_read(base, offset, data)
    }

    fn mmio_write(&self, base: MmioAddress, offset: MmioAddressOffset, data: &[u8]) {
        self.lock().unwrap().mmio_write(base, offset, data)
    }
}

impl<T: MutDevicePio + ?Sized> DevicePio for Mutex<T> {
    fn pio_read(&self, base: PioAddress, offset: PioAddressOffset, data: &mut [u8]) {
        self.lock().unwrap().pio_read(base, offset, data)
    }

    fn pio_write(&self, base: PioAddress, offset: PioAddressOffset, data: &[u8]) {
        self.lock().unwrap().pio_write(base, offset, data)
    }
}
