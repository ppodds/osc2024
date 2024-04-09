use super::{
    heap_allocator::HeapAllocator, heap_end_addr, heap_start_addr,
    page_allocator::BuddyPageAllocator, phys_dram_start_addr, round_up,
    slab_allocator::SlabAllocator, CPIO_END_ADDR, CPIO_START_ADDR, DEVICETREE_START_ADDR,
};
use alloc::alloc::GlobalAlloc;

pub struct AllocatorWrapper {
    has_initialized: bool,
}

impl AllocatorWrapper {
    pub const unsafe fn new() -> Self {
        Self {
            has_initialized: false,
        }
    }

    pub unsafe fn init(&mut self) {
        // initialize buddy memory allocator first
        let mut devicetree = devicetree::FlattenedDevicetree::from_memory(DEVICETREE_START_ADDR);
        devicetree
            .traverse(&move |device_name, property_name, property_value| {
                if device_name == "memory@0" && property_name == "reg" {
                    let memory_end_addr = u64::from_be_bytes(property_value.try_into().unwrap());
                    BUDDY_PAGE_ALLOCATOR.init(0, memory_end_addr as usize)?;
                }
                Ok(())
            })
            .unwrap();
        // for device
        devicetree
            .traverse_reserved_memory(&|start, size| {
                BUDDY_PAGE_ALLOCATOR
                    .reserve_memory(start as usize, (start + size) as usize)
                    .unwrap();
                Ok(())
            })
            .unwrap();
        // kernel / stack / heap...
        BUDDY_PAGE_ALLOCATOR
            .reserve_memory(phys_dram_start_addr(), heap_end_addr())
            .unwrap();
        // CPIO archive
        devicetree
            .traverse(&|device_name, property_name, property_value| {
                if property_name == "linux,initrd-start" {
                    CPIO_START_ADDR =
                        u32::from_be_bytes(property_value.try_into().unwrap()) as usize;
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
        BUDDY_PAGE_ALLOCATOR
            .reserve_memory(CPIO_START_ADDR, round_up(CPIO_END_ADDR))
            .unwrap();
        // device tree
        BUDDY_PAGE_ALLOCATOR
            .reserve_memory(
                DEVICETREE_START_ADDR,
                round_up(DEVICETREE_START_ADDR + devicetree.header().total_size() as usize),
            )
            .unwrap();
        // simple allocator
        BUDDY_PAGE_ALLOCATOR
            .reserve_memory(heap_start_addr(), heap_end_addr())
            .unwrap();
        self.has_initialized = true;
    }
}

unsafe impl GlobalAlloc for AllocatorWrapper {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if self.has_initialized {
            SLAB_ALLOCATOR.alloc(layout)
        } else {
            HEAP_ALLOCATOR.alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let addr = ptr as usize;
        if addr >= heap_start_addr() && addr < heap_end_addr() {
            HEAP_ALLOCATOR.dealloc(ptr, layout)
        } else {
            SLAB_ALLOCATOR.dealloc(ptr, layout)
        }
    }
}

static HEAP_ALLOCATOR: HeapAllocator = unsafe { HeapAllocator::new() };
static BUDDY_PAGE_ALLOCATOR: BuddyPageAllocator = unsafe { BuddyPageAllocator::new() };
static SLAB_ALLOCATOR: SlabAllocator = unsafe { SlabAllocator::new(&BUDDY_PAGE_ALLOCATOR) };
