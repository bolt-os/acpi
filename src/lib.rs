#![no_std]
#![feature(
    new_uninit,                                 // https://github.com/rust-lang/rust/issues/63291
    ptr_metadata,                               // https://github.com/rust-lang/rust/issues/81513
    strict_provenance,                          // https://github.com/rust-lang/rust/issues/95228
)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod fadt;
pub mod madt;

use alloc::boxed::Box;
use core::{
    fmt,
    mem::{size_of, MaybeUninit},
    ptr::addr_of,
};

#[repr(C)]
pub struct Rsdp {
    pub signature: [u8; 8],
    pub checksum:  u8,
    pub oem_id:    [u8; 6],
    pub revision:  u8,
    pub rsdt_addr: u32,

    // ACPI Revision >= 2
    pub length: u32,
    pub xsdt_addr: u64,
    pub x_checksum: u8,
    pub reserved: [u8; 3],
}

#[repr(C, packed)]
pub struct Header {
    pub signature:        [u8; 4],
    pub length:           u32,
    pub revision:         u8,
    pub checksum:         u8,
    pub oem_id:           [u8; 6],
    pub oem_table_id:     u64,
    pub oem_revision:     u32,
    pub creator_id:       u32,
    pub creator_revision: u32,
}

pub trait Bridge: Clone + Copy + fmt::Debug {
    fn map(&self, phys: usize, size: usize) -> usize;
    fn remap(&self, virt: usize, new_size: usize) -> usize;
    fn unmap(&self, virt: usize);
}

#[cfg(feature = "alloc")]
pub struct RootTable<B: Bridge> {
    pub acpi_revision: u8,
    tables:            Box<[*const Header]>,
    bridge:            B,
}

impl<B: Bridge> RootTable<B> {
    pub unsafe fn new(ptr: *const u8, bridge: B) -> RootTable<B> {
        unsafe fn get_table_ptrs<T: Into<u64>, B: Bridge>(
            table: *const Header,
            bridge: B,
        ) -> Box<[*const Header]> {
            let table = table.with_addr(bridge.map(table.addr(), size_of::<Header>()));

            let table_len = addr_of!((*table).length).read_unaligned() as usize;
            let len = (table_len - size_of::<Header>()) / size_of::<T>();
            let data = table.add(1).cast::<T>();
            let mut tables = Box::new_uninit_slice(len);
            for i in 0..len {
                let addr = data.add(i).read_unaligned().into();
                tables
                    .as_mut_ptr()
                    .add(i)
                    .write(MaybeUninit::new(addr as *const Header));
            }
            tables.assume_init()
        }

        let rsdp = unsafe {
            let ptr = ptr.with_addr(bridge.map(ptr.addr(), size_of::<Rsdp>()));
            ptr.cast::<Rsdp>().read_unaligned()
        };

        let acpi_revision = rsdp.revision;
        let tables = if acpi_revision >= 2 {
            let xsdt = rsdp.xsdt_addr as *const Header;
            get_table_ptrs::<u64, B>(xsdt, bridge)
        } else {
            let rsdt = rsdp.rsdt_addr as *const Header;
            get_table_ptrs::<u32, B>(rsdt, bridge)
        };

        Self {
            acpi_revision,
            tables,
            bridge,
        }
    }

    fn map_ptr<T: ?Sized>(&self, ptr: *const T, size: usize) -> *const T {
        ptr.with_addr(self.bridge.map(ptr.addr(), size))
    }

    fn remap_ptr<T: ?Sized>(&self, ptr: *const T, new_size: usize) -> *const T {
        ptr.with_addr(self.bridge.remap(ptr.addr(), new_size))
    }

    fn unmap_ptr<T: ?Sized>(&self, ptr: *const T) {
        self.bridge.unmap(ptr.addr());
    }

    pub fn get_table<T: Sdt>(&self) -> Option<*const T> {
        self.tables.iter().find_map(|ptr| {
            let ptr = self.map_ptr(*ptr, size_of::<Header>());
            let header = unsafe { ptr.read_unaligned() };

            if header.signature == T::SIGNATURE {
                let ptr = self.remap_ptr(ptr, header.length as usize);
                Some(ptr.cast())
            } else {
                None
            }
        })
    }
}

/// System Description Table
pub trait Sdt {
    const SIGNATURE: [u8; 4];
}
