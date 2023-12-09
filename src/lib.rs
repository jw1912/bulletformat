mod ataxx;
mod chess;
mod convert;
mod loader;
mod util;

use std::{
    fs::File,
    io::{self, BufWriter, Write},
    marker::Sized,
};

pub use ataxx::AtaxxBoard;
pub use chess::ChessBoard;
pub use convert::{convert_from_bin, convert_from_text};
pub use loader::DataLoader;

pub trait BulletFormat: IntoIterator + Sized {
    type FeatureType;
    const INPUTS: usize;
    const MAX_FEATURES: usize;

    fn score(&self) -> i16;

    fn result(&self) -> f32;

    fn result_idx(&self) -> usize;

    fn blended_result(&self, blend: f32, scale: f32) -> f32 {
        blend * self.result() + (1. - blend) * util::sigmoid(f32::from(self.score()), scale)
    }

    fn write_to_bin(output: &mut BufWriter<File>, data: &[Self]) -> io::Result<()> {
        let data_slice = util::to_slice_with_lifetime(data);
        output.write_all(data_slice)?;
        Ok(())
    }
}
