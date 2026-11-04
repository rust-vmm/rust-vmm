// Copyright 2026 Arthur Heymans
//
// SPDX-License-Identifier: Apache-2.0

//! IO Remapping Table (IORT) support.

extern crate alloc;

use crate::{sdt::Sdt, Aml, AmlSink};

const TABLE_HEADER_LENGTH: usize = 48;
const NODE_HEADER_LENGTH: usize = 16;
const ROOT_COMPLEX_DATA_LENGTH: usize = 24;
const ID_MAPPING_LENGTH: usize = 20;

/// Allocation hints for PCI Root Complex memory accesses.
pub mod allocation_hints {
    pub const TRANSIENT: u8 = 1 << 0;
    pub const WRITE: u8 = 1 << 1;
    pub const READ: u8 = 1 << 2;
    pub const OVERRIDE: u8 = 1 << 3;
}

/// Flags describing PCI Root Complex memory accesses.
pub mod memory_access_flags {
    pub const COHERENT_PATH_TO_MEMORY: u8 = 1 << 0;
    pub const DEVICE_ATTRIBUTES_CACHEABLE_SHAREABLE: u8 = 1 << 1;
    pub const CAN_WRITE_BACK_SNOOPS: u8 = 1 << 2;
}

/// PCI Root Complex ATS attributes.
pub mod ats_attributes {
    pub const ATS_SUPPORTED: u32 = 1 << 0;
    pub const PRI_SUPPORTED: u32 = 1 << 1;
    pub const PASID_FORWARDING_SUPPORTED: u32 = 1 << 2;
}

/// PCI Root Complex node flags.
pub mod root_complex_flags {
    pub const PASID_SUPPORTED: u32 = 1 << 0;
}

/// Memory access properties advertised by a PCI Root Complex node.
#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryAccessProperties {
    pub cache_coherent: bool,
    pub allocation_hints: u8,
    pub flags: u8,
}

impl MemoryAccessProperties {
    fn encode(self) -> u64 {
        assert_eq!(self.allocation_hints & !0x0f, 0);
        assert_eq!(self.flags & !0x07, 0);

        u64::from(self.cache_coherent)
            | (u64::from(self.allocation_hints) << 32)
            | (u64::from(self.flags) << 56)
    }
}

/// Configuration for an ITS Group and PCI Root Complex IORT.
pub struct Config<'a> {
    pub its_ids: &'a [u32],
    pub pci_segment: u32,
    pub memory_access_properties: MemoryAccessProperties,
    pub ats_attributes: u32,
    pub memory_address_limit: u8,
    pub pasid_capabilities: u16,
    pub root_complex_flags: u32,
    pub id_count: u32,
}

/// IO Remapping Table.
pub struct IORT {
    table: Sdt,
}

impl IORT {
    /// Create a revision 6 IORT with one ITS Group and one PCI Root Complex.
    pub fn new(
        oem_id: [u8; 6],
        oem_table_id: [u8; 8],
        oem_revision: u32,
        config: Config<'_>,
    ) -> Self {
        assert!(!config.its_ids.is_empty());
        assert!(config.id_count > 0);
        assert_eq!(config.ats_attributes & !0x07, 0);
        assert!(config.pasid_capabilities <= 20);
        assert_eq!(config.root_complex_flags & !0x01, 0);

        let its_node_length = NODE_HEADER_LENGTH + 4 + config.its_ids.len() * 4;
        let root_complex_length = NODE_HEADER_LENGTH + ROOT_COMPLEX_DATA_LENGTH + ID_MAPPING_LENGTH;
        let length = TABLE_HEADER_LENGTH + its_node_length + root_complex_length;
        assert!(its_node_length <= u16::MAX as usize);

        let mut table = Sdt::new(
            *b"IORT",
            length as u32,
            6,
            oem_id,
            oem_table_id,
            oem_revision,
        );

        table.write_u32(36, 2); // Node count
        table.write_u32(40, TABLE_HEADER_LENGTH as u32);

        let its_offset = TABLE_HEADER_LENGTH;
        table.write_u8(its_offset, 0); // ITS Group
        table.write_u16(its_offset + 1, its_node_length as u16);
        table.write_u8(its_offset + 3, 1); // Node revision
        table.write_u32(its_offset + NODE_HEADER_LENGTH, config.its_ids.len() as u32);
        for (index, &identifier) in config.its_ids.iter().enumerate() {
            table.write_u32(its_offset + NODE_HEADER_LENGTH + 4 + index * 4, identifier);
        }

        let root_offset = its_offset + its_node_length;
        table.write_u8(root_offset, 2); // PCI Root Complex
        table.write_u16(root_offset + 1, root_complex_length as u16);
        table.write_u8(root_offset + 3, 4); // Node revision
        table.write_u32(root_offset + 4, 1); // Identifier
        table.write_u32(root_offset + 8, 1); // Mapping count
        table.write_u32(
            root_offset + 12,
            (NODE_HEADER_LENGTH + ROOT_COMPLEX_DATA_LENGTH) as u32,
        );

        let root_data = root_offset + NODE_HEADER_LENGTH;
        table.write_u64(root_data, config.memory_access_properties.encode());
        table.write_u32(root_data + 8, config.ats_attributes);
        table.write_u32(root_data + 12, config.pci_segment);
        table.write_u8(root_data + 16, config.memory_address_limit);
        table.write_u16(root_data + 17, config.pasid_capabilities);
        table.write_u32(root_data + 20, config.root_complex_flags);

        let mapping = root_data + ROOT_COMPLEX_DATA_LENGTH;
        table.write_u32(mapping + 4, config.id_count - 1);
        table.write_u32(mapping + 12, its_offset as u32);

        Self { table }
    }
}

impl Aml for IORT {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        self.table.to_aml_bytes(sink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn test_iort() {
        let table = IORT::new(
            *b"RUSTVM",
            *b"TESTIORT",
            1,
            Config {
                its_ids: &[0, 1],
                pci_segment: 2,
                memory_access_properties: MemoryAccessProperties {
                    cache_coherent: true,
                    allocation_hints: allocation_hints::WRITE | allocation_hints::READ,
                    flags: memory_access_flags::COHERENT_PATH_TO_MEMORY
                        | memory_access_flags::DEVICE_ATTRIBUTES_CACHEABLE_SHAREABLE,
                },
                ats_attributes: ats_attributes::ATS_SUPPORTED
                    | ats_attributes::PRI_SUPPORTED
                    | ats_attributes::PASID_FORWARDING_SUPPORTED,
                memory_address_limit: 48,
                pasid_capabilities: 20,
                root_complex_flags: root_complex_flags::PASID_SUPPORTED,
                id_count: 0x1_0000,
            },
        );
        let mut bytes = Vec::new();
        table.to_aml_bytes(&mut bytes);

        assert_eq!(
            bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)),
            0
        );
        assert_eq!(u32::from_le_bytes(bytes[36..40].try_into().unwrap()), 2);
        assert_eq!(u16::from_le_bytes(bytes[49..51].try_into().unwrap()), 28);

        let root_offset = 48 + 28;
        assert_eq!(bytes[root_offset], 2);
        assert_eq!(
            u32::from_le_bytes(
                bytes[root_offset + 28..root_offset + 32]
                    .try_into()
                    .unwrap()
            ),
            2,
        );
        let root_data = root_offset + NODE_HEADER_LENGTH;
        assert_eq!(
            u64::from_le_bytes(bytes[root_data..root_data + 8].try_into().unwrap()),
            0x0300_0006_0000_0001,
        );
        assert_eq!(
            u32::from_le_bytes(bytes[root_data + 8..root_data + 12].try_into().unwrap()),
            7,
        );
        assert_eq!(bytes[root_offset + 32], 48);
        assert_eq!(
            u16::from_le_bytes(bytes[root_data + 17..root_data + 19].try_into().unwrap()),
            20,
        );
        assert_eq!(
            u32::from_le_bytes(bytes[root_data + 20..root_data + 24].try_into().unwrap()),
            root_complex_flags::PASID_SUPPORTED,
        );

        let mapping = root_offset + NODE_HEADER_LENGTH + ROOT_COMPLEX_DATA_LENGTH;
        assert_eq!(
            u32::from_le_bytes(bytes[mapping + 4..mapping + 8].try_into().unwrap()),
            0xffff,
        );
        assert_eq!(
            u32::from_le_bytes(bytes[mapping + 12..mapping + 16].try_into().unwrap()),
            48,
        );
    }
}
