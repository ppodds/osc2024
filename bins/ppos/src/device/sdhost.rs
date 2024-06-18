use aarch64_cpu::registers::{Readable, Writeable};
use cpu::cpu::spin_for_cycle;
use library::sync::mutex::Mutex;
use tock_registers::{
    interfaces::ReadWriteable, register_bitfields, register_structs, registers::*,
};

use super::{
    common::MMIODerefWrapper, device_driver::DeviceDriver, interrupt_controller::InterruptNumber,
};

register_bitfields! [
    u32,
    SDHOST_CMD [
        NEW_CMD OFFSET(15) NUMBITS(1) [],
        BUSY OFFSET(11) NUMBITS(1) [],
        NO_RESPONSE OFFSET(10) NUMBITS(1) [],
        LONG_RESPONSE OFFSET(9) NUMBITS(1) [],
        WRITE OFFSET(7) NUMBITS(1) [],
        READ OFFSET(6) NUMBITS(1) [],
    ],
    SDHOST_CFG [
        DATA_EN OFFSET(4) NUMBITS(1) [],
        SLOW OFFSET(3) NUMBITS(1) [],
        INTBUS OFFSET(1) NUMBITS(1) [],
    ]
];

register_structs! {
    Registers {
        (0x00 => cmd: ReadWrite<u32, SDHOST_CMD::Register>),
        (0x04 => arg: ReadWrite<u32>),
        (0x08 => tout: ReadWrite<u32>),
        (0x0c => cdiv: ReadWrite<u32>),
        (0x10 => resp0: ReadWrite<u32>),
        (0x14 => resp1: ReadWrite<u32>),
        (0x18 => resp2: ReadWrite<u32>),
        (0x1c => resp3: ReadWrite<u32>),
        (0x20 => hsts: ReadWrite<u32>),
        (0x24 => _reserved1),
        (0x30 => pwr: ReadWrite<u32>),
        (0x34 => dbg: ReadWrite<u32>),
        (0x38 => cfg: ReadWrite<u32, SDHOST_CFG::Register>),
        (0x3c => size: ReadWrite<u32>),
        (0x40 => data: ReadWrite<u32>),
        (0x44 => _reserved2),
        (0x50 => cnt: ReadWrite<u32>),
        (0x54 => @END),
    }
}

#[repr(u32)]
enum SDCardCommand {
    GoIdleState = 0,
    SendOpCmd = 1,
    AllSendCid = 2,
    SendRelativeAddress = 3,
    SelectCard = 7,
    SendIfCondition = 8,
    StopTransmission = 12,
    SetBlockLength = 16,
    ReadSingleBlock = 17,
    WriteSingleBlock = 24,
    ApplicationCommand = 55,
}

struct SDHostInner {
    registers: MMIODerefWrapper<Registers>,
    is_hcs: bool,
}

impl SDHostInner {
    const SDHOST_NEW_CMD: u32 = 0x8000;

    const unsafe fn new(mmio_start_addr: usize) -> Self {
        Self {
            registers: MMIODerefWrapper::new(mmio_start_addr),
            is_hcs: false,
        }
    }

    fn setup(&mut self) {
        self.setup_for_sdhost();
        self.setup_for_sdcard();
    }

    fn setup_for_sdhost(&mut self) {
        self.registers.pwr.set(0);
        self.registers.cmd.set(0);
        self.registers.arg.set(0);
        self.registers.tout.set(0xf00000);
        self.registers.cdiv.set(0);
        self.registers.hsts.set(0x7f8);
        self.registers.cfg.set(0);
        self.registers.cnt.set(0);
        self.registers.size.set(0);
        self.registers
            .dbg
            .set(self.registers.dbg.get() & (!(0x1f << 14 | 0x1f << 9)) | (0x4 << 14 | 0x4 << 9));
        spin_for_cycle(250000);
        self.registers.pwr.set(1);
        spin_for_cycle(250000);
        self.registers
            .cfg
            .modify(SDHOST_CFG::DATA_EN::SET + SDHOST_CFG::INTBUS::SET + SDHOST_CFG::SLOW::SET);
        self.registers.cdiv.set(0x148);
    }

    fn setup_for_sdcard(&mut self) -> Result<(), &'static str> {
        self.sd_cmd(
            SDCardCommand::GoIdleState as u32 | SDHOST_CMD::NO_RESPONSE::SET.value,
            0,
        );
        self.sd_cmd(SDCardCommand::SendIfCondition as u32, 0x1aa);
        if self.registers.resp0.get() != 0x1aa {
            return Err("voltage check failed");
        }
        loop {
            self.sd_cmd(SDCardCommand::ApplicationCommand as u32, 0);
            self.sd_cmd(41, 1 << 21 | 1 << 30);
            let resp = self.registers.resp0.get();
            if resp & (1 << 31) != 0 {
                self.is_hcs = resp & (1 << 30) != 0;
                break;
            }
            spin_for_cycle(1000000);
        }
        self.sd_cmd(
            SDCardCommand::AllSendCid as u32 | SDHOST_CMD::LONG_RESPONSE::SET.value,
            0,
        );
        self.sd_cmd(SDCardCommand::SendRelativeAddress as u32, 0);
        self.sd_cmd(SDCardCommand::SelectCard as u32, self.registers.resp0.get());
        self.sd_cmd(SDCardCommand::SetBlockLength as u32, 512);
        Ok(())
    }

    fn wait_sd(&self) {
        while self.registers.cmd.get() & Self::SDHOST_NEW_CMD != 0 {
            core::hint::spin_loop();
        }
    }

    fn sd_cmd(&mut self, cmd: u32, arg: u32) {
        self.registers.arg.set(arg);
        self.registers.cmd.set(cmd | Self::SDHOST_NEW_CMD);
        self.wait_sd();
    }

    fn wait_fifo(&self) {
        while self.registers.hsts.get() & 1 == 0 {
            core::hint::spin_loop();
        }
    }

    fn wait_finish(&self) {
        while (self.registers.dbg.get() & 0xf) != 1 {
            core::hint::spin_loop();
        }
    }

    fn set_block(&mut self, size: u32, count: u32) {
        self.registers.size.set(size);
        self.registers.cnt.set(count);
    }

    fn read_block(&mut self, block_index: u32, buf: &mut [u8]) -> Result<(), &'static str> {
        let block_index = if self.is_hcs {
            block_index << 9
        } else {
            block_index
        };

        loop {
            self.set_block(512, 1);
            self.sd_cmd(
                SDCardCommand::ReadSingleBlock as u32 | SDHOST_CMD::READ::SET.value,
                block_index,
            );

            for i in 0..128 {
                self.wait_fifo();
                let data = self.registers.data.get();
                let bytes = data.to_le_bytes();
                for j in 0..4 {
                    buf[i * 4 + j] = bytes[j];
                }
            }
            if self.registers.hsts.get() & 0xf8 != 0 {
                self.registers.hsts.set(0xf8);
                self.sd_cmd(
                    SDCardCommand::StopTransmission as u32 | SDHOST_CMD::BUSY::SET.value,
                    0,
                );
            } else {
                break;
            }
        }
        self.wait_finish();
        Ok(())
    }

    fn write_block(&mut self, block_index: u32, buf: &[u8]) -> Result<(), &'static str> {
        let block_index = if self.is_hcs {
            block_index << 9
        } else {
            block_index
        };

        loop {
            self.set_block(512, 1);
            self.sd_cmd(
                SDCardCommand::WriteSingleBlock as u32 | SDHOST_CMD::WRITE::SET.value,
                block_index,
            );

            for i in 0..128 {
                self.wait_fifo();
                let mut data = [0_u8; 4];
                for j in 0..4 {
                    data[j] = buf[i * 4 + j];
                }
                self.registers.data.set(u32::from_le_bytes(data));
            }
            if self.registers.hsts.get() & 0xf8 != 0 {
                self.registers.hsts.set(0xf8);
                self.sd_cmd(
                    SDCardCommand::StopTransmission as u32 | SDHOST_CMD::BUSY::SET.value,
                    0,
                );
            } else {
                break;
            }
        }
        self.wait_finish();
        Ok(())
    }
}

pub struct SDHost {
    inner: Mutex<SDHostInner>,
}

impl SDHost {
    pub const unsafe fn new(mmio_start_addr: usize) -> Self {
        Self {
            inner: Mutex::new(SDHostInner::new(mmio_start_addr)),
        }
    }

    pub fn read_block(&self, block_index: u32, buf: &mut [u8]) -> Result<(), &'static str> {
        self.inner.lock().unwrap().read_block(block_index, buf)
    }

    pub fn write_block(&self, block_index: u32, buf: &[u8]) -> Result<(), &'static str> {
        self.inner.lock().unwrap().write_block(block_index, buf)
    }
}

impl DeviceDriver for SDHost {
    type InterruptNumberType = InterruptNumber;

    unsafe fn init(&self) -> Result<(), &'static str> {
        self.inner.lock().unwrap().setup();
        Ok(())
    }
}
