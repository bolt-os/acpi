//! RISC-V Hart Capabilities Table

use crate::size_of_unsized;
use core::{
    mem::size_of,
    ptr::{self, addr_of, Pointee},
};
use libsa::endian::{u16_le, u32_le, u64_le};

/// RISC-V Hart Capabilities Table
///
/// Describes certain features of the system's harts.
///
/// This table is required to be present on RISC-V platforms.
#[repr(C, packed)]
pub struct Rhct {
    pub header: super::Header,
    flags: u32_le,
    time_base_frequency: u64_le,
    nodes_len: u32_le,
    nodes_offset: u32_le,
    nodes: [u8],
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct RhctFlags : u32 {
        const TIMER_CANNOT_WAKE = 1 << 0;
    }
}

unsafe impl super::Sdt for Rhct {
    const SIGNATURE: super::Signature = super::Signature(*b"RHCT");

    fn header(&self) -> &super::Header {
        &self.header
    }

    unsafe fn from_header_ptr(header: *const super::Header) -> *const Self {
        super::from_header_ptr_slice_of::<u8, _>(header)
    }
}

impl Rhct {
    #[inline]
    pub fn flags(&self) -> RhctFlags {
        RhctFlags::from_bits_retain(self.flags.get())
    }

    /// Returns the frequency of the system counter.
    ///
    /// This value is the same for all harts.
    #[inline]
    pub fn time_base_frequency(&self) -> u64 {
        self.time_base_frequency.get()
    }

    /// Returns the number of [`Node`] entries in the RHCT
    #[inline]
    pub fn len(&self) -> usize {
        self.nodes_len.get() as usize
    }

    /// Returns `true` if the [`Node`] array is empty
    ///
    /// This is equivalent to checking if [`.len()`](Rhct::len) returns `0`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get_node(&self, offset: usize) -> Option<*const Header> {
        // Offsets are given relative to the start of the table.
        // Relocate it relative to the start of the `nodes` array.
        let offset = offset - size_of_unsized::<Self>();
        let node = unsafe { &*self.nodes.get(offset..)?.as_ptr().cast::<Header>() };
        // Create a pointer with provenance over all bytes of the node.
        let bytes = self.nodes.get(offset..offset + node.len())?;
        Some(bytes.as_ptr().cast::<Header>())
    }

    /// Returns an iterator over all `HartInfo` nodes
    pub fn nodes(&self) -> impl Iterator<Item = &HartInfo> + '_ {
        let mut offset = self.nodes_offset.get() as usize;
        (0..self.nodes_len.get()).filter_map(move |_| unsafe {
            let header = self.get_node(offset)?;
            offset += (*header).len();
            if (*header).r#type == NodeType::HART_INFO {
                Some(HartInfo::from_header(header))
            } else {
                None
            }
        })
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, PartialOrd)]
pub struct NodeType(pub u16_le);

impl NodeType {
    pub const ISA_STRING: Self = Self(u16_le::new(0));
    pub const CMO_INFO: Self = Self(u16_le::new(1));
    pub const MMU_INFO: Self = Self(u16_le::new(2));
    pub const HART_INFO: Self = Self(u16_le::new(65535));
}

impl core::fmt::Debug for NodeType {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let name = match *self {
            Self::ISA_STRING => "ISA_STRING",
            Self::CMO_INFO => "CMO_INFO",
            Self::MMU_INFO => "MMU_INFO",
            Self::HART_INFO => "HART_INFO",
            _ => return write!(f, "NodeType({})", self.0.get()),
        };
        write!(f, "NodeType::{name}")
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub r#type: NodeType,
    pub len: u16,
    pub revision: u16,
}

impl Header {
    #[inline]
    pub const fn node_type(self) -> NodeType {
        self.r#type
    }

    /// Returns the length of the entire structure, in bytes
    #[inline]
    pub const fn len(self) -> usize {
        self.len as usize
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
}

trait FromHeader {
    /// Create a pointer to a full node from a pointer to its [`NodeHeader`]
    ///
    /// This function handles creating the (potentially fat) pointer to `Self`.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`NodeHeader`] with a matching signature, with
    /// provenance over **all** bytes of the entire table, as specified by the `len` field
    /// of `NodeHeader`.
    unsafe fn from_header<'a>(header: *const Header) -> &'a Self;
}

unsafe fn from_header_slice_of<'a, T: 'a, N>(header: *const Header) -> &'a N
where
    N: 'a + ?Sized + Pointee<Metadata = usize>,
{
    let len = ((*header).len() - size_of_unsized::<N>()) / size_of::<T>();
    &*ptr::from_raw_parts(header, len)
}

#[repr(C, packed)]
pub struct HartInfo {
    header: Header,
    offsets_len: u16_le,
    acpi_processor_uid: u32_le,
    offsets: [u32_le],
}

impl FromHeader for HartInfo {
    unsafe fn from_header<'a>(header: *const Header) -> &'a Self {
        from_header_slice_of::<u32_le, _>(header)
    }
}

impl HartInfo {
    #[inline]
    pub fn acpi_processor_uid(&self) -> u32 {
        self.acpi_processor_uid.get()
    }

    pub fn entries<'rhct>(&self, rhct: &'rhct Rhct) -> impl Iterator<Item = Entry> + 'rhct {
        let offsets = addr_of!(self.offsets);
        (0..offsets.len()).map(move |index| unsafe {
            let offset = offsets.get_unchecked(index).read_unaligned().get();
            let header = rhct.get_node(offset as usize).unwrap();
            match (*header).r#type {
                NodeType::ISA_STRING => Entry::IsaString(IsaString::from_header(header)),
                NodeType::CMO_INFO => Entry::CmoInfo(CmoInfo::from_header(header)),
                NodeType::MMU_INFO => Entry::MmuInfo(MmuInfo::from_header(header)),
                _ => Entry::Unknown(Unknown::from_header(header)),
            }
        })
    }
}

pub enum Entry<'a> {
    IsaString(&'a IsaString),
    CmoInfo(&'a CmoInfo),
    MmuInfo(&'a MmuInfo),
    Unknown(&'a Unknown),
}

pub struct Unknown {
    pub header: Header,
    pub data: [u8],
}

impl FromHeader for Unknown {
    unsafe fn from_header<'a>(header: *const Header) -> &'a Self {
        from_header_slice_of::<u8, _>(header)
    }
}

/// RISC-V ISA String Node
#[repr(C, packed)]
pub struct IsaString {
    pub header: Header,
    /// Length of the ISA string in bytes, including the NUL terminator.
    isa_string_len: u16_le,
    /// NUL-terminated RISC-V ISA string.
    isa_string: [u8],
}

impl FromHeader for IsaString {
    unsafe fn from_header<'a>(header: *const Header) -> &'a Self {
        from_header_slice_of::<u8, _>(header)
    }
}

impl IsaString {
    #[inline]
    pub fn len(&self) -> usize {
        self.isa_string_len.get() as usize - 1
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.isa_string[..self.len()]
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).unwrap()
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct CmoInfo {
    pub header: Header,
    rsvd0: u8,
    cbom_block_size: u8,
    cbop_block_size: u8,
    cboz_block_size: u8,
}

impl FromHeader for CmoInfo {
    unsafe fn from_header<'a>(header: *const Header) -> &'a Self {
        &*header.cast()
    }
}

impl CmoInfo {
    /// Returns the block size for cache block management instructions
    ///
    /// OSPM must ingore this field if the Zicbom extension is not present.
    #[inline]
    pub fn cbom_block_size(&self) -> usize {
        1 << self.cbom_block_size
    }

    /// Returns the block size for cache block prefetch instructions
    ///
    /// OSPM must ingore this field if the Zicbop extension is not present.
    #[inline]
    pub fn cbop_block_size(&self) -> usize {
        1 << self.cbop_block_size
    }

    /// Returns the block size for cache block zero instructions
    ///
    /// OSPM must ingore this field if the Zicboz extension is not present.
    #[inline]
    pub fn cboz_block_size(&self) -> usize {
        1 << self.cboz_block_size
    }
}

/// Memory Management Unit (MMU) Information Node
#[repr(C, packed)]
pub struct MmuInfo {
    pub header: Header,
    rsvd0: u8,
    pub mmu_type: MmuType,
}

impl FromHeader for MmuInfo {
    unsafe fn from_header<'a>(header: *const Header) -> &'a Self {
        &*header.cast()
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, PartialOrd)]
pub struct MmuType(pub u16);

impl MmuType {
    pub const SV39: Self = Self(0);
    pub const SV48: Self = Self(1);
    pub const SV57: Self = Self(2);
}
