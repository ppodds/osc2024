/*
* Usage:
*    unsafe {
*        println!(
*            "{:#08x}",
*            &memory::__phys_dram_start_addr as *const usize as usize
*        );
*        println!(
*            "{:#08x}",
*            &memory::__phys_binary_load_addr as *const usize as usize
*        );
*        println!("{:#08x}", &memory::__bss_begin as *const usize as usize);
*        println!("{:#08x}", &memory::__bss_end as *const usize as usize);
*    }
*/

use crate::memory;
use crate::memory::page_allocator::page_allocator;

extern "C" {
    pub static __phys_dram_start_addr: usize;
    pub static __phys_binary_load_addr: usize;
    pub static __bss_begin: usize;
    pub static __bss_end: usize;
    pub static __heap_begin: usize;
    pub static __heap_end: usize;
    pub static PAGE_SIZE: usize;
}

pub static mut DEVICETREE_START_ADDR: usize = 0;

#[inline(always)]
pub fn round_up_with(v: usize, s: usize) -> usize {
    (v + s - 1) & !(s - 1)
}

#[inline(always)]
pub fn round_up(addr: usize) -> usize {
    round_up_with(addr, page_size())
}

#[inline(always)]
pub fn page_size() -> usize {
    unsafe { &PAGE_SIZE as *const usize as usize }
}

#[inline(always)]
pub fn phys_dram_start_addr() -> usize {
    unsafe { &__phys_dram_start_addr as *const usize as usize }
}

#[inline(always)]
unsafe fn heap_start_addr() -> usize {
    &__heap_begin as *const usize as usize
}

#[inline(always)]
unsafe fn heap_end_addr() -> usize {
    &__heap_end as *const usize as usize
}

pub unsafe fn init_allocator() {
    static mut CPIO_START_ADDR: usize = 0;
    static mut CPIO_END_ADDR: usize = 0;
    // initialize memory allocator first
    let mut devicetree =
        unsafe { devicetree::FlattenedDevicetree::from_memory(memory::DEVICETREE_START_ADDR) };
    devicetree
        .traverse(&move |device_name, property_name, property_value| {
            if device_name == "memory@0" && property_name == "reg" {
                let memory_end_addr = u64::from_be_bytes(property_value.try_into().unwrap());
                page_allocator().init(0, memory_end_addr as usize)?;
            }
            Ok(())
        })
        .unwrap();
    // for device
    devicetree
        .traverse_reserved_memory(&|start, size| {
            page_allocator()
                .reserve_memory(start as usize, (start + size) as usize)
                .unwrap();
            Ok(())
        })
        .unwrap();
    let page_allocator = page_allocator();
    // kernel / stack / heap...
    page_allocator
        .reserve_memory(phys_dram_start_addr(), heap_end_addr())
        .unwrap();
    // CPIO archive
    devicetree
        .traverse(&|device_name, property_name, property_value| {
            if property_name == "linux,initrd-start" {
                CPIO_START_ADDR = u32::from_be_bytes(property_value.try_into().unwrap()) as usize;
                return Ok(());
            }
            Ok(())
        })
        .unwrap();
    devicetree
        .traverse(&|device_name, property_name, property_value| {
            if property_name == "linux,initrd-end" {
                CPIO_END_ADDR = u32::from_be_bytes(property_value.try_into().unwrap()) as usize;
                return Ok(());
            }
            Ok(())
        })
        .unwrap();
    page_allocator
        .reserve_memory(CPIO_START_ADDR, round_up(CPIO_END_ADDR))
        .unwrap();
    // device tree
    page_allocator
        .reserve_memory(
            memory::DEVICETREE_START_ADDR,
            round_up(memory::DEVICETREE_START_ADDR + devicetree.header().total_size() as usize),
        )
        .unwrap();
    // simple allocator
    page_allocator
        .reserve_memory(heap_start_addr(), heap_end_addr())
        .unwrap();
}

pub mod heap_allocator;
pub mod page_allocator;
pub mod slab_allocator;
