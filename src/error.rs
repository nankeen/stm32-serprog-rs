use snafu::Snafu;

#[derive(Snafu, Debug)]
pub enum DataError {
    // #[snafu(display("Buffer of size {} provided while a buffer of size {} was required", buf_size, required))]
    BufferTooSmall { buf_size: usize, required: usize },
}
