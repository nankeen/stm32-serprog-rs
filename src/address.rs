#[derive(Clone, Copy, Debug)]
pub struct Address(pub u32);

impl From<u32> for Address {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
