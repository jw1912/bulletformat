use crate::BulletFormat;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChessBoard {
    occ: u64,
    pcs: [u8; 16],
    score: i16,
    result: u8,
    ksq: u8,
    opp_ksq: u8,
}

const _RIGHT_SIZE: () = assert!(std::mem::size_of::<ChessBoard>() == 32);

impl BulletFormat for ChessBoard {
    type FeatureType = (u8, u8);

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
                .ok_or("No Piece Found!".to_string())?;

            let pc = colour | piece as u8;

            pcs[idx / 2] |= pc << (4 * (idx & 1));

            idx += 1;
        }

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
}
