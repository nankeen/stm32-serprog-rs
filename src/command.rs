#[derive(Clone, Copy, Debug)]
pub enum Command {
    Nop,
    QIFace,
    QCmdMap,
    QPgmName,
    QSerBuf,
    QBusType,
    QChipSize,
    QOpBuf,
    QWrnMaxLen,
    RByte(u32),
    RNBytes { addr: u32, n: u32 },
    OInit,
    OWriteB { addr: u32, data: u8 },
    OWriteN {},
    ODelay,
    OExec,
    SyncNop,
    QRdnMaxLen,
    SBusType,
    OSpiOp,
    SSpiFreq,
    SPinState,
}