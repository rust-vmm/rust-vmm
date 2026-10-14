# Changelog
## [Unreleased]

### Added

### Changed

### Deprecated

### Fixed

## [v0.7.1]

###
* No changes

## [v0.7.0]

### Added
* [[329]](https://github.com/rust-vmm/mshv/pull/329) mshv-bindings: add assignable partition property
* [[329]](https://github.com/rust-vmm/mshv/pull/329) mshv-bindings: refresh synthetic feature bitmaps and add arm64 bindings
* [[331]](https://github.com/rust-vmm/mshv/pull/331) mshv-bindings: regenerate bindings for refreshed partition processor feature unions

### Changed
* [[326]](https://github.com/rust-vmm/mshv/pull/326) mshv-bindings: use alloc_zeroed for Buffer::new()

## [v0.6.9]

###
* No changes

## [v0.6.8]

###
* No changes

## [v0.6.7]

### Changed
* [[301]](https://github.com/rust-vmm/mshv/pull/301) Generate bindings to pick up SMT_ENABLED_GUEST flag

## [v0.6.6]

### Changed
* [[292]](https://github.com/rust-vmm/mshv/pull/292) Add more CPU features supported by the Microsoft Hypervisor
* [[298]](https://github.com/rust-vmm/mshv/pull/298) mshv-bindings: fix mshv_create_partition_v2 bindings for arm64
* [[299]](https://github.com/rust-vmm/mshv/pull/299) mshv: Modify intermediate control registers to support nested hypervisor

## [v0.6.5]

###
* No changes

## [v0.6.4]

###
* No changes

## [v0.6.3]

### Changed
* [[272]] (https://github.com/rust-vmm/mshv/pull/272) mshv-bindings: generate new bindings

## [v0.6.2]

###
* No changes

## [v0.6.1]

###
* No changes

## [v0.6.0]

### Added
* [[243]](https://github.com/rust-vmm/mshv/pull/243) Add bindings for arm64 reset intercepts
* [[244]](https://github.com/rust-vmm/mshv/pull/244) add function to query vmm capabilities

### Fixed
* [[241]](https://github.com/rust-vmm/mshv/pull/241) Fixes for arm64 guests

## [v0.5.2]

###
* No changes

## [v0.5.1]

### Added
* [[218 ]] (https://github.com/rust-vmm/mshv/pull/218) save and restore interrupt vectors

## [v0.5.0]

### Changed
* [[216]] (https://github.com/rust-vmm/mshv/pull/216) mshv-bindings: Regenerate ARM bindings using latest bindgen-cli

## [v0.4.0]

### Added
* [[207]] (https://github.com/rust-vmm/mshv/pull/207) hvcall ioctls
* [[210]] (https://github.com/rust-vmm/mshv/pull/210) mshv-bindings: Add new constants for partition property

### Changed
* [[209]] (https://github.com/rust-vmm/mshv/pull/209) filter MSRs before returing to VMM
* [[208]] https://github.com/rust-vmm/mshv/pull/208) property value is 64 bits

## [v0.3.5]

### Added
* [[196]](https://github.com/rust-vmm/mshv/pull/196) mshv-bindings: Add support for unmarshaling memory intercept
* [[198]](https://github.com/rust-vmm/mshv/pull/198) Add more xsave related registers

### Changed
* [[195]](https://github.com/rust-vmm/mshv/pull/195) mshv-bindings: Update bindings for ARM64 guest
* [[203]](https://github.com/rust-vmm/mshv/pull/203) Add new struct and generate new bindings

## [v0.3.4]

## [v0.3.3]

### Added
* [[184]](https://github.com/rust-vmm/mshv/pull/184) mshv-ioctl: Add an interface to retrieve host partition property
* [[186]](https://github.com/rust-vmm/mshv/pull/186) Add support for StandardRegisters on ARM64
* [[187]](https://github.com/rust-vmm/mshv/pull/187) Validate Cargo.toml and commit messages
* [[188]](https://github.com/rust-vmm/mshv/pull/188) Add new interfaces required for ARM64 guest.
* [[189]](https://github.com/rust-vmm/mshv/pull/189) Add @praveen-pk as the codeowner

### Changed
* [[183]](https://github.com/rust-vmm/mshv/pull/183) Update get/set Standard Registers to use VP register page

### Deprecated

### Fixed
* [[185]](https://github.com/rust-vmm/mshv/pull/185) Make tests conditionally compile for x86
* [[181]](https://github.com/rust-vmm/mshv/pull/181) mshv-bindings: Fix rust clippy warnings

# v0.3.2

### Added
- [[178]](https://github.com/rust-vmm/mshv/pull/178) mshv-bindings: implement send for svm_ghcb_base

### Changed

### Deprecated

### Fixed

- [[179]](https://github.com/rust-vmm/mshv/pull/179) Fix UB with Tree Borrows aliasing model in test suite
- [[177]](https://github.com/rust-vmm/mshv/pull/177) build(deps): update thiserror requirement from 1.0 to 2.0

# v0.3.1

## Added

- [[#161]](https://github.com/rust-vmm/mshv/pull/161) Add definitions to handle GHCB page

### Changed

- [[#175]](https://github.com/rust-vmm/mshv/pull/175) Cargo.toml: Update metadata description to comply with cargo publish standards
- [[#169]](https://github.com/rust-vmm/mshv/pull/169) Cargo.toml: Update Cargo toml with metadata
- [[#168]](https://github.com/rust-vmm/mshv/pull/168) Bump zerocopy crate to the lastest version
- [[#157]](https://github.com/rust-vmm/mshv/pull/157) Update to Rust 2021 edition, and fix a warning.
# v0.1.0

First release of the mshv-bindings crate.
