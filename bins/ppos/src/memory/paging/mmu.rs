use aarch64_cpu::{asm::barrier, registers::*};
use tock_registers::interfaces::ReadWriteable;

use super::page::FixedSizeTranslationTable;

pub struct MemoryManagementUnit {}

impl MemoryManagementUnit {
    pub const FIXED_SIZE_TRANSLATION_TABLE_ADDR: u64 = 0x1000;

    #[inline(always)]
    pub unsafe fn enable_mmu(&self) {
        self.setup_mair();
        self.setup_page_table();
        TTBR0_EL1.set(0x1000);
        TTBR1_EL1.set(0x1000);
        self.setup_translation_controll();
        barrier::isb(barrier::SY);
        self.enable_translation();
        barrier::isb(barrier::SY);
    }

    #[inline(always)]
    fn setup_mair(&self) {
        MAIR_EL1.modify(
            MAIR_EL1::Attr0_Device::nonGathering_nonReordering_noEarlyWriteAck
                + MAIR_EL1::Attr1_Normal_Outer::NonCacheable
                + MAIR_EL1::Attr1_Normal_Inner::NonCacheable,
        );
    }

    #[inline(always)]
    fn setup_translation_controll(&self) {
        TCR_EL1.modify(
            TCR_EL1::T0SZ.val(64 - 48)
                + TCR_EL1::T1SZ.val(64 - 48)
                + TCR_EL1::TG1::KiB_4
                + TCR_EL1::TG0::KiB_4,
        );
    }

    #[inline(never)]
    fn enable_translation(&self) {
        SCTLR_EL1.modify(SCTLR_EL1::M::Enable);
    }

    #[inline(always)]
    unsafe fn setup_page_table(&self) {
        core::ptr::write_volatile(
            0x1000 as *mut FixedSizeTranslationTable,
            FixedSizeTranslationTable::new(),
        );
    }
}

static MMU: MemoryManagementUnit = MemoryManagementUnit {};

pub fn mmu() -> &'static MemoryManagementUnit {
    &MMU
}
