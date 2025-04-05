//!
//! Stylish Go Game
//!

#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
extern crate alloc;

use alloc::vec::Vec;
use stylus_sdk::{
    alloy_primitives::{Address, Uint},
    prelude::*,
};

mod types;
use crate::types::{BOARD_SIZE, EMPTY_BOARD};

sol_storage! {
    #[entrypoint]
    pub struct GoGame {
        // Game state storage
        mapping(address => uint64) game_boards;
        mapping(address => uint32) white_captures;
        mapping(address => uint32) black_captures;
        mapping(address => uint8) last_move_x;
        mapping(address => uint8) last_move_y;
        
        // Leaderboard storage
        mapping(address => uint32) player_points;

        uint32 total_players;
    }
}

#[public]
impl GoGame {
    pub fn create_game(&mut self) {
        let player = self.vm().msg_sender();
        
        self.game_boards.insert(player, Uint::<64, 1>::from(EMPTY_BOARD));
        self.white_captures.insert(player, Uint::<32, 1>::from(0u32));
        self.black_captures.insert(player, Uint::<32, 1>::from(0u32));
        self.last_move_x.insert(player, Uint::<8, 1>::from(0u8));
        self.last_move_y.insert(player, Uint::<8, 1>::from(0u8));
        
        if self.player_points.get(player) == Uint::<32, 1>::from(0u32) {
            self.player_points.insert(player, Uint::<32, 1>::from(0u32));
            self.total_players.set(self.total_players.get() + Uint::<32, 1>::from(1u32));
        }
    }
    
    pub fn has_game(&self, player: Address) -> bool {
        self.game_boards.get(player) != Uint::<64, 1>::from(0u64)
    }
    
    fn update_game(&mut self, 
                  player: Address, 
                  board: u64, 
                  white_captures: u32, 
                  black_captures: u32, 
                  last_move_x: u8, 
                  last_move_y: u8) {
        self.game_boards.insert(player, Uint::<64, 1>::from(board));
        self.white_captures.insert(player, Uint::<32, 1>::from(white_captures));
        self.black_captures.insert(player, Uint::<32, 1>::from(black_captures));
        self.last_move_x.insert(player, Uint::<8, 1>::from(last_move_x));
        self.last_move_y.insert(player, Uint::<8, 1>::from(last_move_y));
    }
    
    pub fn get_board(&self, player: Address) -> u64 {
        let board = self.game_boards.get(player);
        board.try_into().unwrap_or(0)
    }
    
    pub fn get_board_as_array(&self, player: Address) -> Vec<Vec<u8>> {
        let board: u64 = self.game_boards.get(player).try_into().unwrap_or(0);
        let mut result = Vec::with_capacity(BOARD_SIZE);
        
        for y in 0..BOARD_SIZE {
            let mut row = Vec::with_capacity(BOARD_SIZE);
            for x in 0..BOARD_SIZE {
                let stone = self.get_stone_at_position(board, x as u8, y as u8);
                row.push(stone);
            }
            result.push(row);
        }
        
        result
    }
    
    fn get_stone_at_position(&self, board: u64, x: u8, y: u8) -> u8 {
        let position = y as usize * BOARD_SIZE + x as usize;
        let shift = position * 2;
        let value = (board >> shift) & 0b11;
        
        value as u8
    }
    
    fn set_stone_at_position(&self, board: u64, x: u8, y: u8, stone: u8) -> u64 {
        let position = y as usize * BOARD_SIZE + x as usize;
        let shift = position * 2;
        
        // Clear the bits at the position
        let cleared_board = board & !(0b11 << shift);
        
        // Set the new stone value
        cleared_board | ((stone as u64) << shift)
    }
    
    fn is_valid_position(&self, x: u8, y: u8) -> bool {
        x < BOARD_SIZE as u8 && y < BOARD_SIZE as u8
    }
    
    pub fn get_player_points(&self, player: Address) -> u32 {
        self.player_points.get(player).try_into().unwrap_or(0)
    }
    
    pub fn get_total_players(&self) -> u32 {
        self.total_players.get().try_into().unwrap_or(0)
    }
    
    pub fn set_piece(&mut self, x: u8, y: u8) {
        let player = self.vm().msg_sender();
        
        assert!(self.has_game(player), "No active game found");
        
        assert!(self.is_valid_position(x, y), "Invalid position");
        
        let board = self.get_board(player);
        
        let stone = self.get_stone_at_position(board, x, y);
        assert!(stone == 0, "Position is already occupied");
        
        let updated_board = self.set_stone_at_position(board, x, y, 1);
        
        // TODO: Capture surrounded black stones
        
        let white_captures: u32 = self.white_captures.get(player).try_into().unwrap_or(0);
        let black_captures: u32 = self.black_captures.get(player).try_into().unwrap_or(0);
        
        let last_x = x;
        let last_y = y;
        
        self.update_game(player, updated_board, white_captures, black_captures, last_x, last_y);
        
        self.make_contract_move(player);
    }
    
    fn make_contract_move(&mut self, player: Address) {
        let board = self.get_board(player);
        
        let mut found_move = false;
        let mut contract_x = 0u8;
        let mut contract_y = 0u8;
        
        'outer: for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                let stone = self.get_stone_at_position(board, x as u8, y as u8);
                if stone == 0 {
                    contract_x = x as u8;
                    contract_y = y as u8;
                    found_move = true;
                    break 'outer;
                }
            }
        }
        
        if found_move {
            let updated_board = self.set_stone_at_position(board, contract_x, contract_y, 2);
            
            // TODO: Capture surrounded white stones
            
            let white_captures: u32 = self.white_captures.get(player).try_into().unwrap_or(0);
            let black_captures: u32 = self.black_captures.get(player).try_into().unwrap_or(0);
            
            self.update_game(player, updated_board, white_captures, black_captures, contract_x, contract_y);
        }
    }
}
