use crate::prelude::*;
use nom::{
    bytes::streaming,
    combinator::{map, map_opt},
    number::{
        complete::le_u32,
        streaming::{le_u24, le_u8},
    },
    sequence::pair,
    IResult,
};
use stm32f1xx_hal::time::{Hertz, Hz};

#[derive(Clone, Copy, Debug)]
pub enum Command<'a> {
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
    OWriteN { addr: Address, data: &'a [u8] },
    ODelay(u32),
    OExec,
    SyncNop,
    QRdnMaxLen,
    SBusType(BusType),
    OSpiOp { rlen: u32, data: &'a [u8] },
    SSpiFreq(Hertz),
    SPinState(bool),
}

impl<'a> Command<'a> {
    pub const MAX_SIZE: usize = 1024;

    fn opcode(input: &[u8]) -> IResult<&[u8], OpCode> {
        map_opt(le_u8, OpCode::from_u8)(input)
    }

    fn rbyte(input: &[u8]) -> IResult<&[u8], Self> {
        map(le_u24, |addr| Self::RByte(Address(addr)))(input)
    }

    fn rnbytes(input: &[u8]) -> IResult<&[u8], Self> {
        map(pair(le_u24, le_u24), |(addr, n)| Self::RNBytes {
            addr: Address(addr),
            n,
        })(input)
    }

    fn owriteb(input: &[u8]) -> IResult<&[u8], Self> {
        map(pair(le_u24, le_u8), |(addr, data)| Self::OWriteB {
            addr: Address(addr),
            data,
        })(input)
    }

    fn owriten(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, (n, addr)) = pair(le_u24, le_u24)(input)?;
        let result = map(streaming::take(n), |data| Self::OWriteN {
            addr: Address(addr),
            data,
        })(input);
        result
    }

    fn odelay(input: &[u8]) -> IResult<&[u8], Self> {
        map(le_u32, Self::ODelay)(input)
    }

    fn sbustype(input: &[u8]) -> IResult<&[u8], Self> {
        map(le_u8, |flags| Self::SBusType(BusType(flags)))(input)
    }

    fn ospiop(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, (slen, rlen)) = pair(le_u24, le_u24)(input)?;
        let result = map(streaming::take(slen), |data| Self::OSpiOp { rlen, data })(input);
        result
    }

    fn sspifreq(input: &[u8]) -> IResult<&[u8], Self> {
        map(le_u32, |freq| Self::SSpiFreq(Hz(freq)))(input)
    }

    fn spinstate(input: &[u8]) -> IResult<&[u8], Self> {
        map(le_u8, |state| Self::SPinState(state == 0))(input)
    }

    pub fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
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
            OpCode::OWriteB => Self::owriteb(res),
            OpCode::OWriteN => Self::owriten(res),
            OpCode::ODelay => Self::odelay(res),
            OpCode::OExec => ok(Self::OExec),
            OpCode::SyncNop => ok(Self::SyncNop),
            OpCode::QRdnMaxLen => ok(Self::QRdnMaxLen),
            OpCode::SBusType => Self::sbustype(res),
            OpCode::OSpiOp => Self::ospiop(res),
            OpCode::SSpiFreq => Self::sspifreq(res),
            OpCode::SPinState => Self::spinstate(res),
        }
    }
}
