# vm-device

The `vm-device` crate provides:
* device traits defining read and write operations on specialized buses
* device manager (bus-specific traits and a concrete implementation) for
operating devices and dispatching I/O
* abstractions for defining resources and their constraints (e.g. a specific bus
address range, IRQ number, etc)

## Design

The virtual device model is built around four traits, `DevicePio` and
`MutDevicePio` for
[Programmed I/O](https://en.wikipedia.org/wiki/Programmed_input%E2%80%93output)
(PIO), and `DeviceMmio` and `MutDeviceMmio` for
[Memory-mapped I/O](https://en.wikipedia.org/wiki/Memory-mapped_I/O)
(MMIO). The traits define the same methods for handling read and write
operations. The difference is that `DevicePio` and `DeviceMmio` only require
immutable self borrows, whereas `MutDevicePio` and `MutDeviceMmio` require
mutable borrows.

The device manager abstraction is implemented by the `IoManager` struct. It
defines two buses, one for PIO and one for MMIO. For each bus, with the help of
the `PioManager` and `MmioManager` traits, the manager provides methods for
device registration, as well as for dispatching read and write requests.
The manager will determine which device is responsible for handling the I/O
request based on the accessed address range, and will route the request to that
device.

`PioManager` and `MmioManager` traits are useful as interfaces for a couple of
reasons. First, to allow for alternative implementations when the provided
`IoManager` is not sufficient. Second, to allow other crates depend on the
traits without depending on the actual implementation.

The interaction between a guest kernel, a host kernel and a VMM using
`IoManager` is depicted at the diagram below. A driver in the guest kernel
issues an I/O request. That request gets turned by the host kernel’s hypervisor
(KVM) into a trigger for the VMM to handle. The trigger is either a vCPU exit or
an eventfd notification. The VMM extracts address information and determines the
target bus. Then it dispatches a new request to the `IoManager`, which checks
that there’s a virtual device registered on the bus for the requested address,
and finally sends the request to that device.
![vm-device](https://user-images.githubusercontent.com/241037/143853115-b1526028-6836-4845-a311-71cf989c60ef.png)

## Usage

A device is usually attached to a particular bus and thus needs to implement a
trait of only one type. For example, serial port on x86 is a PIO device, while
VirtIO devices use MMIO. It’s also possible for a device to implement both. Once
the type of I/O is determined, the next step is to choose between mutable and
immutable trait variant. If read or write method needs to mutate the device’s
internal state, then the mutable variant must be used.

Before dispatching any I/O requests to the new device, it needs to be registered
with an `IoManager` instance within the specified address range on the bus.
Creating a new `IoManager` is easy by calling `IoManager::new()` without any
configuration. Internally the manager stores devices as trait objects wrapped
in `Arc`’s, therefore if the device implements `MutDevicePio` or
`MutDeviceMmio`, then it must be wrapped in a `Mutex`. The crate contains
automatic implementation of `DevicePio for Mutex<T> where T: MutDevicePio`
and `DeviceMmio for Mutex<T> where T: MutDeviceMmio` but only for the Mutex
type in the standard library. For any other `Mutex` type from 3rd party crates
the blanket implementation must be done by the user.

From now on the IoManager will be routing I/O requests for the registered
address range to the device. The requests are dispatched by the client code, for
example when handling VM exits, using `IoManager`'s methods `pio_read`,
`pio_write`, `mmio_read` and `mmio_write`.

## Examples

### Implementing a simple log PIO device

```rust
use std::sync::{Arc, Mutex};
use vm_device::bus::{PioAddress, PioAddressOffset, PioRange};
use vm_device::device_manager::{IoManager, PioManager};
use vm_device::MutDevicePio;

struct LogDevice {}

impl MutDevicePio for LogDevice {
    fn pio_read(&mut self, base: PioAddress, offset: PioAddressOffset, _data: &mut [u8]) {
        println!("mut pio_read: base {:?}, offset {}", base, offset);
    }

    fn pio_write(&mut self, base: PioAddress, offset: PioAddressOffset, data: &[u8]) {
        println!(
            "mut pio_write: base {:?}, offset {}, data {:?}",
            base, offset, data
        );
    }
}
```

### Registering the device with IoManager and performing I/O

```rust
fn main() {
    let mut manager = IoManager::new();
    let device = LogDevice {};
    let bus_range = PioRange::new(PioAddress(0), 10).unwrap();
    manager
        .register_pio(bus_range, Arc::new(Mutex::new(device)))
        .unwrap();
    manager.pio_write(PioAddress(0), &vec![b'o', b'k']).unwrap();
}
```

## Testing

The `vm-device` is tested using unit tests and integration tests.
It leverages [`rust-vmm-ci`](https://github.com/rust-vmm/rust-vmm-ci)
for continuous testing. All tests are ran in the `rustvmm/dev` container.

## License

This project is licensed under either of:

- [Apache License](http://www.apache.org/licenses/LICENSE-2.0), Version 2.0
- [BSD-3-Clause License](https://opensource.org/licenses/BSD-3-Clause)
