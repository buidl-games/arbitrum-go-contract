//!
//! Stylish Go Game
//!

#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
extern crate alloc;

use stylus_sdk::{
    alloy_primitives::{Address, U256},
    prelude::*,
    msg,
};

mod types;
use crate::types::{Color, Position, BOARD_SIZE, EMPTY_BOARD};

sol_storage! {
    #[entrypoint]
    pub struct GoGame {
        mapping(address => uint64) game_boards;
        
        mapping(address => address) game_players;
        
        mapping(address => uint32) white_captures;
        mapping(address => uint32) black_captures;
        
        mapping(address => uint8) last_move_x;
        mapping(address => uint8) last_move_y;
    }
}

#[public]
impl GoGame {
    
}

#[cfg(test)]
mod test {
    use super::*;

    
}
