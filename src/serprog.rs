use crate::data_utils::{
    OpCode, ResponsePacket, ResponseType, CMD_MAP, I_FACE_VERSION, PGM_NAME, SUPPORTED_BUS,
};
use embedded_hal::serial::Read;
use snafu::Snafu;
use usb_device::bus::UsbBus;
use usb_device::prelude::UsbDevice;
use usbd_serial::SerialPort;

pub(crate) struct SerProg<'a, B>
where
    B: UsbBus,
{
    pub serial: SerialPort<'a, B>,
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

    pub fn send_response(&mut self, buf: &[u8]) {
        let mut write_offset = 0;
        let count = buf.len();
        while write_offset < count {
            match self.serial.write(&buf[write_offset..count]) {
                Ok(len) if len > 0 => {
                    write_offset += len;
                }
                _ => {}
            }
        }
    }

    pub fn handle_command(&mut self, cmd: OpCode) -> Result<ResponsePacket, SerProgError> {
        match cmd {
            OpCode::Nop => self.handle_nop(),
            OpCode::QIface => self.handle_q_iface(),
            OpCode::QCmdMap => self.handle_q_cmd_map(),
            OpCode::QPgmName => self.handle_q_pgm_name(),
            OpCode::QSerBuf => self.handle_q_serbuf(),
            OpCode::QBusType => self.handle_q_bus_type(),
            OpCode::SyncNop => self.handle_sync_nop(),
            OpCode::SBusType => self.handle_s_bus_type(),
            OpCode::OSpiOp => self.handle_o_spi_op(),
            opcode => Err(SerProgError::NotImplemented { opcode }),
        }
    }

    fn handle_nop(&mut self) -> Result<ResponsePacket, SerProgError> {
        Ok(ResponsePacket::Nop)
    }

    fn handle_q_iface(&mut self) -> Result<ResponsePacket, SerProgError> {
        Ok(ResponsePacket::QIface {
            iface_version: I_FACE_VERSION,
        })
    }

    fn handle_q_cmd_map(&mut self) -> Result<ResponsePacket, SerProgError> {
        let cmd_map: [u8; 32] = {
            let mut cmd_map: [u8; 32] = [0; 32];
            cmd_map.copy_from_slice(&CMD_MAP.to_le_bytes());
            cmd_map
        };

        Ok(ResponsePacket::QCmdMap { cmd_map })
    }

    fn handle_q_pgm_name(&mut self) -> Result<ResponsePacket, SerProgError> {
        let pgm_name: [u8; 16] = {
            let mut pgm_name: [u8; 16] = [0; 16];
            pgm_name[0..PGM_NAME.len()].copy_from_slice(PGM_NAME.as_bytes());
            pgm_name
        };

        Ok(ResponsePacket::QPgmName { pgm_name })
    }

    fn handle_q_serbuf(&mut self) -> Result<ResponsePacket, SerProgError> {
        // Pretend to be 64k
        Ok(ResponsePacket::QSerBuf { size: 0xffff })
    }

    fn handle_q_bus_type(&mut self) -> Result<ResponsePacket, SerProgError> {
        Ok(ResponsePacket::QBusType {
            bus_type: SUPPORTED_BUS,
        })
    }

    fn handle_sync_nop(&mut self) -> Result<ResponsePacket, SerProgError> {
        Ok(ResponsePacket::SyncNop)
    }

    fn handle_s_bus_type(&mut self) -> Result<ResponsePacket, SerProgError> {
        let res = if self.read_u8() == SUPPORTED_BUS {
            ResponseType::Ack
        } else {
            ResponseType::Nak
        };

        Ok(ResponsePacket::SBusType { res })
    }

    fn handle_o_spi_op(&mut self) -> Result<ResponsePacket, SerProgError> {
        // TODO: Implement OSpiOp
        Err(SerProgError::NotImplemented { opcode: OpCode::OSpiOp })
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
