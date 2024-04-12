use crate::{BulletFormat, ChessBoard};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CudADFormat {
    pcs: [u8; 16],
    occ: u64,
    mvcnt: u8,
    fmr: u8,
    stmr: u8,
    enp: u8,
    score: i16,
    wdl: i8,
}

impl IntoIterator for CudADFormat {
    type Item = (u8, u8);
    type IntoIter = CudADFormatIter;
    fn into_iter(self) -> Self::IntoIter {
        CudADFormatIter {
            board: self,
            idx: 0,
        }
    }
}

pub struct CudADFormatIter {
    board: CudADFormat,
    idx: usize,
}

impl Iterator for CudADFormatIter {
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

impl CudADFormat {
    pub fn occ(&self) -> u64 {
        self.occ
    }

    fn is_black_to_move(&self) -> bool {
        self.stmr >> 7 > 0
    }

    fn res_stm(&self) -> u8 {
        let r = if self.is_black_to_move() {
            1 - self.wdl
        } else {
            1 + self.wdl
        };

        assert!(r >= 0);

        r as u8
    }
}

impl BulletFormat for CudADFormat {
    type FeatureType = (u8, u8);

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
        self.wdl = (2.0 * result - 1.0) as i8;
    }
}

impl From<CudADFormat> for ChessBoard {
    fn from(cudad: CudADFormat) -> Self {
        let mut board = Self::default();

        let stm = cudad.is_black_to_move();

        if stm {
            board.score = -cudad.score;
        } else {
            board.score = cudad.score;
        }

        board.result = cudad.res_stm();

        let mut features = [(0, 0); 32];
        let mut fidx = 0;

        for (mut piece, mut square) in cudad.into_iter() {
            if stm {
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
