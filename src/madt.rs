use core::{mem::size_of, ptr};

use crate::Sdt;

#[repr(C, packed)]
pub struct Madt {
    pub header:          super::Header,
    pub local_intc_addr: u32,
    pub flags:           Flags,
}

impl Sdt for Madt {
    const SIGNATURE: [u8; 4] = *b"APIC";
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct Flags : u32 {
        const PCAT_COMPAT = 1 << 0;
    }
}

#[repr(C)]
#[derive(Debug)]
struct Header {
    kind: u8,
    len:  u8,
}

#[derive(Clone, Debug)]
pub enum Entry {
    LocalApic {
        processor_uid: u32,
        lapic_id:      u32,
        flags:         LocalApicFlags,
    },
    IoApic {
        apic_id:  u32,
        addr:     usize,
        gsi_base: u32,
    },
    InterruptSourceOverride {
        bus:    u8,
        source: u8,
        gsi:    u32,
        flags:  InterruptSourceOverrideFlags,
    },
    NmiSource {
        flags: u16,
        gsi:   u32,
    },
    LocalApicNmi {
        processor_id: u32,
        flags:        u16,
        lapic_lint:   u8,
    },
    LocalApicAddressOverride {
        addr: usize,
    },
    PlatformSources,
    LocalX2Apic,
    LocalX2ApicNmi,

    RiscvIntc {
        processor_uid: u32,
        hartid:        u64,
        flags:         RiscvIntcFlags,
    },

    Unknown(u8, *const [u8]),
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct LocalApicFlags : u32 {
        const ENABLED = 1 << 0;
        const ONLINE_CAPABLE = 1 << 1;
    }

    #[repr(transparent)]
    pub struct RiscvIntcFlags : u32 {
        const ENABLED = 1 << 0;
        const ONLINE_CAPABLE = 1 << 1;
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct InterruptSourceOverrideFlags(u16);

impl InterruptSourceOverrideFlags {
    pub const POLARITY_MASK: u16 = 0x0003;
    pub const TRIGGER_MASK: u16 = 0x000c;
}

pub unsafe fn iter_madt(madt: *const Madt) -> impl Iterator<Item = Entry> {
    unsafe {
        let mut ptr = madt.cast::<u8>().add(size_of::<Madt>());
        let end = madt.cast::<u8>().add((*madt).header.length as usize);

        core::iter::from_fn(move || {
            if ptr >= end {
                return None;
            }

            let header = ptr.cast::<Header>().read();
            let entry = match header.kind {
                0x00 => {
                    let processor_uid = ptr.add(2).read() as u32;
                    let lapic_id = ptr.add(3).read() as u32;
                    let flags = ptr.add(4).cast::<LocalApicFlags>().read_unaligned();
                    Entry::LocalApic {
                        processor_uid,
                        lapic_id,
                        flags,
                    }
                }
                0x18 => {
                    let version = ptr.add(2).read();
                    assert!(version == 1);
                    let flags = ptr.add(4).cast::<RiscvIntcFlags>().read_unaligned();
                    let hartid = ptr.add(8).cast::<u64>().read_unaligned();
                    let processor_uid = ptr.add(16).cast::<u32>().read_unaligned();
                    Entry::RiscvIntc { processor_uid, hartid, flags }
                }
                kind => {
                    let data = ptr::from_raw_parts(ptr.cast(), header.len as usize);
                    Entry::Unknown(kind, data)
                }
            };
            log::info!("{entry:?}");

            ptr = ptr.add(header.len as usize);
            Some(entry)
        })
    }
}
