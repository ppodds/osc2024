use crate::{
    driver::mailbox,
    memory::{paging::page::PageTable, phys_to_virt},
    scheduler::current,
};

pub fn mbox_call(channel: u8, mbox: *mut u32) -> i32 {
    mailbox().call(
        channel,
        phys_to_virt(
            PageTable::virt_to_phys_by_table(
                unsafe { &*current() }
                    .memory_mapping()
                    .page_table_phys_base_address() as *const PageTable,
                mbox as usize,
            )
            .unwrap(),
        ) as *mut u32,
    );
    1
}
