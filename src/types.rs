use stylus_sdk::alloy_primitives::Address;

pub const BOARD_SIZE: usize = 7;
pub const EMPTY_BOARD: u64 = 0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Empty = 0,
    White = 1, 
    Black = 2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: u8,
    pub y: u8,
}

pub struct PlayerStats {
    pub player: Address,
    pub total_captures: u32,
    pub games_played: u32,
}

pub struct GameResult {
    pub white_captures: u32,
    pub black_captures: u32,
    pub winner: Color,
}
