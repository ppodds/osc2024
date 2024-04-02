use library::sync::mutex::Mutex;

use crate::interrupt_controller::InterruptNumber;

static CUR_INTERRUPT_MANAGER: Mutex<
    &'static (dyn InterruptManager<InterruptNumberType = InterruptNumber> + Sync),
> = Mutex::new(&NULL_INTERRUPT_MANAGER);
static NULL_INTERRUPT_MANAGER: NullInterruptManager = NullInterruptManager {};

struct NullInterruptManager {}

impl InterruptManager for NullInterruptManager {
    type InterruptNumberType = InterruptNumber;

    fn register_handler(
        &self,
        _: InterruptHandlerDescriptor<Self::InterruptNumberType>,
    ) -> Result<(), &'static str> {
        unimplemented!()
    }

    fn enable(&self, _: &Self::InterruptNumberType) {
        unimplemented!()
    }

    fn handle_pending_interrupt(&self) {
        unimplemented!()
    }
}

pub fn register_interrupt_manager(
    new_manager: &'static (dyn InterruptManager<InterruptNumberType = InterruptNumber> + Sync),
) {
    let mut cur_interrupt_manager = CUR_INTERRUPT_MANAGER.lock().unwrap();
    *cur_interrupt_manager = new_manager;
}

pub fn interrupt_manager(
) -> &'static (dyn InterruptManager<InterruptNumberType = InterruptNumber> + Sync) {
    *CUR_INTERRUPT_MANAGER.lock().unwrap()
}

pub trait InterruptHandler {
    /// Called when the corresponding interrupt is asserted.
    fn handle(&self) -> Result<(), &'static str>;
}

#[derive(Copy, Clone)]
pub struct InterruptHandlerDescriptor<T>
where
    T: Copy,
{
    /// The interrupt number.
    number: T,

    /// Descriptive name.
    name: &'static str,

    /// Reference to handler trait object.
    handler: &'static (dyn InterruptHandler + Sync + Send),
}

impl<T> InterruptHandlerDescriptor<T>
where
    T: Copy,
{
    /// Create an instance.
    pub const fn new(
        number: T,
        name: &'static str,
        handler: &'static (dyn InterruptHandler + Sync + Send),
    ) -> Self {
        Self {
            number,
            name,
            handler,
        }
    }

    /// Return the number.
    pub const fn number(&self) -> T {
        self.number
    }

    /// Return the name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Return the handler.
    pub const fn handler(&self) -> &'static (dyn InterruptHandler + Sync + Send) {
        self.handler
    }
}

pub trait InterruptManager {
    type InterruptNumberType: Copy;

    fn register_handler(
        &self,
        interrupt_handler_descriptor: InterruptHandlerDescriptor<Self::InterruptNumberType>,
    ) -> Result<(), &'static str>;

    fn enable(&self, interrupt_number: &Self::InterruptNumberType);

    fn handle_pending_interrupt(&self);
}
