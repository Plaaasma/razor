//! Zobrist hashing. Keys generated at compile time from SplitMix64.

use crate::types::{Color, PieceType, Square};

const fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

pub struct Keys {
    /// [color][piece][square]
    pub piece: [[[u64; 64]; 6]; 2],
    /// [castling rights 0..16]
    pub castling: [u64; 16],
    /// [en passant file]
    pub ep_file: [u64; 8],
    pub side_to_move: u64,
}

const fn generate() -> Keys {
    let mut state = 0x0052_415a_4f52_2121u64; // seed: "RAZOR!!"
    let mut keys = Keys {
        piece: [[[0; 64]; 6]; 2],
        castling: [0; 16],
        ep_file: [0; 8],
        side_to_move: 0,
    };
    let mut c = 0;
    while c < 2 {
        let mut p = 0;
        while p < 6 {
            let mut s = 0;
            while s < 64 {
                keys.piece[c][p][s] = splitmix64(&mut state);
                s += 1;
            }
            p += 1;
        }
        c += 1;
    }
    let mut i = 0;
    while i < 16 {
        keys.castling[i] = splitmix64(&mut state);
        i += 1;
    }
    let mut f = 0;
    while f < 8 {
        keys.ep_file[f] = splitmix64(&mut state);
        f += 1;
    }
    keys.side_to_move = splitmix64(&mut state);
    keys
}

pub static KEYS: Keys = generate();

#[inline(always)]
pub fn piece_key(c: Color, pt: PieceType, sq: Square) -> u64 {
    KEYS.piece[c.idx()][pt.idx()][sq as usize]
}
