// Copyright 2026 Arthur Heymans
//
// SPDX-License-Identifier: Apache-2.0

//! High Precision Event Timer table (HPET).

extern crate alloc;

use crate::{sdt::GenericAddress, sdt::Sdt, Aml, AmlSink};

/// Width of the HPET main counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterSize {
    Bits32,
    Bits64,
}

/// Fields copied from the HPET General Capabilities and ID register.
#[derive(Clone, Copy, Debug)]
pub struct EventTimerBlockId {
    pub hardware_revision: u8,
    pub comparator_count: u8,
    pub counter_size: CounterSize,
    pub legacy_replacement: bool,
    pub pci_vendor_id: u16,
}

impl EventTimerBlockId {
    fn encode(self) -> u32 {
        assert!((3..=32).contains(&self.comparator_count));

        u32::from(self.hardware_revision)
            | (u32::from(self.comparator_count - 1) << 8)
            | (u32::from(self.counter_size == CounterSize::Bits64) << 13)
            | (u32::from(self.legacy_replacement) << 15)
            | (u32::from(self.pci_vendor_id) << 16)
    }
}

/// Page protection guaranteed for the HPET register block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PageProtection {
    None = 0,
    Protected4KiB = 1,
    Protected64KiB = 2,
}

/// HPET configuration.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub event_timer_block_id: EventTimerBlockId,
    pub base_address: u64,
    pub number: u8,
    pub minimum_tick: u16,
    pub page_protection: PageProtection,
    pub oem_attribute: u8,
}

/// High Precision Event Timer table.
pub struct HPET {
    table: Sdt,
}

impl HPET {
    /// Create a complete revision 1 HPET table.
    pub fn new(oem_id: [u8; 6], oem_table_id: [u8; 8], oem_revision: u32, config: Config) -> Self {
        assert_eq!(config.base_address & 0x3ff, 0);
        assert!(config.oem_attribute <= 0x0f);

        let mut table = Sdt::new(*b"HPET", 56, 1, oem_id, oem_table_id, oem_revision);
        table.write_u32(36, config.event_timer_block_id.encode());
        table.write(
            40,
            GenericAddress {
                address_space_id: 0,
                register_bit_width: 64,
                register_bit_offset: 0,
                access_size: 0,
                address: config.base_address,
            },
        );
        table.write_u8(52, config.number);
        table.write_u16(53, config.minimum_tick);
        table.write_u8(
            55,
            (config.oem_attribute << 4) | config.page_protection as u8,
        );

        Self { table }
    }
}

impl Aml for HPET {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        self.table.to_aml_bytes(sink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn test_complete_hpet() {
        let table = HPET::new(
            *b"RUSTVM",
            *b"TESTHPET",
            1,
            Config {
                event_timer_block_id: EventTimerBlockId {
                    hardware_revision: 0xa5,
                    comparator_count: 32,
                    counter_size: CounterSize::Bits64,
                    legacy_replacement: true,
                    pci_vendor_id: 0x8086,
                },
                base_address: 0xfed0_0000,
                number: 3,
                minimum_tick: 128,
                page_protection: PageProtection::Protected64KiB,
                oem_attribute: 0xb,
            },
        );
        let mut bytes = Vec::new();
        table.to_aml_bytes(&mut bytes);

        assert_eq!(bytes.len(), 56);
        assert_eq!(
            bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)),
            0
        );
        assert_eq!(
            u32::from_le_bytes(bytes[36..40].try_into().unwrap()),
            0x8086_bfa5,
        );
        assert_eq!(bytes[40..44], [0, 64, 0, 0]);
        assert_eq!(
            u64::from_le_bytes(bytes[44..52].try_into().unwrap()),
            0xfed0_0000
        );
        assert_eq!(bytes[52], 3);
        assert_eq!(u16::from_le_bytes(bytes[53..55].try_into().unwrap()), 128);
        assert_eq!(bytes[55], 0xb2);
    }

    #[test]
    #[should_panic]
    fn test_hpet_rejects_invalid_comparator_count() {
        let _ = HPET::new(
            *b"RUSTVM",
            *b"TESTHPET",
            1,
            Config {
                event_timer_block_id: EventTimerBlockId {
                    hardware_revision: 1,
                    comparator_count: 2,
                    counter_size: CounterSize::Bits32,
                    legacy_replacement: false,
                    pci_vendor_id: 0,
                },
                base_address: 0,
                number: 0,
                minimum_tick: 0,
                page_protection: PageProtection::None,
                oem_attribute: 0,
            },
        );
    }
}
