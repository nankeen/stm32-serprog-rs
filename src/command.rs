use crate::prelude::*;
use nom::{
    bytes::streaming,
    combinator::{map, map_opt},
    number::streaming::le_u24,
    sequence::pair,
    IResult,
};
use stm32f1xx_hal::time::Hertz;

#[derive(Clone, Copy, Debug)]
pub enum Command {
    Nop,
    QIface,
    QCmdMap,
    QPgmName,
    QSerBuf,
    QBusType,
    QChipSize,
    QOpBuf,
    QWrnMaxLen,
    RByte(Address),
    RNBytes { addr: Address, n: u32 },
    OInit,
    OWriteB { addr: Address, data: u8 },
    OWriteN { addr: Address, data: [u8; 256] },
    ODelay(u32),
    OExec,
    SyncNop,
    QRdnMaxLen,
    SBusType(BusType),
    OSpiOp { rlen: usize, data: [u8; 256] },
    SSpiFreq(Hertz),
    SPinState(bool),
}

impl Command {
    pub const MAX_SIZE: usize = 256;

    fn opcode(input: &[u8]) -> IResult<&[u8], OpCode> {
        map_opt(streaming::take(1usize), |s: &[u8]| OpCode::from_u8(s[0]))(input)
    }

    fn rbyte(input: &[u8]) -> IResult<&[u8], Self> {
        map(le_u24, |addr| Self::RByte(Address(addr)))(input)
    }

    fn rnbytes(input: &[u8]) -> IResult<&[u8], Self> {
        map(pair(le_u24, le_u24), |(addr, n)| Command::RNBytes {
            addr: Address(addr),
            n,
        })(input)
    }

    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (res, opcode) = Self::opcode(input)?;
        let ok = |opcode| Ok((res, opcode));

        match opcode {
            OpCode::Nop => ok(Self::Nop),
            OpCode::QIface => ok(Self::QIface),
            OpCode::QCmdMap => ok(Self::QCmdMap),
            OpCode::QPgmName => ok(Self::QPgmName),
            OpCode::QSerBuf => ok(Self::QSerBuf),
            OpCode::QBusType => ok(Self::QBusType),
            OpCode::QChipSize => ok(Self::QChipSize),
            OpCode::QOpBuf => ok(Self::QOpBuf),
            OpCode::QWrnMaxLen => ok(Self::QWrnMaxLen),
            OpCode::RByte => Self::rbyte(res),
            OpCode::RNBytes => Self::rnbytes(res),
            OpCode::OInit => ok(Self::OInit),
            _ => unimplemented!(),
        }
    }
}
