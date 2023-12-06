mod ataxx;
mod chess;

pub use ataxx::AtaxxBoard;
pub use chess::ChessBoard;

pub fn sigmoid(x: f32, k: f32) -> f32 {
    1. / (1. + (-x * k).exp())
}

pub trait BulletFormat: IntoIterator {
    type FeatureType;
    const INPUTS: usize;
    const MAX_FEATURES: usize;

    fn score(&self) -> i16;

    fn result(&self) -> f32;

    fn result_idx(&self) -> usize;

    fn blended_result(&self, blend: f32, scale: f32) -> f32 {
        blend * self.result() + (1. - blend) * sigmoid(f32::from(self.score()), scale)
    }
}
