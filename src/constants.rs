use crate::prelude::*;

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

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum BusType {
    Parallel = 1 << 0,
    Lpc = 1 << 1,
    Fwh = 1 << 2,
    Spi = 1 << 3,
}
