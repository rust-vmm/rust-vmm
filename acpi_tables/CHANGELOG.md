# Upcoming Release

## Added

- Added AML encoders for commonly used control, reference, arithmetic,
  timing, and thermal-zone operations.
- Added GPIO interrupt, GPIO I/O, I2C, and SPI resource descriptors.
- Added a fixed-size buffer implementation of `AmlSink`.
- Added revision 3 Generic Timer Description Table generation.
- Added IO Remapping Table generation for PCI-to-GIC ITS mappings.
- Added Debug Port Table 2 generation for ARM PL011 UARTs.
- Added Serial Port Console Redirection Table generation for ARM PL011 UARTs.
- Added High Precision Event Timer table generation.

## Fixed

- Fixed the AML opcode ordering for `PowerResource` objects.
