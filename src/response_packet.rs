use stm32f1xx_hal::time::Hertz;

use crate::{buffer::Buffer, prelude::*};

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum ResponseType {
    Ack = 0x06,
    Nak = 0x15,
}

#[repr(C)]
pub enum ResponsePacket {
    Nop,
    QIface {
        iface_version: u16,
    },
    QCmdMap {
        cmd_map: [u8; 32],
    },
    QPgmName {
        pgm_name: [u8; 16],
    },
    QSerBuf {
        size: u16,
    },
    QBusType {
        bus_type: BusType,
    },
    QOpBuf {
        size: u16,
    },
    QWrnMaxLen {
        size: u32,
    },
    SyncNop,
    SBusType {
        res: ResponseType,
    },
    SpiOp {
        res: ResponseType,
        rlen: usize,
        data: Buffer<[u8; MAX_BUFFER_SIZE]>,
    },
    SSpiFreq {
        res: ResponseType,
        set_freq: Hertz,
    },
}

impl ResponsePacket {
    pub const MAX_SIZE: usize = 33;

    pub fn to_bytes(&self, buf: &mut [u8]) -> Result<usize, DataError> {
        let packet_size = self.packet_size();
        let buf_size = buf.len();

        if buf_size < packet_size {
            return Err(DataError::BufferTooSmall {
                buf_size,
                required: packet_size,
            });
        }

        match self {
            ResponsePacket::Nop => {
                buf[0] = ResponseType::Ack as u8;
            }
            ResponsePacket::QIface { iface_version } => {
                buf[0] = ResponseType::Ack as u8;
                buf[1..].copy_from_slice(&iface_version.to_le_bytes());
            }
            ResponsePacket::QCmdMap { cmd_map } => {
                buf[0] = ResponseType::Ack as u8;
                buf[1..].copy_from_slice(cmd_map);
            }
            ResponsePacket::QPgmName { pgm_name } => {
                buf[0] = ResponseType::Ack as u8;
                buf[1..].copy_from_slice(pgm_name);
            }
            ResponsePacket::QSerBuf { size } => {
                buf[0] = ResponseType::Ack as u8;
                buf[1..].copy_from_slice(&size.to_le_bytes());
            }
            ResponsePacket::QBusType { bus_type } => {
                buf[0] = ResponseType::Ack as u8;
                buf[1] = bus_type.0;
            }
            ResponsePacket::QOpBuf { size } => {
                buf[0] = ResponseType::Ack as u8;
                buf[1..].copy_from_slice(&size.to_le_bytes())
            }
            ResponsePacket::QWrnMaxLen { size } => {
                buf[0] = ResponseType::Ack as u8;
                buf[1..].copy_from_slice(&size.to_le_bytes()[..3])
            }
            ResponsePacket::SyncNop => {
                buf[0] = ResponseType::Ack as u8;
                buf[1] = ResponseType::Nak as u8;
            }
            ResponsePacket::SBusType { res } => {
                buf[0] = *res as u8;
            }
            ResponsePacket::SpiOp { res, rlen, data } => {
                buf[0] = *res as u8;
                match res {
                    ResponseType::Nak => (),
                    ResponseType::Ack => {
                        buf[1..*rlen].copy_from_slice(&data[..*rlen]);
                    }
                }
            }
            ResponsePacket::SSpiFreq { res, set_freq } => {
                buf[0] = *res as u8;
                match res {
                    ResponseType::Nak => (),
                    ResponseType::Ack => {
                        buf[1..].copy_from_slice(&set_freq.to_Hz().to_le_bytes());
                    }
                }
            }
        }

        Ok(packet_size)
    }

    pub fn packet_size(&self) -> usize {
        match self {
            ResponsePacket::Nop => 1,
            ResponsePacket::QIface { .. } => 3,
            ResponsePacket::QCmdMap { .. } => 33,
            ResponsePacket::QPgmName { .. } => 17,
            ResponsePacket::QSerBuf { .. } => 3,
            ResponsePacket::QBusType { .. } => 2,
            ResponsePacket::QOpBuf { .. } => 3,
            ResponsePacket::QWrnMaxLen { .. } => 4,
            ResponsePacket::SyncNop => 2,
            ResponsePacket::SBusType { .. } => 1,
            ResponsePacket::SpiOp { rlen, .. } => rlen + 1,
            ResponsePacket::SSpiFreq { .. } => 5,
        }
    }
}
