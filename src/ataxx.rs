use crate::BulletFormat;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AtaxxBoard {
    bbs: [u64; 3],
    score: i16,
    result: u8,
}

const _RIGHT_SIZE: () = assert!(std::mem::size_of::<AtaxxBoard>() == 32);

impl BulletFormat for AtaxxBoard {
    type FeatureType = (u8, u8);
    const INPUTS: usize = 147;
    const MAX_FEATURES: usize = 49;

    fn score(&self) -> i16 {
        self.score
    }

    fn result(&self) -> f32 {
        f32::from(self.result) / 2.
    }

    fn result_idx(&self) -> usize {
        usize::from(self.result)
    }
}

impl IntoIterator for AtaxxBoard {
    type Item = (u8, u8);
    type IntoIter = AtaxxBoardIter;
    fn into_iter(self) -> Self::IntoIter {
        AtaxxBoardIter {
            board: self,
            stage: 0,
        }
    }
}

pub struct AtaxxBoardIter {
    board: AtaxxBoard,
    stage: usize,
}

impl Iterator for AtaxxBoardIter {
    type Item = (u8, u8);
    fn next(&mut self) -> Option<Self::Item> {
        if self.board.bbs[self.stage] == 0 {
            self.stage += 1;

            if self.stage > 2 {
                return None;
            }
        }

        let sq = self.board.bbs[self.stage].trailing_zeros();
        Some((self.stage as u8, sq as u8))
    }
}
