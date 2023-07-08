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
