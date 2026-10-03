mod cudad;
mod marlin;

pub use cudad::{CudADFormat, CudADFormatIter};
pub use marlin::{MarlinFormat, MarlinFormatIter};

use crate::BulletFormat;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChessBoard {
    pub occ: u64,
    pub pcs: [u8; 16],
    pub score: i16,
    pub result: u8,
    pub ksq: u8,
    pub opp_ksq: u8,
    pub extra: [u8; 3],
}

const _RIGHT_SIZE: () = assert!(std::mem::size_of::<ChessBoard>() == 32);

impl BulletFormat for ChessBoard {
    type FeatureType = (u8, u8);

    const HEADER_SIZE: usize = 0;

    fn score(&self) -> i16 {
        self.score
    }

    fn result(&self) -> f32 {
        f32::from(self.result) / 2.
    }

    fn result_idx(&self) -> usize {
        usize::from(self.result)
    }

    fn set_result(&mut self, result: f32) {
        self.result = (2.0 * result) as u8;
    }
}

impl IntoIterator for ChessBoard {
    type Item = (u8, u8);
    type IntoIter = BoardIter;
    fn into_iter(self) -> Self::IntoIter {
        BoardIter {
            board: self,
            idx: 0,
        }
    }
}

pub struct BoardIter {
    board: ChessBoard,
    idx: usize,
}

impl Iterator for BoardIter {
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

impl ChessBoard {
    pub fn occ(&self) -> u64 {
        self.occ
    }

    pub fn our_ksq(&self) -> u8 {
        self.ksq
    }

    pub fn opp_ksq(&self) -> u8 {
        self.opp_ksq
    }

    pub fn extra(&self) -> [u8; 3] {
        self.extra
    }

    /// - Bitboards are in order White, Black, Pawn, Knight, Bishop, Rook, Queen, King.
    /// - Side-to-move is 0 for White, 1 for Black.
    /// - Score is White relative, in Centipawns.
    /// - Result is 0.0 for Black Win, 0.5 for Draw, 1.0 for White Win
    pub fn from_raw(
        mut bbs: [u64; 8],
        stm: usize,
        mut score: i16,
        mut result: f32,
    ) -> Result<Self, String> {
        if stm == 1 {
            for bb in bbs.iter_mut() {
                *bb = bb.swap_bytes();
            }

            bbs.swap(0, 1);

            score = -score;
            result = 1.0 - result;
        }

        let occ = bbs[0] | bbs[1];

        #[cfg(not(target_feature = "avx512vbmi2"))]
        let pcs = {
            let mut pcs = [0; 16];

            let mut idx = 0;
            let mut occ2 = occ;
            while occ2 > 0 {
                let sq = occ2.trailing_zeros();
                let bit = 1 << sq;
                occ2 &= occ2 - 1;

                let colour = u8::from((bit & bbs[1]) > 0) << 3;
                let piece = bbs
                    .iter()
                    .skip(2)
                    .position(|bb| bit & bb > 0)
                    .ok_or("No Piece Found!")?;

                let pc = colour | piece as u8;

                pcs[idx / 2] |= pc << (4 * (idx & 1));

                idx += 1;
            }
            pcs
        };

        #[cfg(target_feature = "avx512vbmi2")]
        let pcs = unsafe {
            use std::arch::x86_64::*;

            let black = bbs[1];
            let bbs = std::mem::transmute::<[u64; 8], __m512i>(bbs);

            // Transpose u64x8 to u8x64
            let bits = _mm512_gf2p8affine_epi64_epi8(
                _mm512_set1_epi64(0x8040201008040201u64 as i64),
                _mm512_permutexvar_epi8(
                    _mm512_set_epi8(
                        7, 15, 23, 31, 39, 47, 55, 63, 6, 14, 22, 30, 38, 46, 54, 62, 5, 13, 21,
                        29, 37, 45, 53, 61, 4, 12, 20, 28, 36, 44, 52, 60, 3, 11, 19, 27, 35, 43,
                        51, 59, 2, 10, 18, 26, 34, 42, 50, 58, 1, 9, 17, 25, 33, 41, 49, 57, 0, 8,
                        16, 24, 32, 40, 48, 56,
                    ),
                    bbs,
                ),
                0,
            );

            // Convert from one-hot representation to piece indexes
            let ptype_bits =
                _mm512_srli_epi16(_mm512_and_si512(bits, _mm512_set1_epi8(0xFCu8 as i8)), 2);
            let ptype = _mm512_popcnt_epi8(_mm512_sub_epi8(ptype_bits, _mm512_set1_epi8(1)));
            let ptype = _mm512_mask_add_epi8(ptype, black, ptype, _mm512_set1_epi8(8));

            // Extract only occupied squares
            let compressed = _mm512_castsi512_si256(_mm512_maskz_compress_epi8(occ, ptype));

            // Compress nibbles from u8x32 to u4x32
            let y = _mm256_maddubs_epi16(compressed, _mm256_set1_epi16(0x1001));
            let y = _mm_packus_epi16(_mm256_castsi256_si128(y), _mm256_extracti128_si256(y, 1));
            std::mem::transmute::<__m128i, [u8; 16]>(y)
        };

        let result = (2.0 * result) as u8;
        let ksq = (bbs[0] & bbs[7]).trailing_zeros() as u8;
        let opp_ksq = (bbs[1] & bbs[7]).trailing_zeros() as u8 ^ 56;

        Ok(Self {
            occ,
            pcs,
            score,
            result,
            ksq,
            opp_ksq,
            extra: [0; 3],
        })
    }
}

impl std::str::FromStr for ChessBoard {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        let split: Vec<_> = s.split('|').collect();

        let fen = split[0];
        let score = split.get(1).ok_or("Malformed!")?.trim();
        let wdl = split.get(2).ok_or("Malformed!")?.trim();

        let parts: Vec<&str> = fen.split_whitespace().collect();
        let board_str = *parts.first().ok_or("Malformed FEN!")?;
        let stm_str = *parts.get(1).ok_or("Malformed FEN!")?;

        let stm = u8::from(stm_str == "b");

        let mut board = Self::default();

        let mut idx = 0;

        let mut parse_row = |i: usize, row: &str| {
            let mut col = 0;
            for ch in row.chars() {
                if ('1'..='8').contains(&ch) {
                    col += ch.to_digit(10).expect("hard coded") as usize;
                } else if let Some(mut piece) = "PNBRQKpnbrqk".chars().position(|el| el == ch) {
                    let mut square = 8 * i + col;

                    piece = (piece / 6) << 3 | (piece % 6);

                    // black to move
                    if stm == 1 {
                        piece ^= 8;
                        square ^= 56;
                    }

                    if piece == 5 {
                        board.ksq = square as u8;
                    }

                    if piece == 13 {
                        board.opp_ksq = square as u8 ^ 56;
                    }

                    board.occ |= 1 << square;

                    if idx >= 32 {
                        return Err(s);
                    }

                    board.pcs[idx / 2] |= (piece as u8) << (4 * (idx & 1));
                    idx += 1;
                    col += 1;
                }
            }
            Ok(())
        };

        if stm == 1 {
            for (i, row) in board_str.split('/').enumerate() {
                parse_row(7 - i, row)?;
            }
        } else {
            for (i, row) in board_str.split('/').rev().enumerate() {
                parse_row(i, row)?;
            }
        }

        board.score = if let Ok(x) = score.parse::<i16>() {
            x
        } else {
            println!("{s}");
            return Err(String::from("Bad score!"));
        };

        board.result = match wdl {
            "1.0" | "[1.0]" | "1" => 2,
            "0.5" | "[0.5]" | "1/2" => 1,
            "0.0" | "[0.0]" | "0" => 0,
            _ => {
                println!("{s}");
                return Err(String::from("Bad game result!"));
            }
        };

        if stm == 1 {
            board.score = -board.score;
            board.result = 2 - board.result;
        }

        Ok(board)
    }
}

#[test]
fn from_raw_test() {
    let bbs: [u64; 8] = [
        0x91ff241018000000,
        0x0000800200737d91,
        0x00e7801208502d00,
        0x0000040010220000,
        0x0018000000014000,
        0x8100000000000081,
        0x0000200000001000,
        0x1000000000000010,
    ];
    let board = ChessBoard::from_raw(bbs, 0, 0, 0.0).unwrap();
    assert_eq!(board.occ, 0x91FFA41218737D91);
    assert_eq!(
        board.pcs,
        [
            0xDB, 0x8B, 0x88, 0x8C, 0xAA, 0x89, 0x89, 0x10, 0x8, 0x41, 0x8, 0x00, 0x22, 0x00, 0x30,
            0x35
        ]
    );
}
