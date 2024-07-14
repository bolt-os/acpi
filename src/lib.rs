#![no_std]
#![feature(
    allocator_api,                              // https://github.com/rust-lang/rust/issues/32838
    arbitrary_self_types,                       // https://github.com/rust-lang/rust/issues/44874
    layout_for_ptr,                             // https://github.com/rust-lang/rust/issues/69835
    ptr_metadata,                               // https://github.com/rust-lang/rust/issues/81513
    slice_ptr_get,                              // https://github.com/rust-lang/rust/issues/74265
    strict_provenance,                          // https://github.com/rust-lang/rust/issues/95228
    exposed_provenance,                         // https://github.com/rust-lang/rust/issues/95228
)]

pub mod sdt;

pub use sdt::{RootTable, Sdt};

use core::{
    mem,
    ptr::{self, Pointee},
};

#[repr(C)]
pub struct Rsdp {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub revision: u8,
    pub rsdt_addr: u32,

    // ACPI Revision >= 2
    pub length: u32,
    pub xsdt_addr: u64,
    pub x_checksum: u8,
    pub reserved: [u8; 3],
}

fn size_of_unsized<T: ?Sized + Pointee<Metadata = usize>>() -> usize {
    let ptr = ptr::from_raw_parts::<T>((1usize << (usize::BITS - 1)) as *const (), 0);
    unsafe { mem::size_of_val_raw(ptr) }
}
