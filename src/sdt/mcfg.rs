use crate::Sdt;
use core::ptr::addr_of;

#[repr(C, packed)]
pub struct Mcfg {
    pub header: super::Header,
    reserved: u64,
    entries: [Entry],
}

unsafe impl Sdt for Mcfg {
    const SIGNATURE: super::Signature = super::Signature(*b"MCFG");

    fn header(&self) -> &super::Header {
        &self.header
    }

    unsafe fn from_header_ptr(header: *const super::Header) -> *const Self {
        super::from_header_ptr_slice_of::<Entry, _>(header)
    }
}

impl Mcfg {
    pub fn entries(&self) -> impl Iterator<Item = Entry> {
        let ptr = addr_of!(self.entries);
        (0..ptr.len()).map(move |i| unsafe { ptr.get_unchecked(i).read_unaligned() })
    }
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct Entry {
    pub ecam_base: u64,
    pub segment: u16,
    pub bus_start: u8,
    pub bus_end: u8,
    pub reserved: u32,
}
