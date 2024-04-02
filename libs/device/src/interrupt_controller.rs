use crate::local_interrupt_controller::LocalInterruptController;
use crate::{
    common::BoundedUsize, interrupt_manager::InterruptHandlerDescriptor,
    peripheral_ic::PeripheralIC,
};
use crate::{device_driver::DeviceDriver, interrupt_manager::InterruptManager};

pub type LocalInterrupt = BoundedUsize<0, { InterruptController::MAX_LOCAL_INTERRUPT_NUMBER }>;
pub type PeripheralInterrupt = BoundedUsize<
    { InterruptController::MAX_LOCAL_INTERRUPT_NUMBER + 1 },
    { InterruptController::MAX_PERIPHERAL_INTERRUPT_NUMBER },
>;

#[derive(Copy, Clone)]
#[repr(usize)]
pub enum LocalInterruptType {
    Timer1 = 1,
    Timer3 = 3,
}

#[derive(Copy, Clone)]
#[repr(usize)]
pub enum PeripherialInterruptType {
    Aux = 29,
}

#[derive(Copy, Clone)]
pub enum InterruptNumber {
    Local(LocalInterrupt),
    Peripheral(PeripheralInterrupt),
}

impl core::fmt::Display for InterruptNumber {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InterruptNumber::Local(n) => write!(f, "Local interrupt {}", n),
            InterruptNumber::Peripheral(n) => write!(f, "Peripheral interrupt {}", n),
        }
    }
}

pub struct PendingInterrupts {
    bitmask: u64,
}

impl PendingInterrupts {
    pub fn new(bitmask: u64) -> Self {
        Self { bitmask }
    }
}

impl Iterator for PendingInterrupts {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bitmask == 0 {
            return None;
        }

        let next = self.bitmask.trailing_zeros() as usize;
        self.bitmask &= self.bitmask.wrapping_sub(1);
        Some(next)
    }
}

pub struct InterruptController {
    peripheral_ic: PeripheralIC,
    local_interrupt_controller: LocalInterruptController,
}

impl InterruptController {
    pub const MAX_LOCAL_INTERRUPT_NUMBER: usize = 3;
    pub const MAX_PERIPHERAL_INTERRUPT_NUMBER: usize = 63;

    pub const unsafe fn new(
        peripherial_ic_mmio_start_addr: usize,
        core_interrupt_source_mmio_start_addr: usize,
    ) -> Self {
        Self {
            peripheral_ic: PeripheralIC::new(peripherial_ic_mmio_start_addr),
            local_interrupt_controller: LocalInterruptController::new(
                core_interrupt_source_mmio_start_addr,
            ),
        }
    }
}

impl DeviceDriver for InterruptController {
    type InterruptNumberType = InterruptNumber;
}

impl InterruptManager for InterruptController {
    type InterruptNumberType = InterruptNumber;

    fn register_handler(
        &self,
        interrupt_handler_descriptor: InterruptHandlerDescriptor<Self::InterruptNumberType>,
    ) -> Result<(), &'static str> {
        match interrupt_handler_descriptor.number() {
            InterruptNumber::Local(local_interrupt_number) => self
                .local_interrupt_controller
                .register_handler(InterruptHandlerDescriptor::new(
                    local_interrupt_number,
                    interrupt_handler_descriptor.name(),
                    interrupt_handler_descriptor.handler(),
                )),
            InterruptNumber::Peripheral(peripheral_interrupt_number) => self
                .peripheral_ic
                .register_handler(InterruptHandlerDescriptor::new(
                    peripheral_interrupt_number,
                    interrupt_handler_descriptor.name(),
                    interrupt_handler_descriptor.handler(),
                )),
        }
    }

    fn enable(&self, interrupt_number: &Self::InterruptNumberType) {
        match interrupt_number {
            InterruptNumber::Local(n) => self.local_interrupt_controller.enable(n),
            InterruptNumber::Peripheral(n) => self.peripheral_ic.enable(n),
        };
    }

    fn handle_pending_interrupt(&self) {
        self.local_interrupt_controller.handle_pending_interrupt();
        self.peripheral_ic.handle_pending_interrupt();
    }
}
