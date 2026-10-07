# Changelog

## Upcoming Release

### Added

- [[#175]](https://github.com/rust-vmm/rust-vmm/pull/175) The x86_64
  `KVM_GET_ONE_REG` / `KVM_SET_ONE_REG` register ids from Linux 6.18:
  `KVM_X86_REG_TYPE_MSR`, `KVM_X86_REG_TYPE_KVM`, `KVM_REG_GUEST_SSP` and the
  `kvm_x86_reg_msr` / `kvm_x86_reg_kvm` id builders.

### Changed

- [[381]](https://github.com/rust-vmm/kvm/pull/381)
  Raised kvm_irq_routing cap to KVM_MAX_IRQ_ROUTES (4096) matching the
  kernel
- Regenerated the riscv64 `KVM_RISCV_ISA_EXT_ID` enum from Linux
  v7.3-rc6 UAPI headers, adding `ZICBOP`, `ZFBFMIN`, `ZVFBFMIN`,
  `ZVFBFWMA`, `ZCLSD`, `ZILSD`, `ZALASR`, `ZICFILP` and `ZICFISS`
  (IDs 71-79) and bumping `..._MAX` from 71 to 80.
- Added the missing `zicbop_block_size` trailing field to the riscv64
  `kvm_riscv_config` struct, matching the kernel's
  `arch/riscv/include/uapi/asm/kvm.h`.

## v0.14.1

### Fixed

- [[#350]](https://github.com/rust-vmm/kvm/pull/350) Fixed a feature
  inconsistency where the `serde` implementations for `kvm_xsave2` were emitted
  without the `fam-wrappers` feature, breaking builds that enabled `serde`
  alone. These impls now require the `fam-wrappers` feature.

## v0.14.0

### Changed

- Rust edition 2024

## v0.13.0

### Added

- [[#322]](https://github.com/rust-vmm/kvm/pull/322)
  Foundations for `kvm-ioctls`'s GET_NESTED_STATE and SET_NESTED_STATE

### Changed

- Regenerate bindings from Linux kernel v6.15.

### Removed

## v0.12.0

### Added

- [[323]](https://github.com/rust-vmm/kvm/pull/323) Upgrade vmm-sys-util to v0.14.0
- Added FamStructWrapper for `kvm_irq_routing` type.
- Added serde support for `kvm_irq_routing` and `kvm_irq_routing_entry` types.

## v0.11.1

### Added

- Implemented `Serialize` / `Deserialize` for `kvm_xsave2`.

## v0.11.0

### Changed

- Updated KVM_MAX_CPUID_ENTRIES to 256.

## v0.10.0

### Added

- RISC-V KVM bindings for Linux kernel v6.9, including serialization support.

## v0.9.1

### Changed

- Fixed and validated manual (De)Serialize implementations to work with
  `serde_json` crate.

## v0.9.0

### Changed

- Replaced the v6.2 bindings of arm64, x86\_64 with the v6.9 ones.

### Removed

- Removed v6.2 bindings.

## v0.8.2

### Changed

- Improve performance of bindings deserialization by \~5% by avoiding
  a temporary allocation.

## v0.8.1

### Fixed

- Implement `Default` for `kvm_xsave2`, which fixes usage of `Xsave`
  unconditionally causing compile errors in downstream crates.

## v0.8.0

### Added

- An opt-in feature `serde` that enables [`serde`](https://serde.rs)-based
  (de)serialization of various bindings.

## v0.7.0

### Changed
- API change in the bindings from upstream kernel changes:
  * system\_event has been made into a new union
- The x86 module has been renamed to x86\_64 for consistency (matches the kernel
  architecture directory name)
- Added all features to the generated docs.rs documentation.

### Removed

- Dropped "x86" (32-bit) x86 support

## v0.6.0

### Changed

- Updated vmm-sys-utils dependency to 0.11.0
- Switched to specifying dependencies using caret requirements
  instead of comparision requirements

### Added

- Implement `PartialEq` for fam\_wrappers

## v0.5.0

### Changed

- Replaced the v4.20 bindings with the v5.13 ones.

### Removed

- Removed v4.14 bindings.

## v0.4.0

- vmm-sys-utils dependency bumped to match kvm-ioctls.

## v0.3.0

### Added

- Enabled `fam-wrappers` support on arm and arm64.
- Added fam-wrapper for the arm specific `kvm_reg_list` struct.

## v0.2.0

### Added

- Added opt-in feature `fam-wrappers` that enables exporting
  safe wrappers over generated structs with flexible array
  members. This optional feature has an external dependency
  on `vmm-sys-util`.
- Added safe fam-wrappers for `kvm_msr_list`, `kvm_msrs`,
  and `kvm_cpuid2`.

## v0.1.1

### Changed

- Do not enforce rust Edition 2018.

## v0.1.0

### Added

- KVM bindings for Linux kernel version 4.14 and 4.20 with
  support for arm, arm64, x86 and x86_64.
