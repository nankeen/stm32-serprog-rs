use core::borrow::{Borrow, BorrowMut};
use core::{cmp, ptr};

pub struct Buffer<S: BorrowMut<[u8]>> {
    store: S,
    rpos: usize,
    wpos: usize,
}

impl<S: BorrowMut<[u8]>> Buffer<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            rpos: 0,
            wpos: 0,
        }
    }

    pub fn clear(&mut self) {
        self.rpos = 0;
        self.wpos = 0;
    }

    pub fn available_read(&self) -> usize {
        // Assumption: 0 <= rpos <= store_len && rpos <= wpos < (rpos + store_len)
        self.wpos - self.rpos
    }

    fn available_read_without_wrap(&self) -> usize {
        cmp::min(self.wpos, self.store.borrow().len()) - self.rpos
    }

    // Amount of space in bytes available for writing
    pub fn available_write(&self) -> usize {
        self.available_write_without_discard() + self.rpos
    }

    fn available_write_without_discard(&self) -> usize {
        self.store.borrow().len() - self.wpos
    }

    pub fn consume(&mut self, amt: usize) {
        let count = cmp::min(amt, self.available_read());
        let store_len = self.store.borrow().len();

        self.rpos += count;

        if self.rpos >= store_len {
            self.rpos %= store_len;
            self.wpos %= store_len;
        }
    }

    // Writes as much as possible of data to the buffer and returns the number of bytes written
    pub fn write(&mut self, data: &[u8]) -> usize {
        if data.len() > self.available_write_without_discard() && self.rpos > 0 {
            // data doesn't fit in already available space, and there is data to discard
            self.discard_already_read_data();
        }

        let count = cmp::min(self.available_write_without_discard(), data.len());
        if count == 0 {
            // Buffer is full (or data is empty)
            return 0;
        }

        self.store.borrow_mut()[self.wpos..self.wpos + count].copy_from_slice(&data[..count]);

        self.wpos += count;
        count
    }

    // Reserves max_count bytes of space for writing, and passes a slice pointing to them to a
    // closure for writing. The closure should return the number of bytes actually written and is
    // allowed to write less than max_bytes. If the callback returns an error, any written data is
    // ignored.
    pub fn write_all<E>(
        &mut self,
        max_count: usize,
        f: impl FnOnce(&mut [u8]) -> Result<usize, E>,
    ) -> Result<usize, E> {
        if max_count > self.available_write_without_discard() {
            // Data doesn't fit in currently available space
            if max_count > self.available_write() {
                // Data doesn't fit even if we discard already read data
                return Ok(0);
            }

            self.discard_already_read_data();
        }

        assert!(self.available_write_without_discard() >= max_count);

        f(&mut self.store.borrow_mut()[self.wpos..self.wpos + max_count]).map(|count| {
            self.wpos += count;
            count
        })
    }

    // Takes up to max_count bytes from the buffer and passes a slice pointing to them to a closure
    // for reading. The closure should return the number of bytes actually read and is allowed to
    // read less than max_bytes. If the callback returns an error, the data is not discarded from
    // the buffer.
    pub fn read<'a, A>(&'a mut self, max_count: usize, f: impl FnOnce(&'a [u8]) -> A) -> A {
        let count = cmp::min(max_count, self.available_read());

        f(&self.store.borrow()[self.rpos..self.rpos + count])
    }

    fn discard_already_read_data(&mut self) {
        let data = self.store.borrow_mut();
        if self.rpos != data.len() {
            unsafe {
                ptr::copy(
                    &data[self.rpos] as *const u8,
                    &mut data[0] as *mut u8,
                    self.available_read(),
                );
            }
        }

        self.wpos -= self.rpos;
        self.rpos = 0;
    }
}

#[derive(Debug)]
pub struct BufferStore256([u8; 256]);

impl BufferStore256 {
    pub fn new() -> Self {
        Self([0; 256])
    }
}

impl Borrow<[u8]> for BufferStore256 {
    fn borrow(&self) -> &[u8] {
        &self.0
    }
}

impl BorrowMut<[u8]> for BufferStore256 {
    fn borrow_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {

    const DATA: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    const LEN: usize = 5;
    type Buf = crate::buffer::Buffer<[u8; 16]>;

    #[test]
    fn write() {
        let mut b = Buf::new([0; 16]);

        assert_eq!(b.write(&DATA[0..2]), 2);
        assert_eq!(b.available_write(), LEN - 2);
        assert_eq!(b.available_read(), 2);

        assert_eq!(b.write(&DATA[0..5]), 3);
        assert_eq!(b.available_write(), 0);
        assert_eq!(b.available_read(), LEN);
    }

    #[test]
    fn read() {
        let mut b = Buf::new([0; 16]);

        assert_eq!(b.write(&DATA[0..4]), 4);

        b.read(3, |data| {
            assert_eq!(data, &DATA[0..3]);
        });
        b.read(1, |data| {
            assert_eq!(data, &DATA[3..4]);
        });
        b.read(1, |data| {
            assert_eq!(data, &[]);
        });
    }

    #[test]
    fn clear() {
        let mut b = Buf::new([0; 16]);

        b.write(&DATA[0..2]);
        b.clear();

        assert_eq!(b.available_write(), LEN);
        assert_eq!(b.available_read(), 0);
    }

    #[test]
    fn discard() {
        let mut b = Buf::new([0; 16]);

        assert_eq!(b.write(&DATA[0..4]), 4);
        b.read(2, |data| {
            assert_eq!(data, &DATA[0..2]);
        });

        assert_eq!(b.write(&DATA[4..7]), 3);
        b.read(5, |data| {
            assert_eq!(data, &DATA[2..7]);
        });

        assert_eq!(b.available_read(), 0);
    }
}
