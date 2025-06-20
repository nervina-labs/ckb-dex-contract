use ckb_std::error::SysError;

#[repr(i8)]
#[derive(Clone, Copy)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,
    LockArgsInvalid = 5,
    DexOwnerLockNotMatch,
    DexFTTotalValueNotMatch,
    DexNFTTotalValueNotMatch,
    DexSetupInvalid,
    TotalValueOverflow = 10,
    UnitTypeNotMatch,
    TotalValueNotMatch,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            Unknown(err_code) => panic!("unexpected sys error {}", err_code),
        }
    }
}
