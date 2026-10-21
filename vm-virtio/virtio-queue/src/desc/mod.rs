//! Descriptor types for virtio queue.

use zerocopy::little_endian::{U16, U32, U64};
use zerocopy::{FromBytes, Immutable, IntoBytes};

pub mod packed;
pub mod split;

/// a virtio descriptor
#[deprecated = "Descriptor has been deprecated. Please use RawDescriptor"]
pub type Descriptor = RawDescriptor;

/// A virtio descriptor's layout constraints with C representation.
/// This is a unified representation of the memory layout order
/// for packed descriptors and split descriptors.
/// This type corresponds to struct virtq_desc, see:
/// <https://docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html#x1-720008>
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, FromBytes, IntoBytes, Immutable)]
pub struct RawDescriptor(U64, U32, U16, U16);

impl From<split::Descriptor> for RawDescriptor {
    fn from(desc: split::Descriptor) -> Self {
        RawDescriptor(
            U64::from(desc.addr().0),
            U32::from(desc.len()),
            U16::from(desc.flags()),
            U16::from(desc.next()),
        )
    }
}

impl From<packed::Descriptor> for RawDescriptor {
    fn from(desc: packed::Descriptor) -> Self {
        RawDescriptor(
            U64::from(desc.addr().0),
            U32::from(desc.len()),
            U16::from(desc.id()),
            U16::from(desc.flags()),
        )
    }
}

impl From<RawDescriptor> for split::Descriptor {
    fn from(desc: RawDescriptor) -> split::Descriptor {
        split::Descriptor::new(desc.0.into(), desc.1.into(), desc.2.into(), desc.3.into())
    }
}

impl From<RawDescriptor> for packed::Descriptor {
    fn from(desc: RawDescriptor) -> packed::Descriptor {
        packed::Descriptor::new(desc.0.into(), desc.1.into(), desc.2.into(), desc.3.into())
    }
}

#[cfg(test)]
mod tests {

    use super::{packed, split, RawDescriptor, U16, U32, U64};

    #[test]
    fn test_desc_from_split() {
        let split_desc = split::Descriptor::new(1, 2, 3, 4);
        let desc = RawDescriptor::from(split_desc);
        assert_eq!(split_desc.addr().0, desc.0.get());
        assert_eq!(split_desc.len(), desc.1.get());
        assert_eq!(split_desc.flags(), desc.2.get());
        assert_eq!(split_desc.next(), desc.3.get());
    }

    #[test]
    fn test_split_from_desc() {
        let desc = RawDescriptor(U64::from(1), U32::from(2), U16::from(3), U16::from(4));
        let split_desc = split::Descriptor::from(desc);
        assert_eq!(split_desc.addr().0, desc.0.get());
        assert_eq!(split_desc.len(), desc.1.get());
        assert_eq!(split_desc.flags(), desc.2.get());
        assert_eq!(split_desc.next(), desc.3.get());
    }

    #[test]
    fn test_desc_from_packed() {
        let packed_desc = packed::Descriptor::new(1, 2, 3, 4);
        let desc = RawDescriptor::from(packed_desc);
        assert_eq!(packed_desc.addr().0, desc.0.get());
        assert_eq!(packed_desc.len(), desc.1.get());
        assert_eq!(packed_desc.id(), desc.2.get());
        assert_eq!(packed_desc.flags(), desc.3.get());
    }

    #[test]
    fn test_packed_from_desc() {
        let desc = RawDescriptor(U64::from(1), U32::from(2), U16::from(3), U16::from(4));
        let packed_desc = packed::Descriptor::from(desc);
        assert_eq!(packed_desc.addr().0, desc.0.get());
        assert_eq!(packed_desc.len(), desc.1.get());
        assert_eq!(packed_desc.id(), desc.2.get());
        assert_eq!(packed_desc.flags(), desc.3.get());
    }
}
