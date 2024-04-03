use core::{array, slice};

use alloc::{collections::LinkedList, vec::Vec};
use library::{println, sync::mutex::Mutex};

use crate::memory::PAGE_SIZE;

pub unsafe trait PageAllocator {
    unsafe fn init(&self, page_start_addr: usize, page_end_addr: usize)
        -> Result<(), &'static str>;

    /**
     * Allocate a large size memory
     * size should be align to page size
     */
    unsafe fn alloc_page(&self, size: usize) -> Result<usize, &'static str>;

    unsafe fn free_page(&self, page_start_addr: usize);
}

struct NullPageAllocator {}

impl NullPageAllocator {
    const fn new() -> Self {
        Self {}
    }
}

unsafe impl PageAllocator for NullPageAllocator {
    unsafe fn init(
        &self,
        page_start_addr: usize,
        page_end_addr: usize,
    ) -> Result<(), &'static str> {
        unimplemented!();
    }

    unsafe fn alloc_page(&self, size: usize) -> Result<usize, &'static str> {
        unimplemented!();
    }

    unsafe fn free_page(&self, page_start_addr: usize) {
        unimplemented!();
    }
}

#[derive(Debug)]
pub struct BuddyPageAllocator {
    // workaround
    frame_free_list: Mutex<Option<[LinkedList<usize>; Self::MAX_PAGE_VAL + 1]>>,
    status: Mutex<Vec<usize>>,
    boundary: Mutex<(usize, usize)>,
}

impl BuddyPageAllocator {
    const MAX_PAGE_VAL: usize = 15;
    const FREE_VAL: usize = usize::MAX - 1;
    const ALLOCATED_VAL: usize = usize::MAX - 2;

    pub const fn new() -> Self {
        Self {
            frame_free_list: Mutex::new(None),
            status: Mutex::new(Vec::new()),
            boundary: Mutex::new((0, 0)),
        }
    }

    #[inline(always)]
    fn is_align_to_page_size(addr: usize) -> bool {
        addr % Self::page_size() == 0
    }

    #[inline(always)]
    fn page_size() -> usize {
        unsafe { &PAGE_SIZE as *const usize as usize }
    }

    #[inline(always)]
    fn biggest_part_frame_amount() -> usize {
        1 << Self::MAX_PAGE_VAL
    }

    #[inline(always)]
    fn biggest_part_size() -> usize {
        Self::page_size() * Self::biggest_part_frame_amount()
    }

    #[inline(always)]
    fn find_buddy_index(frame_index: usize, frame_val: usize) -> usize {
        frame_index ^ (1 << frame_val)
    }
}

unsafe impl PageAllocator for BuddyPageAllocator {
    unsafe fn init(
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
        let mut frame_free_list = array::from_fn(|_| LinkedList::new());
        let memory_total_size = page_end_addr - page_start_addr;
        println!(
            "Page allocatable memory total size: {} (reserved zone included)",
            memory_total_size
        );
        println!("Page size: {}", Self::page_size());
        let frame_amount = memory_total_size / Self::page_size();
        println!("Frame amount: {}", frame_amount);
        let mut status = self.status.lock().unwrap();
        status.resize(frame_amount, Self::FREE_VAL);
        let biggest_part_page_amount = Self::biggest_part_frame_amount();
        let biggest_part_size = Self::biggest_part_size();
        println!("Biggest part size: {}", biggest_part_size);
        // memory size is not align to biggest part size is not take into account
        let frame_amount_of_biggest_part = frame_amount / biggest_part_page_amount;
        println!("Biggest part amount: {}", frame_amount_of_biggest_part);
        for i in 0..frame_amount_of_biggest_part {
            let page_frame_index = i * biggest_part_page_amount;
            frame_free_list[Self::MAX_PAGE_VAL].push_back(page_frame_index);
            println!(
                "Add page frame {} into free list of value {}",
                page_frame_index,
                Self::MAX_PAGE_VAL
            );
            status[page_frame_index] = biggest_part_page_amount - 1;
        }
        *self.frame_free_list.lock().unwrap() = Some(frame_free_list);
        Ok(())
    }

    /**
     * Allocate a large size memory
     * size should be align to page size
     */
    unsafe fn alloc_page(&self, size: usize) -> Result<usize, &'static str> {
        let page_size = unsafe { &PAGE_SIZE as *const usize as usize };
        if size % page_size != 0 {
            return Err("Allocate size not align to page size");
        }
        if size > Self::biggest_part_size() {
            return Err("Request size is too big");
        }
        let request_frame_amount = size / Self::page_size();
        let request_val = request_frame_amount.ilog2();
        let mut frame_free_list = self.frame_free_list.lock().unwrap();
        let mut request_node_val = request_val as usize;
        while request_node_val <= Self::MAX_PAGE_VAL {
            if !frame_free_list.as_ref().unwrap()[request_node_val].is_empty() {
                break;
            }
            if request_node_val == Self::MAX_PAGE_VAL {
                return Err("There is no enough memory for now");
            }
            request_node_val += 1;
        }
        println!("Request node value: {}", request_node_val);
        let index = frame_free_list.as_mut().unwrap()[request_node_val]
            .pop_front()
            .unwrap();
        let mut status = self.status.lock().unwrap();
        status[index] = Self::ALLOCATED_VAL;
        // free redundant
        let mut release_node_frame_amount = (1 << request_node_val) >> 1;
        let mut release_node_val = request_node_val - 1;
        while release_node_frame_amount > request_frame_amount {
            let release_index = index + release_node_frame_amount;
            status[release_index] = release_node_val;
            frame_free_list.as_mut().unwrap()[release_node_val].push_back(release_index);
            println!(
                "Release frame {} and set value = {}",
                release_index, release_node_val
            );
            release_node_val -= 1;
            release_node_frame_amount >>= 1;
        }
        let allocate_addr = self.boundary.lock().unwrap().0 + index * Self::page_size();
        println!("Allocate start addr: {:#18x}", allocate_addr);
        Ok(allocate_addr)
    }

    unsafe fn free_page(&self, page_start_addr: usize) {
        todo!()
    }
}

pub fn register_page_allocator(page_allocator: &'static (dyn PageAllocator + Sync)) {
    *PAGE_ALLOCATOR.lock().unwrap() = page_allocator;
}

pub fn page_allocator() -> &'static (dyn PageAllocator + Sync) {
    *PAGE_ALLOCATOR.lock().unwrap()
}

static PAGE_ALLOCATOR: Mutex<&'static (dyn PageAllocator + Sync)> =
    Mutex::new(&NullPageAllocator::new());
