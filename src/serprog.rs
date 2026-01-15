use crate::{
    data_utils::{
        OpCode, ResponsePacket, ResponseType, CMD_MAP, I_FACE_VERSION, MAX_BUFFER_SIZE, PGM_NAME,
        SUPPORTED_BUS,
    },
    spi::SpiManager,
};
use embedded_hal::serial::Read;
use snafu::Snafu;
use stm32f1xx_hal::{afio::MAPR, gpio::gpioa::CRL, rcc::APB2, time::U32Ext};
use usb_device::{bus::UsbBus, prelude::UsbDevice};
use usbd_serial::SerialPort;

pub(crate) struct SerProg<'a, B>
where
    B: UsbBus,
{
    spi_manager: SpiManager,
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
    pub fn new(
        spi_manager: SpiManager,
        serial: SerialPort<'a, B>,
        usb_dev: UsbDevice<'a, B>,
    ) -> Self {
        Self {
            spi_manager,
            serial,
            usb_dev,
        }
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

    fn read_u32(&mut self) -> u32 {
        let mut val = self.read_u8() as u32;
        val |= (self.read_u8() as u32) << 8;
        val |= (self.read_u8() as u32) << 16;
        val |= (self.read_u8() as u32) << 24;
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

    pub fn handle_command(
        &mut self,
        cmd: OpCode,
        mapr: &mut MAPR,
        crl: &mut CRL,
        apb: &mut APB2,
    ) -> Result<ResponsePacket, SerProgError> {
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
            OpCode::SSpiFreq => self.handle_s_spi_freq(mapr, crl, apb),
            OpCode::SPinState => self.handle_s_pin_state(mapr, crl, apb),
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
        // Read slen (3 bytes, little-endian)
        let slen = self.read_u24_as_u32() as usize;

        // Read rlen (3 bytes, little-endian)
        let rlen = self.read_u24_as_u32() as usize;

        // Validate buffer constraints
        if slen > MAX_BUFFER_SIZE || rlen > MAX_BUFFER_SIZE {
            return Ok(ResponsePacket::SpiOp {
                res: ResponseType::Nak,
                rlen: 0,
                data: [0; MAX_BUFFER_SIZE],
            });
        }

        // Read write data into temporary buffer
        let mut write_buf = [0u8; MAX_BUFFER_SIZE];
        for i in 0..slen {
            write_buf[i] = self.read_u8();
        }

        // Prepare read buffer
        let mut read_buf = [0u8; MAX_BUFFER_SIZE];

        // Perform SPI transfer with CS control
        match self
            .spi_manager
            .transfer_with_cs(&write_buf[..slen], &mut read_buf[..rlen])
        {
            Ok(()) => Ok(ResponsePacket::SpiOp {
                res: ResponseType::Ack,
                rlen,
                data: read_buf,
            }),
            Err(()) => Ok(ResponsePacket::SpiOp {
                res: ResponseType::Nak,
                rlen: 0,
                data: [0; MAX_BUFFER_SIZE],
            }),
        }
    }

    fn handle_s_spi_freq(
        &mut self,
        mapr: &mut MAPR,
        crl: &mut CRL,
        apb: &mut APB2,
    ) -> Result<ResponsePacket, SerProgError> {
        // Implement SSpiFreq
        let freq = self.read_u32();
        if freq == 0 {
            Ok(ResponsePacket::SSpiFreq {
                res: ResponseType::Nak,
                set_freq: 0,
            })
        } else {
            self.spi_manager.configure(freq.hz(), mapr, crl, apb);
            Ok(ResponsePacket::SSpiFreq {
                res: ResponseType::Ack,
                set_freq: freq,
            })
        }
    }

    fn handle_s_pin_state(
        &mut self,
        mapr: &mut MAPR,
        crl: &mut CRL,
        apb: &mut APB2,
    ) -> Result<ResponsePacket, SerProgError> {
        let pin_state = self.read_u8();

        if pin_state == 0 {
            // Disable pin drivers
            self.spi_manager.disable(crl);
        } else {
            // Enable pin drivers if currently disabled
            if self.spi_manager.is_disabled() {
                // Use the stored last_freq
                let freq = self.spi_manager.last_freq;
                self.spi_manager.enable(freq, mapr, crl, apb);
            }
        }

        Ok(ResponsePacket::SPinState {
            res: ResponseType::Ack,
        })
    }
}
