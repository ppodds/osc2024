use core::{alloc::GlobalAlloc, mem::align_of};

use library::sync::mutex::Mutex;

use super::{heap_end_addr, heap_start_addr};

#[global_allocator]
static KERNEL_HEAP_ALLOCATOR: HeapAllocator = unsafe { HeapAllocator::new() };

struct HeapAllocatorInner {
    current: usize,
}

impl HeapAllocatorInner {
    const unsafe fn new() -> Self {
        Self { current: 0 }
    }

    unsafe fn alloc(&mut self, layout: core::alloc::Layout) -> *mut u8 {
        if heap_start_addr() + self.current + layout.size() > heap_end_addr() {
            panic!(
                "Heap memory is not enough to allocate {} bytes",
                layout.size()
            );
        }
        let p = (heap_start_addr() + self.current) as *mut u8;
        self.current += layout.size();
        // align to 8 bytes
        self.current += (self.current as *const u8).align_offset(align_of::<u64>());
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {}
}

pub struct HeapAllocator {
    inner: Mutex<HeapAllocatorInner>,
}

impl HeapAllocator {
    pub const unsafe fn new() -> Self {
        Self {
            inner: Mutex::new(HeapAllocatorInner::new()),
        }
    }
}

unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut inner = self.inner.lock().unwrap();
        inner.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let inner = self.inner.lock().unwrap();
        inner.dealloc(ptr, layout);
    }
}
