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
    const INPUTS: usize = 768;
    const MAX_FEATURES: usize = 32;

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
    type Item = (u8, u8, u8, u8);
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
    type Item = (u8, u8, u8, u8);
    fn next(&mut self) -> Option<Self::Item> {
        if self.board.occ == 0 {
            return None;
        }

        let square = self.board.occ.trailing_zeros() as u8;
        let piece = (self.board.pcs[self.idx / 2] >> (4 * (self.idx & 1))) & 0b1111;

        self.board.occ &= self.board.occ - 1;
        self.idx += 1;

        Some((piece, square, self.board.ksq, self.board.opp_ksq))
    }
}

impl ChessBoard {
    pub fn from_epd(epd: &str) -> Result<Self, String> {
        let split: Vec<_> = epd.split('|').collect();

        let fen = split[0];
        let score = split[1].trim();
        let wdl = split[2].trim();

        let parts: Vec<&str> = fen.split_whitespace().collect();
        let board_str = parts[0];
        let stm_str = parts[1];

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
                        return Err(epd);
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
            println!("{epd}");
            return Err(String::from("Bad score!"));
        };

        board.result = match wdl {
            "1.0" | "[1.0]" | "1" => 2,
            "0.5" | "[0.5]" | "1/2" => 1,
            "0.0" | "[0.0]" | "0" => 0,
            _ => {
                println!("{epd}");
                return Err(String::from("Bad game result!"));
            }
        };

        if stm == 1 {
            board.score = -board.score;
            board.result = 2 - board.result;
        }

        Ok(board)
    }

    pub fn from_marlinformat(mf: &MarlinFormat) -> Self {
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

        for (colour, mut piece, mut square) in mf.into_iter() {
            piece |= colour << 3;

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
    type Item = (u8, u8, u8);
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
    type Item = (u8, u8, u8);
    fn next(&mut self) -> Option<Self::Item> {
        if self.board.occ == 0 {
            return None;
        }

        let square = self.board.occ.trailing_zeros() as u8;
        let coloured_piece = (self.board.pcs[self.idx / 2] >> (4 * (self.idx & 1))) & 0b1111;

        let mut piece = coloured_piece & 0b111;
        if piece == 6 {
            piece = 3;
        }

        let colour = coloured_piece >> 3;

        self.board.occ &= self.board.occ - 1;
        self.idx += 1;

        Some((colour, piece, square))
    }
}
