use crate::{size_of_unsized, Sdt};
use core::{mem::size_of, ptr};
use libsa::endian::{u16_le, u32_le, u64_le};

/// Multiple APIC Description Table
#[repr(C, packed)]
pub struct Madt {
    header: super::Header,
    local_intc_addr: u32_le,
    flags: u32_le,
    ics: [u8],
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    pub struct MadtFlags : u32 {
        const PCAT_COMPAT = 1 << 0;
    }
}

unsafe impl Sdt for Madt {
    const SIGNATURE: super::Signature = super::Signature(*b"APIC");

    unsafe fn from_header_ptr(header: *const super::Header) -> *const Self {
        super::from_header_ptr_slice_of::<u8, _>(header)
    }
}

impl Madt {
    #[inline]
    pub fn flags(&self) -> MadtFlags {
        MadtFlags::from_bits_retain(self.flags.get())
    }

    #[inline]
    pub fn local_intc_addr(&self) -> u32 {
        self.local_intc_addr.get()
    }

    pub fn entries(&self) -> impl Iterator<Item = Entry> + '_ {
        let mut offset = 0;
        core::iter::from_fn(move || unsafe {
            let header = &*self
                .ics
                .get(offset..offset + size_of::<Header>())?
                .as_ptr()
                .cast::<Header>();
            let bytes = self.ics.get(offset..offset + header.total_size as usize)?;
            offset += header.total_size as usize;

            macro_rules! cast {
                ($to:ident) => {
                    Entry::$to(&*bytes.as_ptr().cast::<$to>())
                };
            }

            let entry = match bytes[0] {
                0x00 => cast!(LocalApic),
                0x01 => cast!(IoApic),
                0x02 => cast!(InterruptSourceOverride),
                0x03 => cast!(NmiSource),
                0x04 => cast!(LocalApicNmi),
                0x05 => cast!(LocalApicAddressOverride),
                0x09 => cast!(LocalX2Apic),
                0x0a => cast!(LocalX2ApicNmi),
                0x0b => cast!(GicCpuInterface),
                0x0c => cast!(GicDistributor),
                0x0d => cast!(GicMsiFrame),
                0x0e => cast!(GicRedistributor),
                0x0f => cast!(GicInterruptTranslationService),
                0x10 => cast!(MultiprocessorWakeup),
                0x18 => cast!(RiscvIntc),
                _ => {
                    let len = bytes.len() - size_of_unsized::<Unknown>();
                    Entry::Unknown(&*ptr::from_raw_parts::<Unknown>(bytes.as_ptr(), len))
                }
            };

            Some(entry)
        })
    }
}

pub enum Entry<'a> {
    LocalApic(&'a LocalApic),
    IoApic(&'a IoApic),
    InterruptSourceOverride(&'a InterruptSourceOverride),
    NmiSource(&'a NmiSource),
    LocalApicNmi(&'a LocalApicNmi),
    LocalApicAddressOverride(&'a LocalApicAddressOverride),
    LocalX2Apic(&'a LocalX2Apic),
    LocalX2ApicNmi(&'a LocalX2ApicNmi),
    GicCpuInterface(&'a GicCpuInterface),
    GicDistributor(&'a GicDistributor),
    GicMsiFrame(&'a GicMsiFrame),
    GicRedistributor(&'a GicRedistributor),
    GicInterruptTranslationService(&'a GicInterruptTranslationService),
    MultiprocessorWakeup(&'a MultiprocessorWakeup),
    RiscvIntc(&'a RiscvIntc),
    Unknown(&'a Unknown),
}

#[repr(C)]
pub struct Header {
    r#type: u8,
    total_size: u8,
}

#[repr(C, packed)]
pub struct Unknown {
    pub header: Header,
    pub data: [u8],
}

#[repr(C, packed)]
pub struct LocalApic {
    header: Header,
    acpi_processor_uid: u8,
    apic_id: u8,
    flags: u32_le,
}

bitflags::bitflags! {
    /// Local APIC Flags
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    pub struct LocalApicFlags : u32 {
        /// Enabled
        ///
        /// If this bit is set the processor is ready for use. If this bit is clear and the
        /// Online Capable bit is set, system hardware supports enabling this processor during
        /// OS runtime. If this bit is clear and the Online Capable bit is also clear, this
        /// processor is unusable, and OSPM shall ignore the contents of the [`ProcessorLocalApic`]
        /// structure.
        const ENABLED = 1 << 0;
        /// Online Capable
        ///
        /// The information conveyed by this bit depends on the value of the Enabled bit. If
        /// the Enabled bit is set, this bit is reserved and must be zero. Otherwise, if this
        /// bit is set, system hardware supports enabling this processor during OS runtime.
        const ONLINE_CAPABLE = 1 << 1;
    }
}

impl LocalApic {
    /// Returns the ACPI Processor UID for the CPU this interrupt controller belongs to
    #[inline]
    pub fn acpi_processor_uid(&self) -> u32 {
        self.acpi_processor_uid as u32
    }

    /// Returns the APIC ID for this APIC
    #[inline]
    pub fn apic_id(&self) -> u32 {
        self.apic_id as u32
    }

    #[inline]
    pub fn flags(&self) -> LocalApicFlags {
        LocalApicFlags::from_bits_retain(self.flags.get())
    }
}

#[repr(C, packed)]
pub struct IoApic {
    header: Header,
    io_apic_id: u8,
    reserved: u8,
    io_apic_addr: u32_le,
    gsi_base: u32_le,
}

impl IoApic {
    /// Returns the I/O APIC ID for this I/O APIC
    #[inline]
    pub fn io_apic_id(&self) -> u32 {
        self.io_apic_id as u32
    }

    /// Returns the base physical address for this I/O APIC
    #[inline]
    pub fn io_apic_addr(&self) -> u32 {
        self.io_apic_addr.get()
    }

    /// Returns the base Global System Interrupt for this I/O APIC
    #[inline]
    pub fn gsi_base(&self) -> u32 {
        self.gsi_base.get()
    }
}

#[repr(C, packed)]
pub struct InterruptSourceOverride {
    header: Header,
    pub bus: u8,
    pub source: u8,
    global_system_interrupt: u32_le,
    flags: u16_le,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct InterruptSourceFlags(u16);

impl InterruptSourceFlags {
    pub const POLARITY_MASK: u16 = 0x0003;
    pub const TRIGGER_MASK: u16 = 0x000c;
}

impl InterruptSourceOverride {
    /// Returns the bus-relative interrupt source as a tuple of `(bus, irq)`
    pub fn source(&self) -> (u8, u8) {
        (self.bus, self.source)
    }

    /// Returns the Global System Interrput signaled by the interrupt source
    #[inline]
    pub fn global_system_interrupt(&self) -> u32 {
        self.global_system_interrupt.get()
    }

    #[inline]
    pub fn flags(&self) -> InterruptSourceFlags {
        InterruptSourceFlags(self.flags.get())
    }
}

/// Non-Maskable Interrupt Source
#[repr(C, packed)]
pub struct NmiSource {
    header: Header,
    flags: u16_le,
    global_system_interrupt: u32_le,
}

impl NmiSource {
    #[inline]
    pub fn flags(&self) -> InterruptSourceFlags {
        InterruptSourceFlags(self.flags.get())
    }

    /// Returns the Global System Interrupt signaled by this NMI
    #[inline]
    pub fn global_system_interrupt(&self) -> u32 {
        self.global_system_interrupt.get()
    }
}

#[repr(C, packed)]
pub struct LocalApicNmi {
    header: Header,
    acpi_processor_uid: u32_le,
    flags: u16_le,
    local_apic_lintn: u8,
}

impl LocalApicNmi {
    /// Returns the ACPI Processor UID for the CPU associated with this NMI
    #[inline]
    pub fn acpi_processor_uid(&self) -> u32 {
        self.acpi_processor_uid.get()
    }

    #[inline]
    pub fn flags(&self) -> InterruptSourceFlags {
        InterruptSourceFlags(self.flags.get())
    }

    /// Returns the Local APIC LINT# pin to which this NMI is connected
    #[inline]
    pub fn local_apic_lintn(&self) -> u8 {
        self.local_apic_lintn
    }
}

#[repr(C, packed)]
pub struct LocalApicAddressOverride {
    header: Header,
    reserved: u16_le,
    local_apic_addr: u64_le,
}

#[repr(C, packed)]
pub struct LocalX2Apic {
    header: Header,
    reserved: u16_le,
    x2apic_id: u32_le,
    flags: u32_le,
    acpi_processor_uid: u32_le,
}

impl LocalX2Apic {
    #[inline]
    pub fn acpi_processor_uid(&self) -> u32 {
        self.acpi_processor_uid.get()
    }
}

#[repr(C, packed)]
pub struct LocalX2ApicNmi {
    header: Header,
    flags: u16_le,
    acpi_processor_uid: u32_le,
    local_x2apic_lintn: u8,
    reserved: [u8; 3],
}

impl LocalX2ApicNmi {
    #[inline]
    pub fn acpi_processor_uid(&self) -> u32 {
        self.acpi_processor_uid.get()
    }

    #[inline]
    pub fn flags(&self) -> InterruptSourceFlags {
        InterruptSourceFlags(self.flags.get())
    }
}

#[repr(C, packed)]
pub struct GicCpuInterface {
    header: Header,
    reserved0: [u8; 2],
    cpu_interface_number: u32_le,
    acpi_processor_uid: u32_le,
    flags: u32_le,
    parking_protocol_version: u32_le,
    performance_interrupt_gsiv: u32_le,
    parked_addr: u64_le,
    physical_base_addr: u64_le,
    gicv: u64_le,
    gich: u64_le,
    vgic_maintenance_interrupt: u32_le,
    gicr_base_addr: u64_le,
    mpidr: u64_le,
    processor_power_efficiency_class: u8,
    reserved1: u8,
    spe_overflow_interrupt: u16_le,
    trbe_interrupt: u16_le,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct GicCpuInterfaceFlags : u32 {
        /// Enabled
        ///
        /// If this bit is set the processor is ready for use. If this bit is clear and the
        /// Online Capable bit is set, system hardware supports enabling this processor during
        /// OS runtime. If this bit is clear and the Online Capable bit is also clear, this
        /// processor is unusable, and OSPM shall ignore the contents of the [`ProcessorLocalApic`]
        /// structure.
        const ENABLED = 1 << 0;
        const PERFORMANCE_INTERRUPT_MODE = 1 << 1;
        const VGIC_MAINTENANCE_INTERRUPT_MODE_FLAGS = 1 << 2;
        /// Online Capable
        ///
        /// The information conveyed by this bit depends on the value of the Enabled bit. If
        /// the Enabled bit is set, this bit is reserved and must be zero. Otherwise, if this
        /// bit is set, system hardware supports enabling this processor during OS runtime.
        const ONLINE_CAPABLE = 1 << 3;
    }
}

impl GicCpuInterface {
    #[inline]
    pub fn acpi_processor_uid(&self) -> u32 {
        self.acpi_processor_uid.get()
    }
}

#[repr(C, packed)]
pub struct GicDistributor {
    header: Header,
    reserved0: [u8; 2],
    gic_id: u32_le,
    phys_base_addr: u64_le,
    system_vector_base: u32_le,
    gic_version: u8,
    reserved: [u8; 3],
}

#[repr(C, packed)]
pub struct GicMsiFrame {
    header: Header,
    reserved0: [u8; 2],
    gic_msi_frame_id: u32_le,
    phys_base_addr: u64_le,
    flags: u32_le,
    spi_count: u16_le,
    spi_base: u16_le,
}

#[repr(C, packed)]
pub struct GicRedistributor {
    header: Header,
    reserved0: [u8; 2],
    discovery_range_base_addr: u64_le,
    discovery_range_length: u32_le,
}

#[repr(C, packed)]
pub struct GicInterruptTranslationService {
    header: Header,
    reserved0: [u8; 2],
    gic_its_id: u32_le,
    phys_base_addr: u64_le,
    reserved: [u8; 4],
}

#[repr(C, packed)]
pub struct MultiprocessorWakeup {
    header: Header,
    mailbox_version: u16_le,
    reserved0: [u8; 4],
    mailbox_addr: u64_le,
}

/// RISC-V Hart-Local Interrupt Controller
#[repr(C, packed)]
pub struct RiscvIntc {
    header: Header,
    version: u8,
    reserved: u8,
    flags: u32_le,
    hartid: u64_le,
    acpi_processor_uid: u32_le,
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    pub struct RiscvIntcFlags : u32 {
        /// Enabled
        ///
        /// If this bit is set the processor is ready for use. If this bit is clear and the
        /// Online Capable bit is set, system hardware supports enabling this processor during
        /// OS runtime. If this bit is clear and the Online Capable bit is also clear, this
        /// processor is unusable, and OSPM shall ignore the contents of the [`RiscvIntc`]
        /// structure.
        const ENABLED = 1 << 0;
        /// Online Capable
        ///
        /// The information conveyed by this bit depends on the value of the Enabled bit. If
        /// the Enabled bit is set, this bit is reserved and must be zero. Otherwise, if this
        /// bit is set, system hardware supports enabling this processor during OS runtime.
        const ONLINE_CAPABLE = 1 << 1;
    }
}

impl RiscvIntc {
    #[inline]
    pub fn flags(&self) -> RiscvIntcFlags {
        RiscvIntcFlags::from_bits_retain(self.flags.get())
    }

    /// Returns the hartid for the hart this interrupt controller belongs to
    #[inline]
    pub fn hartid(&self) -> u64 {
        self.hartid.get()
    }

    /// Returns the ACPI Processor UID for the hart this interrupt controller belongs to
    #[inline]
    pub fn acpi_processor_uid(&self) -> u32 {
        self.acpi_processor_uid.get()
    }

    #[inline]
    pub fn is_startable(&self) -> bool {
        self.flags()
            .intersects(RiscvIntcFlags::ENABLED | RiscvIntcFlags::ONLINE_CAPABLE)
    }
}
