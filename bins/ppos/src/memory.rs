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
fn round_up_with(v: usize, s: usize) -> usize {
    (v + s - 1) & !(s - 1)
}

#[inline(always)]
fn round_up(addr: usize) -> usize {
    round_up_with(addr, page_size())
}

#[inline(always)]
fn page_size() -> usize {
    unsafe { &PAGE_SIZE as *const usize as usize }
}

pub mod heap_allocator;
pub mod page_allocator;
pub mod slab_allocator;
