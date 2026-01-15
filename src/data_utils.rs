use snafu::Snafu;

pub const I_FACE_VERSION: u16 = 0x01;
pub const PGM_NAME: &str = "stm32-vserprog";
// Support SPI only
pub const SUPPORTED_BUS: u8 = 1 << 3;
pub const CMD_MAP: u32 = 1 << OpCode::Nop as u8
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
pub const MAX_BUFFER_SIZE: usize = 128;

#[derive(Snafu, Debug)]
pub enum DataError {
    // #[snafu(display("Buffer of size {} provided while a buffer of size {} was required", buf_size, required))]
    BufferTooSmall { buf_size: usize, required: usize },
}

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
        bus_type: u8,
    },
    SyncNop,
    SBusType {
        res: ResponseType,
    },
    SpiOp {
        res: ResponseType,
        rlen: usize,
        data: [u8; MAX_BUFFER_SIZE],
    },
    SSpiFreq {
        res: ResponseType,
        set_freq: u32,
    },
    SPinState {
        res: ResponseType,
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
                buf[1] = *bus_type;
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
                        buf[1..].copy_from_slice(&set_freq.to_le_bytes());
                    }
                }
            }
            ResponsePacket::SPinState { res } => {
                buf[0] = *res as u8;
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
            ResponsePacket::SyncNop => 2,
            ResponsePacket::SBusType { .. } => 1,
            ResponsePacket::SpiOp { rlen, .. } => rlen + 1,
            ResponsePacket::SSpiFreq { .. } => 5,
            ResponsePacket::SPinState { .. } => 1,
        }
    }
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
