use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    marker::PhantomData,
    path::Path,
};

use crate::util;

pub struct DataLoader<T> {
    file: File,
    buffer_size: usize,
    marker: PhantomData<T>,
}

impl<T> DataLoader<T> {
    const DATA_SIZE: usize = std::mem::size_of::<T>();

    pub fn new(path: impl AsRef<Path>, buffer_size_mb: usize) -> io::Result<Self> {
        Ok(Self {
            file: File::open(path)?,
            buffer_size: buffer_size_mb * 1024 * 1024,
            marker: PhantomData,
        })
    }

    pub fn len(&self) -> usize {
        self.file.metadata().unwrap().len() as usize / Self::DATA_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn map_batches<F: FnMut(&[T])>(self, batch_size: usize, mut f: F) {
        let batches_per_load = self.buffer_size / Self::DATA_SIZE / batch_size;
        let cap = Self::DATA_SIZE * batch_size * batches_per_load;

        let mut reader = BufReader::with_capacity(cap, self.file);

        while let Ok(buf) = reader.fill_buf() {
            if buf.is_empty() {
                break;
            }

            let data = util::to_slice_with_lifetime(buf);

            for batch in data.chunks(batch_size) {
                f(batch);
            }

            let consumed = buf.len();
            reader.consume(consumed);
        }
    }

    pub fn max_batch_size(&self) -> usize {
        self.buffer_size / Self::DATA_SIZE
    }

    pub fn map_positions<F: Fn(&T)>(self, f: F) {
        let batch_size = self.max_batch_size();
        self.map_batches(batch_size, |batch| {
            for pos in batch {
                f(pos);
            }
        });
    }
}
