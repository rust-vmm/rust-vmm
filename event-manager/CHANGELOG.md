# Changelog

## Upcoming Version

### Changed
### Added
### Fixed
### Removed

## v0.4.2

### Changed

 - Updated vmm-sys-util version to 0.15.0

## v0.4.1

### Changed

 - Updated vmm-sys-util version to 0.14.0

## v0.4.0

### Changed

- Added `remote_endpoint` feature to generated docs.rs documentation.
- Derived the `Debug` trait for multiple types

## v0.3.0

### Changed

- Updated to Rust version 2021.
- Dependencies are now specified by caret, so that they're no longer
  automatically updated when new ones are published.
- The `Display` implementations for the `EventFd` and `Epoll` variants of the
  `Error` type now contain the inner error message as well.

### Added

- The `Error` type now implements `Eq`.

## v0.2.1

### Changed

- Updated the vmm-sys-util dependency to v0.8.0.

### Fixed

- Fixed `RemoteEndpoint` `Clone` implementation.
- Check the maximum capacity when calling `EventManager::new`.

## v0.2.0

### Fixed

- Fixed a race condition that might lead to wrongfully call the dispatch
  function for an inactive event
  ([[#41]](https://github.com/rust-vmm/event-manager/issues/41)).

### Added

- By default, the event manager can dispatch 256 events at one time. This limit
  can now be increased by using the `new_with_capacity` constructor
  ([[#37]](https://github.com/rust-vmm/event-manager/issues/37)).

## v0.1.0

This is the first release of event-manager.
The event-manager provides abstractions for implementing event based systems.
For now, this crate only works on Linux and uses the epoll API to provide a
mechanism for handling I/O notifications.
