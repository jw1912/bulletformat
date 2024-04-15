use std::{
    fs::File,
    io::{self, Read},
    marker::PhantomData,
    path::Path,
};

use crate::{util, BulletFormat};

pub struct DataLoader<T> {
    file: File,
    buffer_size: usize,
    marker: PhantomData<T>,
}

impl<T: BulletFormat> DataLoader<T> {
    const DATA_SIZE: usize = std::mem::size_of::<T>();

    pub fn new(path: impl AsRef<Path>, buffer_size_mb: usize) -> io::Result<Self> {
        Ok(Self {
            file: File::open(path)?,
            buffer_size: buffer_size_mb * 1024 * 1024,
            marker: PhantomData,
        })
    }

    pub fn len(&self) -> usize {
        (self.file.metadata().unwrap().len() as usize - T::HEADER_SIZE) / Self::DATA_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn map_batches<F: FnMut(&[T])>(mut self, batch_size: usize, mut f: F) {
        let batches_per_load = self.buffer_size / Self::DATA_SIZE / batch_size;
        let cap = Self::DATA_SIZE * batch_size * batches_per_load;

        if T::HEADER_SIZE > 0 {
            let mut header = vec![0; T::HEADER_SIZE];
            self.file.read_exact(&mut header).unwrap();
        }

        let mut buffer = vec![0; cap];
        loop {
            let bytes_read = self.file.read(&mut buffer).unwrap();

            if bytes_read == 0 {
                break;
            }

            let data = util::to_slice_with_lifetime(&buffer[..bytes_read]);

            for batch in data.chunks(batch_size) {
                f(batch);
            }
        }
    }

    pub fn max_batch_size(&self) -> usize {
        self.buffer_size / Self::DATA_SIZE
    }

    pub fn map_positions<F: FnMut(&T)>(self, mut f: F) {
        let batch_size = self.max_batch_size();
        self.map_batches(batch_size, |batch| {
            for pos in batch {
                f(pos);
            }
        });
    }

    pub fn map_batches_threaded_loading<F: FnMut(&[T])>(mut self, batch_size: usize, mut f: F) {
        use std::sync::mpsc::sync_channel;

        let batches_per_load = self.buffer_size / Self::DATA_SIZE / batch_size;
        let cap = Self::DATA_SIZE * batch_size * batches_per_load;

        let (sender, reciever) = sync_channel::<Vec<u8>>(2);

        let dataloader = std::thread::spawn(move || {
            if T::HEADER_SIZE > 0 {
                let mut header = vec![0; T::HEADER_SIZE];
                self.file.read_exact(&mut header).unwrap();
            }

            let mut buffer = vec![0; cap];
            loop {
                let bytes_read = self.file.read(&mut buffer).unwrap();

                if bytes_read == 0 {
                    break;
                }

                sender.send(buffer.to_vec()).unwrap();
            }
        });

        while let Ok(buf) = reciever.recv() {
            let data = util::to_slice_with_lifetime(&buf);

            for batch in data.chunks(batch_size) {
                f(batch);
            }
        }

        dataloader.join().unwrap();
    }
}
