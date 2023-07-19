use core::{
    borrow::{Borrow, BorrowMut},
    convert::TryInto,
};

use crate::{buffer::Buffer, command::Command, prelude::*, spi::SpiManager};
use snafu::Snafu;
use stm32f1xx_hal::{
    pac::SPI2,
    spi::{Pins, Remap},
    time::{Hertz, Hz},
};
use usb_device::{bus::UsbBus, prelude::UsbDevice};
use usbd_serial::SerialPort;

pub(crate) struct SerProg<'a, B, REMAP, PINS>
where
    B: UsbBus,
    REMAP: Remap<Periph = SPI2>,
    PINS: Pins<REMAP>,
{
    spi_manager: Option<SpiManager<REMAP, PINS>>,
    serial: SerialPort<'a, B>,
    op_buf: Buffer<[u8; OP_BUF_SIZE]>,
    ser_buf: Buffer<[u8; SER_BUF_SIZE]>,
    _usb_dev: UsbDevice<'a, B>,
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

pub const OP_BUF_SIZE: usize = 1024;
pub const SER_BUF_SIZE: usize = 1024;

impl<'a, B, REMAP, PINS> SerProg<'a, B, REMAP, PINS>
where
    B: UsbBus + 'a,
    REMAP: Remap<Periph = SPI2>,
    PINS: Pins<REMAP>,
{
    pub fn new(
        spi_manager: SpiManager<REMAP, PINS>,
        serial: SerialPort<'a, B>,
        _usb_dev: UsbDevice<'a, B>,
    ) -> Self {
        Self {
            spi_manager: Some(spi_manager),
            serial,
            op_buf: Buffer::new([0u8; OP_BUF_SIZE]),
            ser_buf: Buffer::new([0u8; SER_BUF_SIZE]),
            _usb_dev,
        }
    }

    pub fn process_command<RS: BorrowMut<[u8]>>(
        &mut self,
        buffer: &mut Buffer<RS>,
    ) -> Result<ResponsePacket, SerProgError> {
        let (bytes_parsed, cmd) = loop {
            buffer
                .write_all(buffer.available_write(), |buf| self.serial.read(buf))
                .map_err(|_| SerProgError::ReadFail)?;

            let n = buffer.available_read();

            match buffer.read(n, Command::parse) {
                // Loop and get more data if incomplete
                Err(nom::Err::Incomplete(_)) => (),
                Err(_) => break Err(SerProgError::ReadFail),
                Ok((bytes_left, cmd)) => {
                    let bytes_parsed = n - bytes_left.len();
                    break Ok((bytes_parsed, cmd));
                }
            }
        }?;

        let response = self.handle_command(cmd)?;

        buffer.consume(bytes_parsed);

        Ok(response)
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

    fn handle_command(&mut self, cmd: Command) -> Result<ResponsePacket, SerProgError> {
        match cmd {
            Command::Nop => Ok(ResponsePacket::Nop),
            Command::QIface => self.handle_q_iface(),
            Command::QCmdMap => self.handle_q_cmd_map(),
            Command::QPgmName => self.handle_q_pgm_name(),
            Command::QSerBuf => self.handle_q_serbuf(),
            Command::QBusType => self.handle_q_bus_type(),
            Command::QChipSize => self.handle_q_chip_size(),
            Command::QOpBuf => self.handle_q_op_buf(),
            Command::QWrnMaxLen => self.handle_q_wrn_max_len(),
            Command::RByte(addr) => self.handle_r_byte(addr),
            Command::SyncNop => self.handle_sync_nop(),
            Command::SBusType(bustype) => self.handle_s_bus_type(&bustype),
            Command::OSpiOp { rlen, data } => {
                self.handle_o_spi_op(rlen.try_into().unwrap(), Buffer::new(data))
            }
            Command::SSpiFreq(freq) => self.handle_s_spi_freq(freq),
            _ => unimplemented!("command not implemented"),
        }
    }

    fn handle_q_op_buf(&self) -> Result<ResponsePacket, SerProgError> {
        Ok(ResponsePacket::QOpBuf {
            size: self.op_buf.len().try_into().unwrap(),
        })
    }

    fn handle_q_wrn_max_len(&self) -> Result<ResponsePacket, SerProgError> {
        Ok(ResponsePacket::QWrnMaxLen {
            size: self.ser_buf.len().try_into().unwrap(),
        })
    }

    fn handle_q_chip_size(&self) -> Result<ResponsePacket, SerProgError> {
        // TODO
        Err(SerProgError::NotImplemented {
            opcode: OpCode::QChipSize,
        })
    }

    fn handle_r_byte(&self, _address: Address) -> Result<ResponsePacket, SerProgError> {
        // TODO
        Err(SerProgError::NotImplemented {
            opcode: OpCode::RByte,
        })
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
        Ok(ResponsePacket::QSerBuf {
            size: self.ser_buf.len().try_into().unwrap(),
        })
    }

    fn handle_q_bus_type(&mut self) -> Result<ResponsePacket, SerProgError> {
        Ok(ResponsePacket::QBusType {
            bus_type: BusType::SPI,
        })
    }

    fn handle_sync_nop(&mut self) -> Result<ResponsePacket, SerProgError> {
        Ok(ResponsePacket::SyncNop)
    }

    fn handle_s_bus_type(&mut self, &bustype: &BusType) -> Result<ResponsePacket, SerProgError> {
        let res = match bustype {
            BusType::SPI => ResponseType::Ack,
            _ => ResponseType::Nak,
        };

        Ok(ResponsePacket::SBusType { res })
    }

    fn handle_o_spi_op<S: Borrow<[u8]>>(
        &mut self,
        rlen: usize,
        tx_data: Buffer<S>,
    ) -> Result<ResponsePacket, SerProgError> {
        let rx_buffer = Buffer::new([0u8; MAX_BUFFER_SIZE]);
        let (rx_buffer, _tx_buffer, spi) = self
            .spi_manager
            .take()
            // FIXME: Use the right errors
            .ok_or(SerProgError::WriteFail)
            .and_then(|spi| {
                spi.read_write(rx_buffer, tx_data)
                    .map_err(|_| SerProgError::WriteFail)
            })?;

        self.spi_manager = Some(spi);

        Ok(ResponsePacket::SpiOp {
            res: ResponseType::Ack,
            rlen,
            data: rx_buffer,
        })
    }

    fn handle_s_spi_freq(&mut self, freq: Hertz) -> Result<ResponsePacket, SerProgError> {
        // Implement SSpiFreq
        if freq == Hz(0) {
            Ok(ResponsePacket::SSpiFreq {
                res: ResponseType::Nak,
                set_freq: Hz(0),
            })
        } else {
            self.spi_manager = self
                .spi_manager
                .take()
                .map(|spi_manager| spi_manager.configure(freq));

            Ok(ResponsePacket::SSpiFreq {
                res: ResponseType::Ack,
                set_freq: freq,
            })
        }
    }
}
