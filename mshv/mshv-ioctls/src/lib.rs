// Copyright © 2020, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![deny(missing_docs)]
#![allow(clippy::doc_lazy_continuation)]

//! A safe wrapper around the kernel's MSHV interface.
//!
//! This crate offers safe wrappers for:
//! - [system ioctls](struct.Mshv.html) using the `Mshv` structure
//! - [VM ioctls](struct.VmFd.html) using the `VmFd` structure
//! - [vCPU ioctls](struct.VcpuFd.html) using the `VcpuFd` structure
//! - [device ioctls](struct.DeviceFd.html) using the `DeviceFd` structure
//!
//! # Platform support
//!
//! - x86_64
//!
//! **NOTE:** The list of available ioctls is not extensive.
//!
//! # Example - Running a VM on x86_64
//!
//! In this example we are creating a Virtual Machine (VM) with one vCPU.
//! On the vCPU we are running machine specific code. This example is based on
//! the [LWN article](https://lwn.net/Articles/658511/) on using the MSHV API.
//!
//! To get code running on the vCPU we are going through the following steps:
//!
//! 1. Instantiate MSHV. This is used for running
//!    [system specific ioctls](struct.Mshv.html).
//! 2. Use the MSHV object to create a VM. The VM is used for running
//!    [VM specific ioctls](struct.VmFd.html).
//! 3. Initialize the guest memory for the created VM. In this dummy example we
//!    are adding only one memory region and write the code in one memory page.
//! 4. Create a vCPU using the VM object. The vCPU is used for running
//!    [vCPU specific ioctls](struct.VcpuFd.html).
//! 5. Setup architectural specific general purpose registers and special registers. For
//!    details about how and why these registers are set, please check the
//!    [LWN article](https://lwn.net/Articles/658511/) on which this example is
//!    built.
//! 6. Run the vCPU code in a loop and check the
//!    [exit reasons](enum.VcpuExit.html).
//!
//!
//! ```ignore
//! use crate::ioctls::system::Mshv;
//! use std::io::Write;
//! use libc::c_void;
//!
//! fn run_vm() {
//!     let mshv = Mshv::new().unwrap();
//!     let vm = mshv.create_vm().unwrap();
//!     let vcpu = vm.create_vcpu(0).unwrap();
//!     // This example is based on https://lwn.net/Articles/658511/
//!     #[rustfmt::skip]
//!     let code:[u8;11] = [
//!         0xba, 0xf8, 0x03,  /* mov $0x3f8, %dx */
//!         0x00, 0xd8,         /* add %bl, %al */
//!         0x04, b'0',         /* add $'0', %al */
//!         0xee,               /* out %al, (%dx) */
//!         /* send a 0 to indicate we're done */
//!         0xb0, b'\0',        /* mov $'\0', %al */
//!         0xee,               /* out %al, (%dx) */
//!     ];
//!
//!     let mem_size = 0x4000;
//!     // SAFETY: FFI call.
//!     let load_addr = unsafe {
//!         libc::mmap(
//!             std::ptr::null_mut(),
//!             mem_size,
//!             libc::PROT_READ | libc::PROT_WRITE,
//!             libc::MAP_ANONYMOUS | libc::MAP_SHARED | libc::MAP_NORESERVE,
//!             -1,
//!             0,
//!         )
//!     } as *mut u8;
//!     let mem_region = mshv_user_mem_region {
//!         flags: set_bits!(u8, MSHV_SET_MEM_BIT_WRITABLE, MSHV_SET_MEM_BIT_EXECUTABLE),
//!         guest_pfn: 0x1,
//!         size: 0x1000,
//!         userspace_addr: load_addr as u64,
//!         ..Default::default()
//!     };
//!
//!     vm.map_user_memory(mem_region).unwrap();
//!
//!     // SAFETY: load_addr is a valid pointer from mmap. Its length is mem_size.
//!     unsafe {
//!         // Get a mutable slice of `mem_size` from `load_addr`.
//!         let mut slice = slice::from_raw_parts_mut(load_addr, mem_size);
//!         slice.write_all(&code).unwrap();
//!     }
//!
//!     //Get CS Register
//!     let mut cs_reg = hv_register_assoc {
//!         name: hv_register_name::HV_X64_REGISTER_CS as u32,
//!         ..Default::default()
//!     };
//!     vcpu.get_reg(slice::from_mut(&mut cs_reg)).unwrap();
//!
//!     // SAFETY: access union fields
//!     unsafe {
//!         assert_ne!({ cs_reg.value.segment.base }, 0);
//!         assert_ne!({ cs_reg.value.segment.selector }, 0);
//!     };
//!
//!     cs_reg.value.segment.base = 0;
//!     cs_reg.value.segment.selector = 0;
//!
//!     vcpu.set_reg(&[
//!         cs_reg,
//!         hv_register_assoc {
//!             name: hv_register_name::HV_X64_REGISTER_RAX as u32,
//!             value: hv_register_value { reg64: 2 },
//!             ..Default::default()
//!         },
//!         hv_register_assoc {
//!             name: hv_register_name::HV_X64_REGISTER_RBX as u32,
//!             value: hv_register_value { reg64: 2 },
//!             ..Default::default()
//!         },
//!         hv_register_assoc {
//!             name: hv_register_name::HV_X64_REGISTER_RIP as u32,
//!             value: hv_register_value { reg64: 0x1000 },
//!             ..Default::default()
//!         },
//!         hv_register_assoc {
//!             name: hv_register_name::HV_X64_REGISTER_RFLAGS as u32,
//!             value: hv_register_value { reg64: 0x2 },
//!             ..Default::default()
//!         },
//!     ])
//!     .unwrap();
//!
//!     let hv_message: hv_message = Default::default();
//!     let mut done = false;
//!     loop {
//!         let ret_hv_message: hv_message = vcpu.run(hv_message).unwrap();
//!         match ret_hv_message.header.message_type {
//!             hv_message_type_HVMSG_X64_HALT => {
//!                 println!("VM Halted!");
//!                 break;
//!             }
//!             hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT => {
//!                 let io_message = ret_hv_message.to_ioport_info().unwrap();
//!
//!                 if !done {
//!                     assert!(io_message.rax == b'4' as u64);
//!                     assert!(io_message.port_number == 0x3f8);
//!                     // SAFETY: access union fields.
//!                     unsafe {
//!                         assert!(io_message.access_info.__bindgen_anon_1.string_op() == 0);
//!                         assert!(io_message.access_info.__bindgen_anon_1.access_size() == 1);
//!                     }
//!                     assert!(
//!                         io_message.header.intercept_access_type == /*HV_INTERCEPT_ACCESS_WRITE*/ 1_u8
//!                     );
//!                     done = true;
//!                     /* Advance rip */
//!                     vcpu.set_reg(&[hv_register_assoc {
//!                         name: hv_register_name::HV_X64_REGISTER_RIP as u32,
//!                         value: hv_register_value {
//!                             reg64: io_message.header.rip + 1,
//!                         },
//!                         ..Default::default()
//!                     }])
//!                     .unwrap();
//!                 } else {
//!                     assert!(io_message.rax == b'\0' as u64);
//!                     assert!(io_message.port_number == 0x3f8);
//!                     // SAFETY: access union fields.
//!                     unsafe {
//!                         assert!(io_message.access_info.__bindgen_anon_1.string_op() == 0);
//!                         assert!(io_message.access_info.__bindgen_anon_1.access_size() == 1);
//!                     }
//!                     assert!(
//!                         io_message.header.intercept_access_type == /*HV_INTERCEPT_ACCESS_WRITE*/ 1_u8
//!                     );
//!                     break;
//!                 }
//!             }
//!             _ => {
//!                 println!("Message type: 0x{:x?}", {
//!                     ret_hv_message.header.message_type
//!                 });
//!                 panic!("Unexpected Exit Type");
//!             }
//!         };
//!     }
//!     assert!(done);
//!     vm.unmap_user_memory(mem_region).unwrap();
//!     // SAFETY: FFI call. We're sure load_addr and mem_size are correct.
//!     unsafe { libc::munmap(load_addr as *mut c_void, mem_size) };
//! }
//! ```

#[macro_use]
mod ioctls;
pub use ioctls::device::DeviceFd;
pub use ioctls::system::Mshv;
pub use ioctls::vcpu::VcpuFd;
pub use ioctls::vm::InterruptRequest;
pub use ioctls::vm::IoEventAddress;
pub use ioctls::vm::NoDatamatch;
pub use ioctls::vm::VmFd;
pub use ioctls::vm::VmType;
pub use ioctls::MshvError;

#[macro_use]
mod mshv_ioctls;
pub use mshv_ioctls::*;

#[macro_use]
extern crate vmm_sys_util;
