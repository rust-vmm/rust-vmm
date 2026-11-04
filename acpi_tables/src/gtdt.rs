// Copyright 2026 Arthur Heymans
//
// SPDX-License-Identifier: Apache-2.0

//! Generic Timer Description Table (GTDT).

extern crate alloc;

use crate::{sdt::Sdt, Aml, AmlSink};

const TABLE_LENGTH: usize = 104;
const TIMER_BLOCK_LENGTH: usize = 20;
const TIMER_FRAME_LENGTH: usize = 40;
const WATCHDOG_LENGTH: usize = 28;

/// Physical address value used when a GTDT register block is not present.
pub const ADDRESS_NOT_PRESENT: u64 = u64::MAX;

/// Flags for the fixed GTDT timer entries.
pub mod timer_flags {
    pub const EDGE_TRIGGERED: u32 = 1 << 0;
    pub const ACTIVE_LOW: u32 = 1 << 1;
    pub const ALWAYS_ON: u32 = 1 << 2;
}

/// Interrupt flags for a GT Block timer frame.
pub mod frame_timer_flags {
    pub const EDGE_TRIGGERED: u32 = 1 << 0;
    pub const ACTIVE_LOW: u32 = 1 << 1;
}

/// Common flags for a GT Block timer frame.
pub mod frame_common_flags {
    pub const SECURE: u32 = 1 << 0;
    pub const ALWAYS_ON: u32 = 1 << 1;
}

/// Flags for an Arm Generic Watchdog.
pub mod watchdog_flags {
    pub const EDGE_TRIGGERED: u32 = 1 << 0;
    pub const ACTIVE_LOW: u32 = 1 << 1;
    pub const SECURE: u32 = 1 << 2;
}

/// A timer interrupt and its context-specific flags.
#[derive(Clone, Copy, Debug, Default)]
pub struct TimerInterrupt {
    pub gsiv: u32,
    pub flags: u32,
}

/// One timer frame in a [`TimerBlock`].
#[derive(Clone, Copy, Debug)]
pub struct TimerFrame {
    pub frame_number: u8,
    pub base_address: u64,
    pub el0_base_address: u64,
    pub physical_timer: TimerInterrupt,
    pub virtual_timer: TimerInterrupt,
    pub common_flags: u32,
}

/// A Generic Timer Block platform timer structure.
#[derive(Clone, Copy, Debug)]
pub struct TimerBlock<'a> {
    pub control_base: u64,
    pub frames: &'a [TimerFrame],
}

/// An Arm Generic Watchdog platform timer structure.
#[derive(Clone, Copy, Debug)]
pub struct Watchdog {
    pub refresh_base: u64,
    pub control_base: u64,
    pub timer: TimerInterrupt,
}

/// A GTDT platform timer structure.
#[derive(Clone, Copy, Debug)]
pub enum PlatformTimer<'a> {
    TimerBlock(TimerBlock<'a>),
    Watchdog(Watchdog),
}

impl PlatformTimer<'_> {
    fn encoded_len(&self) -> usize {
        match self {
            Self::TimerBlock(block) => {
                assert!((1..=8).contains(&block.frames.len()));
                TIMER_BLOCK_LENGTH + block.frames.len() * TIMER_FRAME_LENGTH
            }
            Self::Watchdog(_) => WATCHDOG_LENGTH,
        }
    }

    fn write(&self, table: &mut Sdt, offset: usize) {
        match self {
            Self::TimerBlock(block) => {
                let length = self.encoded_len();
                table.write_u8(offset, 0);
                table.write_u16(offset + 1, length as u16);
                table.write_u64(offset + 4, block.control_base);
                table.write_u32(offset + 12, block.frames.len() as u32);
                table.write_u32(offset + 16, TIMER_BLOCK_LENGTH as u32);

                for (index, frame) in block.frames.iter().enumerate() {
                    assert!(frame.frame_number < 8);
                    let frame_offset = offset + TIMER_BLOCK_LENGTH + index * TIMER_FRAME_LENGTH;
                    table.write_u8(frame_offset, frame.frame_number);
                    table.write_u64(frame_offset + 4, frame.base_address);
                    table.write_u64(frame_offset + 12, frame.el0_base_address);
                    table.write_u32(frame_offset + 20, frame.physical_timer.gsiv);
                    table.write_u32(frame_offset + 24, frame.physical_timer.flags);
                    table.write_u32(frame_offset + 28, frame.virtual_timer.gsiv);
                    table.write_u32(frame_offset + 32, frame.virtual_timer.flags);
                    table.write_u32(frame_offset + 36, frame.common_flags);
                }
            }
            Self::Watchdog(watchdog) => {
                table.write_u8(offset, 1);
                table.write_u16(offset + 1, WATCHDOG_LENGTH as u16);
                table.write_u64(offset + 4, watchdog.refresh_base);
                table.write_u64(offset + 12, watchdog.control_base);
                table.write_u32(offset + 20, watchdog.timer.gsiv);
                table.write_u32(offset + 24, watchdog.timer.flags);
            }
        }
    }
}

/// GTDT configuration for Arm generic timers.
#[derive(Clone, Copy, Debug)]
pub struct Config<'a> {
    pub counter_control_base: u64,
    pub secure_el1_timer: TimerInterrupt,
    pub nonsecure_el1_timer: TimerInterrupt,
    pub virtual_el1_timer: TimerInterrupt,
    pub nonsecure_el2_timer: TimerInterrupt,
    pub counter_read_base: u64,
    pub virtual_el2_timer: TimerInterrupt,
    pub platform_timers: &'a [PlatformTimer<'a>],
}

/// Generic Timer Description Table.
pub struct GTDT {
    table: Sdt,
}

impl GTDT {
    /// Create a complete revision 3 GTDT.
    pub fn new(
        oem_id: [u8; 6],
        oem_table_id: [u8; 8],
        oem_revision: u32,
        config: Config<'_>,
    ) -> Self {
        let length = config
            .platform_timers
            .iter()
            .try_fold(TABLE_LENGTH, |length, timer| {
                length.checked_add(timer.encoded_len())
            })
            .expect("GTDT length overflow");
        let mut table = Sdt::new(
            *b"GTDT",
            u32::try_from(length).expect("GTDT exceeds the ACPI table length field"),
            3,
            oem_id,
            oem_table_id,
            oem_revision,
        );

        table.write_u64(36, config.counter_control_base);
        table.write_u32(48, config.secure_el1_timer.gsiv);
        table.write_u32(52, config.secure_el1_timer.flags);
        table.write_u32(56, config.nonsecure_el1_timer.gsiv);
        table.write_u32(60, config.nonsecure_el1_timer.flags);
        table.write_u32(64, config.virtual_el1_timer.gsiv);
        table.write_u32(68, config.virtual_el1_timer.flags);
        table.write_u32(72, config.nonsecure_el2_timer.gsiv);
        table.write_u32(76, config.nonsecure_el2_timer.flags);
        table.write_u64(80, config.counter_read_base);
        table.write_u32(
            88,
            u32::try_from(config.platform_timers.len())
                .expect("GTDT platform timer count exceeds u32"),
        );
        table.write_u32(
            92,
            if config.platform_timers.is_empty() {
                0
            } else {
                TABLE_LENGTH as u32
            },
        );
        table.write_u32(96, config.virtual_el2_timer.gsiv);
        table.write_u32(100, config.virtual_el2_timer.flags);

        let mut offset = TABLE_LENGTH;
        for platform_timer in config.platform_timers {
            platform_timer.write(&mut table, offset);
            offset += platform_timer.encoded_len();
        }

        Self { table }
    }
}

impl Aml for GTDT {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        self.table.to_aml_bytes(sink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn test_complete_gtdt() {
        let frames = [TimerFrame {
            frame_number: 3,
            base_address: 0x1000,
            el0_base_address: 0x2000,
            physical_timer: TimerInterrupt {
                gsiv: 40,
                flags: frame_timer_flags::ACTIVE_LOW,
            },
            virtual_timer: TimerInterrupt {
                gsiv: 41,
                flags: frame_timer_flags::EDGE_TRIGGERED,
            },
            common_flags: frame_common_flags::SECURE | frame_common_flags::ALWAYS_ON,
        }];
        let platform_timers = [
            PlatformTimer::TimerBlock(TimerBlock {
                control_base: 0x3000,
                frames: &frames,
            }),
            PlatformTimer::Watchdog(Watchdog {
                refresh_base: 0x4000,
                control_base: 0x5000,
                timer: TimerInterrupt {
                    gsiv: 48,
                    flags: watchdog_flags::ACTIVE_LOW | watchdog_flags::SECURE,
                },
            }),
        ];
        let table = GTDT::new(
            *b"RUSTVM",
            *b"TESTGTDT",
            1,
            Config {
                counter_control_base: 0x6000,
                secure_el1_timer: TimerInterrupt { gsiv: 29, flags: 1 },
                nonsecure_el1_timer: TimerInterrupt { gsiv: 30, flags: 2 },
                virtual_el1_timer: TimerInterrupt { gsiv: 27, flags: 3 },
                nonsecure_el2_timer: TimerInterrupt { gsiv: 26, flags: 4 },
                counter_read_base: 0x7000,
                virtual_el2_timer: TimerInterrupt { gsiv: 28, flags: 5 },
                platform_timers: &platform_timers,
            },
        );
        let mut bytes = Vec::new();
        table.to_aml_bytes(&mut bytes);

        assert_eq!(bytes.len(), TABLE_LENGTH + 60 + WATCHDOG_LENGTH);
        assert_eq!(
            bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)),
            0
        );
        assert_eq!(
            u64::from_le_bytes(bytes[80..88].try_into().unwrap()),
            0x7000
        );
        assert_eq!(u32::from_le_bytes(bytes[88..92].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(bytes[96..100].try_into().unwrap()), 28);

        let block = TABLE_LENGTH;
        assert_eq!(bytes[block], 0);
        assert_eq!(
            u16::from_le_bytes(bytes[block + 1..block + 3].try_into().unwrap()),
            60
        );
        assert_eq!(
            u32::from_le_bytes(bytes[block + 12..block + 16].try_into().unwrap()),
            1
        );
        let frame = block + TIMER_BLOCK_LENGTH;
        assert_eq!(bytes[frame], 3);
        assert_eq!(
            u64::from_le_bytes(bytes[frame + 4..frame + 12].try_into().unwrap()),
            0x1000
        );
        assert_eq!(
            u32::from_le_bytes(bytes[frame + 36..frame + 40].try_into().unwrap()),
            3
        );

        let watchdog = block + 60;
        assert_eq!(bytes[watchdog], 1);
        assert_eq!(
            u16::from_le_bytes(bytes[watchdog + 1..watchdog + 3].try_into().unwrap()),
            28
        );
        assert_eq!(
            u32::from_le_bytes(bytes[watchdog + 20..watchdog + 24].try_into().unwrap()),
            48
        );
    }

    #[test]
    fn test_gtdt_without_platform_timers() {
        let table = GTDT::new(
            *b"RUSTVM",
            *b"TESTGTDT",
            1,
            Config {
                counter_control_base: ADDRESS_NOT_PRESENT,
                secure_el1_timer: TimerInterrupt::default(),
                nonsecure_el1_timer: TimerInterrupt::default(),
                virtual_el1_timer: TimerInterrupt::default(),
                nonsecure_el2_timer: TimerInterrupt::default(),
                counter_read_base: ADDRESS_NOT_PRESENT,
                virtual_el2_timer: TimerInterrupt::default(),
                platform_timers: &[],
            },
        );
        let mut bytes = Vec::new();
        table.to_aml_bytes(&mut bytes);

        assert_eq!(bytes.len(), TABLE_LENGTH);
        assert_eq!(u32::from_le_bytes(bytes[88..92].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(bytes[92..96].try_into().unwrap()), 0);
    }
}
