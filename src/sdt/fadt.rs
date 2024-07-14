use libsa::endian::{u16_le, u32_le, u64_le};

use crate::Sdt;

/// Fixed ACPI Description Table
#[repr(C, packed)]
pub struct Fadt {
    pub header: super::Header,
    pub firmware_ctrl: u32_le,
    pub dsdt: u32_le,
    pub reserved0: u8,
    pub preferred_pm_profile: u8,
    pub sci_int: u16,
    pub smi_cmd: u32_le,
    pub acpi_enable: u8,
    pub acpi_disable: u8,
    pub s4bios_req: u8,
    pub pstate_cnt: u8,
    pub pm1a_evt_blk: u32_le,
    pub pm1b_evt_blk: u32_le,
    pub pm1a_cnt_blk: u32_le,
    pub pm1b_cnt_blk: u32_le,
    pub pm2_cnt_blk: u32_le,
    pub pm_tmr_blk: u32_le,
    pub gpe0_blk: u32_le,
    pub gpe1_blk: u32_le,
    pub pm1_evt_len: u8,
    pub pm1_cnt_len: u8,
    pub pm2_cnt_len: u8,
    pub pm_tmr_len: u8,
    pub gpe0_blk_len: u8,
    pub gpe1_blk_len: u8,
    pub gpe1_base: u8,
    pub cst_cnt: u8,
    pub p_lvl2_lat: u16_le,
    pub p_lvl3_lat: u16_le,
    pub flush_size: u16_le,
    pub flush_stride: u16_le,
    pub duty_offset: u8,
    pub duty_width: u8,
    pub day_alrm: u8,
    pub mon_alrm: u8,
    pub century: u8,
    pub iapc_boot_arch: [u8; 2],
    pub reserved1: u8,
    pub flags: u32_le,
    pub reset_reg: [u8; 12],
    pub reset_value: u8,
    pub arm_boot_arch: [u8; 2],
    pub fadt_minor_version: u8,
    pub x_firmware_ctrl: u64_le,
    pub x_dsdt: u64_le,
    pub x_pm1a_evt_blk: [u8; 12],
    pub x_pm1b_evt_blk: [u8; 12],
    pub x_pm1a_cnt_blk: [u8; 12],
    pub x_pm1b_cnt_blk: [u8; 12],
    pub x_pm2_cnt_blk: [u8; 12],
    pub x_pm_tmr_blk: [u8; 12],
    pub x_gpe0_blk: [u8; 12],
    pub x_gpe1_blk: [u8; 12],
    pub sleep_control_reg: [u8; 12],
    pub sleep_status_reg: [u8; 12],
    pub hypervisor_vendor_identity: u64_le,
}

unsafe impl Sdt for Fadt {
    const SIGNATURE: super::Signature = super::Signature(*b"FACP");

    fn header(&self) -> &super::Header {
        &self.header
    }

    unsafe fn from_header_ptr(ptr: *const super::Header) -> *const Self {
        ptr.cast()
    }
}

impl Fadt {
    pub fn flags(&self) -> FadtFlags {
        FadtFlags::from_bits_retain(self.flags.get())
    }

    pub fn dsdt(&self) -> u64 {
        if self.header.revision >= 2 {
            self.x_dsdt.get()
        } else {
            self.dsdt.get() as u64
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    pub struct FadtFlags : u32 {
        const WBINVD = 1 << 0;
        const WBINVD_FLUSH = 1 << 1;
        const PROC_C1 = 1 << 2;
        const P_LVL2_UP = 1 << 3;
        const PWR_BUTTON = 1 << 4;
        const SLP_BUTTON = 1 << 5;
        const FIX_RTC = 1 << 6;
        const RTC_S4 = 1 << 7;
        const TMR_VAL_EXT = 1 << 8;
        const DCK_CAP = 1 << 9;
        const RESET_REG_SUP = 1 << 10;
        const SEALED_CASE = 1 << 11;
        const HEADLESS = 1 << 12;
        const CPU_SW_SLP = 1 << 13;
        const PCI_EXP_WAK = 1 << 14;
        const USE_PLATFORM_CLOCK = 1 << 15;
        const S4_RTC_STS_VALID = 1 << 16;
        const REMOTE_POWER_ON_CAPABLE = 1 << 17;
        const FORCE_APIC_CLUSTER_MODEL = 1 << 18;
        const FORCE_APIC_PHYSICAL_DESTINATION_MODE = 1 << 19;
        const HW_REDUCED_ACPI = 1 << 20;
        const LOW_POWER_S0_IDLE_CAPABLE = 1 << 21;
        const PERSISTENT_CPU_CACHES_0 = 1 << 22;
        const PERSISTENT_CPU_CACHES_1 = 1 << 23;
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PreferredPmProfile {
    Desktop,
    Mobile,
    Workstation,
    EnterpriseServer,
    SohoServer,
    AppliancePc,
    PerformanceServer,
    Tablet,
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    pub struct IapcBootFlags : u16 {
        const LEGACY_DEVICES = 1 << 0;
        const I8042 = 1 << 1;
        const VGA_NOT_PRESENT = 1 << 2;
        const MSI_NOT_SUPPORTED = 1 << 3;
        const PCIE_ASPM_CONTROLS = 1 << 4;
        const CMOS_RTC_NOT_PRESENT = 1 << 5;
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    pub struct ArmBootFlags : u16 {
        const PSCI_COMPLIANT = 1 << 0;
        const PSCI_USE_HVC = 1 << 1;
    }
}

/// Firmware ACPI Control Structure
#[repr(C, packed)]
pub struct Facs {
    pub signature: [u8; 4],
    pub length: u32,
    pub hardware_signature: u64,
    pub firmware_waking_vector: [u8; 12],
    pub global_lock: u32,
    pub flags: FacsFlags,
    pub x_firmware_waking_vector: u64,
    pub version: u8,
    pub reserved0: [u8; 3],
    pub ospm_flags: OspmFlags,
    pub reserved1: [u8; 24],
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    pub struct FacsFlags : u32 {
        const S4BIOS_F = 1 << 0;
        const WAKE_64BIT_SUPPORTED_F = 1 << 1;
    }

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    pub struct OspmFlags : u32 {
        const WAKE_64BIT_F = 1 << 0;
    }
}
