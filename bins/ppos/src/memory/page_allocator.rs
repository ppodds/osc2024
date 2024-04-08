use core::{alloc::GlobalAlloc, array};

use alloc::vec::Vec;
use library::{println, sync::mutex::Mutex};

use super::{page_size, round_up};

type FrameIndex = usize;
// simulate linked list with Vec which is better for cache
type OrderFreeList = Vec<FrameIndex>;
type FrameFreeList = [OrderFreeList; BuddyPageAllocator::MAX_ORDER + 1];

#[derive(Debug)]
pub struct BuddyPageAllocator {
    // workaround
    frame_free_list: Mutex<Option<FrameFreeList>>,
    boundary: Mutex<(usize, usize)>,
}

impl BuddyPageAllocator {
    const MAX_ORDER: usize = 11;

    pub const fn new() -> Self {
        Self {
            frame_free_list: Mutex::new(None),
            boundary: Mutex::new((0, 0)),
        }
    }

    #[inline(always)]
    fn is_align_to_page_size(addr: usize) -> bool {
        addr % page_size() == 0
    }

    #[inline(always)]
    fn biggest_part_frame_amount() -> usize {
        1 << Self::MAX_ORDER
    }

    #[inline(always)]
    fn biggest_part_size() -> usize {
        page_size() * Self::biggest_part_frame_amount()
    }

    #[inline(always)]
    fn find_buddy_index(frame_index: usize, frame_order: usize) -> usize {
        frame_index ^ (1 << frame_order)
    }

    fn merge_buddy(&self, frame_index: usize, frame_order: usize) {
        if frame_order == Self::MAX_ORDER {
            return;
        }
        let buddy_index = Self::find_buddy_index(frame_index, frame_order);
        println!("buddy index: {}", buddy_index);
        let mut frame_free_list = self.frame_free_list.lock().unwrap();
        let list_ins = &mut frame_free_list.as_mut().unwrap()[frame_order];
        let mut buddy_index_in_free_list = 0;
        // remove buddy from free list
        let mut found = false;
        for i in 0..list_ins.len() {
            if list_ins[i] == buddy_index {
                buddy_index_in_free_list = i;
                // list_ins.swap_remove(buddy_index_in_free_list);
                found = true;
                break;
            }
        }
        if found {
            list_ins.swap_remove(buddy_index_in_free_list);
            println!("Remove buddy from free list. buddy index: {}", buddy_index);
            println!(
                "Merge frame {} into frame {}. New val: {}",
                buddy_index,
                frame_index,
                frame_order + 1
            );
            // if merge success, merge buddy again
            self.merge_buddy(frame_index, frame_order + 1);
        }
    }

    pub unsafe fn init(
        &self,
        page_start_addr: usize,
        page_end_addr: usize,
    ) -> Result<(), &'static str> {
        if !Self::is_align_to_page_size(page_start_addr)
            || !Self::is_align_to_page_size(page_end_addr)
        {
            return Err("Page start / end address should be align to page size");
        }
        *self.boundary.lock().unwrap() = (page_start_addr, page_end_addr);
        let mut frame_free_list = array::from_fn(|_| Vec::new());
        let memory_total_size = page_end_addr - page_start_addr;
        println!(
            "Page allocatable memory total size: {} (reserved zone included)",
            memory_total_size
        );
        println!("Page size: {}", page_size());
        let frame_amount = memory_total_size / page_size();
        println!("Frame amount: {}", frame_amount);
        let biggest_part_page_amount = Self::biggest_part_frame_amount();
        let biggest_part_size = Self::biggest_part_size();
        println!("Biggest part size: {}", biggest_part_size);
        // memory size is not align to biggest part size is not take into account
        let frame_amount_of_biggest_part = frame_amount / biggest_part_page_amount;
        println!("Biggest part amount: {}", frame_amount_of_biggest_part);
        for i in 0..frame_amount_of_biggest_part {
            let page_frame_index = i * biggest_part_page_amount;
            frame_free_list[Self::MAX_ORDER].push(page_frame_index);
            println!(
                "Add page frame {} into free list of order {}",
                page_frame_index,
                Self::MAX_ORDER
            );
        }
        *self.frame_free_list.lock().unwrap() = Some(frame_free_list);
        Ok(())
    }

    /**
     * Allocate a large size memory
     * size should be align to page size
     */
    pub unsafe fn alloc_page(&self, order: usize) -> Result<usize, &'static str> {
        if order > Self::MAX_ORDER {
            return Err("Request order is too big");
        }
        let request_frame_amount = 1 << order;
        let mut frame_free_list = self.frame_free_list.lock().unwrap();
        let mut request_node_order = order;
        while request_node_order <= Self::MAX_ORDER {
            if !frame_free_list.as_ref().unwrap()[request_node_order].is_empty() {
                break;
            }
            if request_node_order == Self::MAX_ORDER {
                return Err("There is no enough memory for now");
            }
            request_node_order += 1;
        }
        println!("Request node order: {}", request_node_order);
        let free_frame_index = frame_free_list.as_mut().unwrap()[request_node_order]
            .pop()
            .unwrap();
        // free redundant
        let mut release_node_frame_amount = 1 << request_node_order;
        let mut release_node_order = request_node_order;
        while release_node_frame_amount > request_frame_amount {
            release_node_order -= 1;
            release_node_frame_amount >>= 1;
            let release_index = free_frame_index + release_node_frame_amount;
            frame_free_list.as_mut().unwrap()[release_node_order].push(release_index);
            println!(
                "Release frame {} and set order = {}",
                release_index, release_node_order
            );
        }
        let allocate_addr = self.boundary.lock().unwrap().0 + free_frame_index * page_size();
        println!(
            "Allocate start addr: {:#18x}, frame index: {}",
            allocate_addr, free_frame_index
        );
        Ok(allocate_addr)
    }

    pub unsafe fn free_page(
        &self,
        page_start_addr: usize,
        order: usize,
    ) -> Result<(), &'static str> {
        let page_size = page_size();
        if page_start_addr % page_size != 0 {
            return Err("Page start address should align to page size");
        }
        let frame_index = (page_start_addr - self.boundary.lock().unwrap().0) / page_size;
        let max_frame_index =
            ((self.boundary.lock().unwrap().1 - self.boundary.lock().unwrap().0) / page_size) - 1;
        if frame_index > max_frame_index {
            return Err("Provide page start address is not a valid frame start address");
        }
        // merge free block
        self.merge_buddy(frame_index, order);
        self.frame_free_list.lock().unwrap().as_mut().unwrap()[order].push(frame_index);
        println!(
            "Frame {} has been freed(addr: {:#08x})",
            frame_index, page_start_addr
        );
        Ok(())
    }

    #[inline(always)]
    fn get_order_from_layout(layout: core::alloc::Layout) -> usize {
        let size = round_up(layout.size());
        (size / page_size()).ilog2() as usize
    }
}

unsafe impl GlobalAlloc for BuddyPageAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.alloc_page(Self::get_order_from_layout(layout))
            .unwrap() as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.free_page(ptr as usize, Self::get_order_from_layout(layout))
            .unwrap();
    }
}

pub fn page_allocator() -> &'static BuddyPageAllocator {
    *PAGE_ALLOCATOR.lock().unwrap()
}

static BUDDY_PAGE_ALLOCATOR: BuddyPageAllocator = BuddyPageAllocator::new();

static PAGE_ALLOCATOR: Mutex<&'static BuddyPageAllocator> = Mutex::new(&BUDDY_PAGE_ALLOCATOR);
