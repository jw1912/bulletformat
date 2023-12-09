use std::{
    fs::File,
    io::{self, BufWriter, Write, BufReader, BufRead},
    path::Path,
    str::FromStr,
};

use crate::{BulletFormat, DataLoader};

pub fn convert_from_text<U>(
    inp_path: impl AsRef<Path>,
    out_path: impl AsRef<Path>,
) -> io::Result<()>
where U: BulletFormat + FromStr<Err = String> + Send
{
    let loader = BufReader::new(File::open(inp_path).unwrap());
    let mut output = BufWriter::new(File::create(out_path)?);
    let mut buffer = Vec::new();

    for (i, line) in loader.lines().map(Result::unwrap).enumerate() {
        match line.parse::<U>() {
            Ok(position) => buffer.push(position),
            Err(error) => {
                println!("Error Parsing Line {}: {line}", i + 1);
                println!("Error Type: {error}");
            },
        }

        if buffer.len() % 16_384 == 0 {
            BulletFormat::write_to_bin(&mut output, &buffer).unwrap();
            buffer.clear();
        }
    }

    BulletFormat::write_to_bin(&mut output, &buffer).unwrap();
    buffer.clear();

    Ok(())
}

pub fn convert_from_bin<T, U>(
    inp_path: impl AsRef<Path>,
    out_path: impl AsRef<Path>,
    threads: usize,
) -> io::Result<()>
where
    T: Copy + Send + Sync,
    U: BulletFormat + From<T> + Send,
{
    let loader = DataLoader::<T>::new(inp_path, 512)?;
    let to_convert = loader.len();
    let mut output = BufWriter::new(File::create(out_path)?);
    let batch_size = loader.max_batch_size();
    let mut converted_count = 0;

    loader.map_batches(batch_size, |batch| {
        converted_count += batch.len();
        let converted = std::thread::scope(|s| {
            let chunk_size = batch.len() / threads + 1;
            batch
                .chunks(chunk_size)
                .map(|chunk| {
                    s.spawn(move || {
                        let mut buffer = Vec::with_capacity(chunk.len());
                        for &pos in chunk {
                            buffer.push(U::from(pos));
                        }
                        buffer
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|p| p.join().unwrap())
                .collect::<Vec<_>>()
        });

        for part in converted {
            BulletFormat::write_to_bin(&mut output, &part).unwrap();
        }

        print!(
            "> Converted {converted_count} / {to_convert} ({}%)\r",
            100.0 * converted_count as f32 / to_convert as f32
        );
        let _ = std::io::stdout().flush();
    });

    println!();

    Ok(())
}
