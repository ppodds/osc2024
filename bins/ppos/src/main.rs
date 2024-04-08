#![no_std]
#![no_main]

extern crate alloc;

mod driver;
mod exception;
mod memory;
mod shell;

use core::{arch::global_asm, panic::PanicInfo};
use cpu::cpu::{enable_kernel_space_interrupt, switch_to_el1};
use library::println;
use memory::page_allocator::page_allocator;
use shell::Shell;

global_asm!(include_str!("boot.s"));

#[no_mangle]
pub unsafe extern "C" fn _start_rust(devicetree_start_addr: usize) -> ! {
    memory::DEVICETREE_START_ADDR = devicetree_start_addr;
    // switch to EL1 and jump to kernel_init
    switch_to_el1(
        &memory::__phys_binary_load_addr as *const usize as u64,
        kernel_init,
    );
}

unsafe fn kernel_init() -> ! {
    exception::handling_init();
    driver::init().unwrap();
    enable_kernel_space_interrupt();
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
    kernel_start();
}

fn kernel_start() -> ! {
    let mut shell = Shell::new();
    shell.run();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("{}", _info);
    loop {}
}
