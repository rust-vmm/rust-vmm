# Changelog

## Upcoming Release

### Removed

- Remove versionize support

## v0.4.1

### Changed

- Widen vmm-sys-util dependency to 0.12.0-0.14.0

## v0.4.0

### Changed

- Updated vmm-sys-util dependency to 0.12.1
- Updated versionize dependency to 0.2.0

## v0.3.0

### Changed

- Updated vmm-sys-util dependency to 0.11.0
- Switched to specifying dependencies using caret requirements
  instead of comparision requirements

## v0.2.0

### Added

- Added support for a `(De)Serialize` and `Versionize` serial state object,
  `SerialStateSer`([#73](https://github.com/rust-vmm/vm-superio/pull/73)).

## v0.1.0

This is the first `vm-superio-ser` release.
The `vm-superio-ser` crate provides support for persisting the states from
`vm-superio`. For now, it provides this support only for the `Rtc` device.
`RtcStateSer` can be used by customers who need an `RtcState` that is also
`(De)Serialize` and/or `Versionize`.
This version of `RtcStateSer` is compatible with v0.5.0 version of `RtcState`.
