#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum OpCode {
    Nop = 0x00,
    QIface = 0x01,
    QCmdMap = 0x02,
    QPgmName = 0x03,
    QSerBuf = 0x04,
    QBusType = 0x05,
    QChipSize = 0x06,
    QOpBuf = 0x07,
    QWrnMaxLen = 0x08,
    RByte = 0x09,
    RNBytes = 0x0A,
    OInit = 0x0B,
    OWriteB = 0x0C,
    OWriteN = 0x0D,
    ODelay = 0x0E,
    OExec = 0x0F,
    SyncNop = 0x10,
    QRdnMaxLen = 0x11,
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
