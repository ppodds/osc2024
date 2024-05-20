use alloc::{rc::Rc, vec::Vec};
use library::sync::mutex::Mutex;
use tock_registers::register_bitfields;

use super::page::{MemoryAccessPermission, MemoryAttribute, PageTable};

register_bitfields! [
    u32,
    pub PROTECTION [
        PROT_NONE OFFSET(3) NUMBITS(1) [],
        PROT_READ OFFSET(2) NUMBITS(1) [],
        PROT_WRITE OFFSET(1) NUMBITS(1) [],
        PROT_EXEC OFFSET(0) NUMBITS(1) [],
    ],
];

#[derive(Debug, Clone)]
pub struct MemoryMappingInfo {
    virt_addr: usize,
    phys_addr: usize,
    protection: u32,
    size: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryMapping {
    /// list of memory mapping info
    memory_mapping_info_list: Vec<MemoryMappingInfo>,
    /// user space page table
    page_table: Rc<Mutex<PageTable>>,
}

impl MemoryMapping {
    pub fn new() -> Self {
        Self {
            memory_mapping_info_list: Vec::new(),
            page_table: Rc::new(Mutex::new(PageTable::new())),
        }
    }

    #[inline(always)]
    pub fn page_table_phys_base_address(&self) -> u64 {
        self.page_table.lock().unwrap().phys_base_address()
    }

    pub fn map_pages(
        &mut self,
        virt_addr: usize,
        phys_addr: usize,
        size: usize,
        memory_attribute: MemoryAttribute,
        access_permission: MemoryAccessPermission,
    ) -> Result<(), &'static str> {
        self.page_table.lock().unwrap().map_pages(
            virt_addr,
            phys_addr,
            size,
            memory_attribute,
            access_permission,
        )
    }

    pub fn unmap_pages(&mut self, virt_addr: usize, size: usize) -> Result<(), &'static str> {
        self.page_table.lock().unwrap().unmap_pages(virt_addr, size)
    }
}
