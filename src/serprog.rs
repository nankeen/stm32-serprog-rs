use core::borrow::BorrowMut;

use crate::{buffer::Buffer, command::Command, prelude::*, spi::SpiManager};
use snafu::Snafu;
use stm32f1xx_hal::{afio::MAPR, gpio::Cr, prelude::*};
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

    pub fn process_command<RS: BorrowMut<[u8]>>(
        &mut self,
        buffer: &mut Buffer<RS>,
    ) -> Result<ResponsePacket, SerProgError> {
        let (bytes_parsed, cmd) = loop {
            buffer
                .write_all(buffer.available_write(), |mut buf| {
                    self.serial.read(&mut buf)
                })
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
            Command::QIface => self.handle_qiface(),
            _ => unimplemented!("command not implemented"),
        }
    }

    fn handle_qiface(&mut self) -> Result<ResponsePacket, SerProgError> {
        Ok(ResponsePacket::QIface {
            iface_version: I_FACE_VERSION,
        })
    }

    // fn handle_q_cmd_map(&mut self) -> Result<ResponsePacket, SerProgError> {
    //     let cmd_map: [u8; 32] = {
    //         let mut cmd_map: [u8; 32] = [0; 32];
    //         cmd_map.copy_from_slice(&CMD_MAP.to_le_bytes());
    //         cmd_map
    //     };

    //     Ok(ResponsePacket::QCmdMap { cmd_map })
    // }

    // fn handle_q_pgm_name(&mut self) -> Result<ResponsePacket, SerProgError> {
    //     let pgm_name: [u8; 16] = {
    //         let mut pgm_name: [u8; 16] = [0; 16];
    //         pgm_name[0..PGM_NAME.len()].copy_from_slice(PGM_NAME.as_bytes());
    //         pgm_name
    //     };

    //     Ok(ResponsePacket::QPgmName { pgm_name })
    // }

    // fn handle_q_serbuf(&mut self) -> Result<ResponsePacket, SerProgError> {
    //     // Pretend to be 64k
    //     Ok(ResponsePacket::QSerBuf { size: 0xffff })
    // }

    // fn handle_q_bus_type(&mut self) -> Result<ResponsePacket, SerProgError> {
    //     Ok(ResponsePacket::QBusType {
    //         bus_type: SUPPORTED_BUS,
    //     })
    // }

    // fn handle_sync_nop(&mut self) -> Result<ResponsePacket, SerProgError> {
    //     Ok(ResponsePacket::SyncNop)
    // }

    // fn handle_s_bus_type(&mut self) -> Result<ResponsePacket, SerProgError> {
    //     unimplemented!();
    //     // let res = if self.read_u8() == SUPPORTED_BUS {
    //     //     ResponseType::Ack
    //     // } else {
    //     //     ResponseType::Nak
    //     // };

    //     // Ok(ResponsePacket::SBusType { res })
    // }

    // fn handle_o_spi_op(&mut self) -> Result<ResponsePacket, SerProgError> {
    //     // TODO: Implement OSpiOp
    //     Err(SerProgError::NotImplemented {
    //         opcode: OpCode::OSpiOp,
    //     })
    // }

    // fn handle_s_spi_freq(
    //     &mut self,
    //     mapr: &mut MAPR,
    //     crl: &mut Cr<'A', false>,
    // ) -> Result<ResponsePacket, SerProgError> {
    //     // Implement SSpiFreq
    //     unimplemented!();
    //     // let freq = self.read_u32();
    //     // if freq == 0 {
    //     //     Ok(ResponsePacket::SSpiFreq {
    //     //         res: ResponseType::Nak,
    //     //         set_freq: 0,
    //     //     })
    //     // } else {
    //     //     self.spi_manager.configure(freq.Hz(), mapr, crl);
    //     //     Ok(ResponsePacket::SSpiFreq {
    //     //         res: ResponseType::Ack,
    //     //         set_freq: freq,
    //     //     })
    //     // }
    // }

    // fn spi_select(&mut self) {
    //     // TODO
    //     // self.spi_cs.set_low().unwrap();
    // }

    // fn spi_unselect(&mut self) {
    //     // TODO
    //     // self.spi_cs.set_high().unwrap();
    // }
}
