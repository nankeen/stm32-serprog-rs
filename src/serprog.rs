use embedded_hal::serial::Read;
use snafu::Snafu;
use usb_device::bus::UsbBus;
use usb_device::prelude::UsbDevice;
use usbd_serial::SerialPort;

const I_FACE_VERSION: u16 = 0x01;
const PGM_NAME: &str = "stm32-vserprog";
// Support SPI only
const SUPPORTED_BUS: u8 = 1 << 3;
const CMD_MAP: u32 = 1 << OpCode::Nop as u8
    | 1 << OpCode::QIface as u8
    | 1 << OpCode::QCmdMap as u8
    | 1 << OpCode::QPgmName as u8
    | 1 << OpCode::QSerBuf as u8
    | 1 << OpCode::QBusType as u8
    | 1 << OpCode::SyncNop as u8
    | 1 << OpCode::OSpiOp as u8
    | 1 << OpCode::SBusType as u8
    | 1 << OpCode::SSpiFreq as u8
    | 1 << OpCode::SPinState as u8;

#[repr(u8)]
pub enum ReturnType {
    Ack = 0x06,
    Nak = 0x15,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum OpCode {
    Nop = 0x00,
    QIface = 0x01,
    QCmdMap = 0x02,
    QPgmName = 0x03,
    QSerBuf = 0x04,
    QBusType = 0x05,
    _QChipSize = 0x06,
    _QOpBuf = 0x07,
    _QWrnMaxLen = 0x08,
    _RByte = 0x09,
    _RNBytes = 0x0A,
    _OInit = 0x0B,
    _OWriteB = 0x0C,
    _OWriteN = 0x0D,
    _ODelay = 0x0E,
    _OExec = 0x0F,
    SyncNop = 0x10,
    _QRdnMaxLen = 0x11,
    SBusType = 0x12,
    OSpiOp = 0x13,
    SSpiFreq = 0x14,
    SPinState = 0x15,
}

impl OpCode {
    pub fn from_u8(n: u8) -> Option<OpCode> {
        if n <= 0x15 {
            Some(unsafe { core::mem::transmute(n) })
        } else {
            None
        }
    }
}

pub(crate) struct SerProg<'a, B>
where
    B: UsbBus,
{
    serial: SerialPort<'a, B>,
    usb_dev: UsbDevice<'a, B>,
}

#[derive(Snafu, Debug)]
pub enum SerProgError {
    #[snafu(display("Could not write to serial"))]
    WriteFail,
    #[snafu(display("Could not read from serial"))]
    ReadFail,
    #[snafu(display("OpCode {:?} is not implemented", opcode))]
    NotImplemented { opcode: OpCode },
}

impl<'a, B> SerProg<'a, B>
where
    B: UsbBus,
{
    pub fn new(serial: SerialPort<'a, B>, usb_dev: UsbDevice<'a, B>) -> Self {
        Self { serial, usb_dev }
    }

    pub fn read_u8(&mut self) -> u8 {
        loop {
            if let Ok(c) = Read::read(&mut self.serial) {
                return c;
            }

            if !self.usb_dev.poll(&mut [&mut self.serial]) {
                continue;
            }
        }
    }

    fn read_u24_as_u32(&mut self) -> u32 {
        let mut val = self.read_u8() as u32;
        val |= (self.read_u8() as u32) << 8;
        val |= (self.read_u8() as u32) << 16;
        val
    }

    pub fn handle_command(&mut self, cmd: OpCode) -> Result<(), SerProgError> {
        match cmd {
            OpCode::Nop => self.handle_nop(),
            OpCode::QIface => self.handle_q_iface(),
            OpCode::QCmdMap => self.handle_q_cmd_map(),
            OpCode::QPgmName => self.handle_q_pgm_name(),
            OpCode::QSerBuf => self.handle_q_serbuf(),
            OpCode::QBusType => self.handle_q_bus_type(),
            OpCode::SyncNop => self.handle_sync_nop(),
            OpCode::SBusType => self.handle_s_bus_type(),
            opcode => Err(SerProgError::NotImplemented { opcode }),
        }
    }

    fn handle_nop(&mut self) -> Result<(), SerProgError> {
        self.serial
            .write(&[ReturnType::Ack as u8])
            .map(|_| ())
            .map_err(|_| SerProgError::WriteFail)
    }

    fn handle_q_iface(&mut self) -> Result<(), SerProgError> {
        let ret_packet: [u8; 3] = {
            let mut ret_packet: [u8; 3] = [0; 3];
            ret_packet[0] = ReturnType::Ack as u8;
            ret_packet[1..].copy_from_slice(&I_FACE_VERSION.to_le_bytes());
            ret_packet
        };

        self.serial
            .write(&ret_packet)
            .map(|_| ())
            .map_err(|_| SerProgError::WriteFail)
    }

    fn handle_q_cmd_map(&mut self) -> Result<(), SerProgError> {
        let ret_packet: [u8; 33] = {
            let mut ret_packet: [u8; 33] = [0; 33];
            ret_packet[0] = ReturnType::Ack as u8;
            ret_packet[1..5].copy_from_slice(&CMD_MAP.to_le_bytes());
            ret_packet
        };

        self.serial
            .write(&ret_packet)
            .map(|_| ())
            .map_err(|_| SerProgError::WriteFail)
    }

    fn handle_q_pgm_name(&mut self) -> Result<(), SerProgError> {
        let ret_packet: [u8; 17] = {
            let mut ret_packet: [u8; 17] = [0; 17];
            ret_packet[0] = ReturnType::Ack as u8;
            ret_packet[1..PGM_NAME.len() + 1].copy_from_slice(PGM_NAME.as_bytes());
            ret_packet
        };

        self.serial
            .write(&ret_packet)
            .map(|_| ())
            .map_err(|_| SerProgError::WriteFail)
    }

    fn handle_q_serbuf(&mut self) -> Result<(), SerProgError> {
        // Pretend to be 64k
        self.serial
            .write(&[ReturnType::Ack as u8, 0xff, 0xff])
            .map(|_| ())
            .map_err(|_| SerProgError::WriteFail)
    }

    fn handle_q_bus_type(&mut self) -> Result<(), SerProgError> {
        self.serial
            .write(&[ReturnType::Ack as u8, SUPPORTED_BUS])
            .map(|_| ())
            .map_err(|_| SerProgError::WriteFail)
    }

    fn handle_sync_nop(&mut self) -> Result<(), SerProgError> {
        self.serial
            .write(&[ReturnType::Ack as u8, ReturnType::Nak as u8])
            .map(|_| ())
            .map_err(|_| SerProgError::WriteFail)
    }

    fn handle_s_bus_type(&mut self) -> Result<(), SerProgError> {
        let response = if self.read_u8() == SUPPORTED_BUS {
            ReturnType::Ack
        } else {
            ReturnType::Nak
        };

        self.serial
            .write(&[response as u8])
            .map(|_| ())
            .map_err(|_| SerProgError::WriteFail)
    }
}

/*
pub(crate) trait SerProg {
    fn handle_command(&mut self, cmd: OpCode) -> AsRef<[u8]>;
    fn read_u8(&mut self) -> u8;
    fn read_u24_as_u32(&mut self) -> u32 {
        self.read_u8() as u32
    }
}

impl<B, RS, WS> SerProg for SerialPort<'_, B, RS, WS>
where
    B: UsbBus,
    RS: BorrowMut<[u8]>,
    WS: BorrowMut<[u8]>,
{
    fn read_u8(&mut self) -> u8 {
        Read::read(self).unwrap()
    }

    fn handle_command(&mut self, cmd: OpCode) {
        match cmd {
            Nop => Ack
            _ => {}
        }
    }
}
*/
