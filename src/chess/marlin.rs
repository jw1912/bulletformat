use crate::{BulletFormat, ChessBoard};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MarlinFormat {
    occ: u64,
    pcs: [u8; 16],
    stm_enp: u8,
    hfm: u8,
    fmc: u16,
    score: i16,
    result: u8,
    extra: u8,
}

impl IntoIterator for MarlinFormat {
    type Item = (u8, u8);
    type IntoIter = MarlinFormatIter;
    fn into_iter(self) -> Self::IntoIter {
        MarlinFormatIter {
            board: self,
            idx: 0,
        }
    }
}

pub struct MarlinFormatIter {
    board: MarlinFormat,
    idx: usize,
}

impl Iterator for MarlinFormatIter {
    type Item = (u8, u8);
    fn next(&mut self) -> Option<Self::Item> {
        if self.board.occ == 0 {
            return None;
        }

        let square = self.board.occ.trailing_zeros() as u8;
        let piece = (self.board.pcs[self.idx / 2] >> (4 * (self.idx & 1))) & 0b1111;

        self.board.occ &= self.board.occ - 1;
        self.idx += 1;

        Some((piece, square))
    }
}

impl MarlinFormat {
    pub fn occ(&self) -> u64 {
        self.occ
    }

    fn is_black_to_move(&self) -> bool {
        self.stm_enp >> 7 > 0
    }

    fn res_stm(&self) -> u8 {
        if self.is_black_to_move() {
            2 - self.result
        } else {
            self.result
        }
    }
}

impl BulletFormat for MarlinFormat {
    type FeatureType = (u8, u8);

    const HEADER_SIZE: usize = 0;

    fn score(&self) -> i16 {
        if self.is_black_to_move() {
            -self.score
        } else {
            self.score
        }
    }

    fn result(&self) -> f32 {
        f32::from(self.res_stm()) / 2.
    }

    fn result_idx(&self) -> usize {
        usize::from(self.res_stm())
    }

    fn set_result(&mut self, result: f32) {
        self.result = (2.0 * result) as u8;
    }
}

impl From<MarlinFormat> for ChessBoard {
    fn from(mf: MarlinFormat) -> Self {
        let mut board = Self::default();

        let stm = usize::from(mf.stm_enp >> 7);

        if stm == 1 {
            board.score = -mf.score;
            board.result = 2 - mf.result;
        } else {
            board.score = mf.score;
            board.result = mf.result;
        }

        let mut features = [(0, 0); 32];
        let mut fidx = 0;

        for (mut piece, mut square) in mf.into_iter() {
            if stm == 1 {
                piece ^= 8;
                square ^= 56;
            }

            if piece == 5 {
                board.ksq = square;
            }

            if piece == 13 {
                board.opp_ksq = square ^ 56;
            }

            features[fidx] = (piece, square);
            fidx += 1;
        }

        features[..fidx].sort_by_key(|feat| feat.1);

        for (idx, (piece, square)) in features.iter().enumerate().take(fidx) {
            board.occ |= 1 << square;
            board.pcs[idx / 2] |= piece << (4 * (idx & 1));
        }

        board
    }
}
