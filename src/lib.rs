#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
extern crate alloc;

use alloc::vec::Vec;
use stylus_sdk::{
    alloy_primitives::{Address, Uint},
    prelude::*,
};

mod constants;
use crate::constants::BOARD_SIZE;

sol_storage! {
    #[entrypoint]
    pub struct GoGame {
        mapping(address => uint128) game_boards;
        mapping(address => uint32) white_captures;
        mapping(address => uint32) black_captures;
        mapping(address => uint8) last_move_x;
        mapping(address => uint8) last_move_y;
        
        mapping(address => bool) player_passed;
        mapping(address => bool) contract_passed;
        mapping(address => bool) game_ended;
        
        mapping(address => uint32) player_points;
        mapping(uint32 => address) player_address_by_index;
        mapping(address => uint32) player_index;
        mapping(uint32 => uint32) player_rank;

        uint32 total_players;
    }
}

#[public]
impl GoGame {
    pub fn create_game(&mut self) {
        let player = self.vm().msg_sender();
        
        let special_board = 1u128 << 127; 
        self.game_boards.insert(player, Uint::<128, 2>::from(special_board));
        
        self.white_captures.insert(player, Uint::<32, 1>::from(0u32));
        self.black_captures.insert(player, Uint::<32, 1>::from(0u32));
        self.last_move_x.insert(player, Uint::<8, 1>::from(0u8));
        self.last_move_y.insert(player, Uint::<8, 1>::from(0u8));
        self.player_passed.insert(player, false);
        self.contract_passed.insert(player, false);
        self.game_ended.insert(player, false);
        
        if self.player_points.get(player) == Uint::<32, 1>::from(0u32) {
            self.player_points.insert(player, Uint::<32, 1>::from(0u32));
            self.total_players.set(self.total_players.get() + Uint::<32, 1>::from(1u32));
        }
    }
    
    pub fn has_game(&self, player: Address) -> bool {
        self.game_boards.get(player) != Uint::<128, 2>::from(0u128) && 
        !self.game_ended.get(player)
    }
    
    pub fn get_board(&self, player: Address) -> u128 {
        let board = self.game_boards.get(player);
        board.try_into().unwrap_or(0u128)
    }
    
    pub fn get_board_as_array(&self, player: Address) -> Vec<Vec<u8>> {
        let board: u128 = self.game_boards.get(player).try_into().unwrap_or(0u128);
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
    
    pub fn get_player_points(&self, player: Address) -> u32 {
        self.player_points.get(player).try_into().unwrap_or(0)
    }
    
    pub fn get_total_players(&self) -> u32 {
        self.total_players.get().try_into().unwrap_or(0)
    }
    
    pub fn set_piece(&mut self, x: u8, y: u8) {
        let player = self.vm().msg_sender();
        
        assert!(self.has_game(player), "No active game found");
        assert!(!self.game_ended.get(player), "Game already ended");
        assert!(self.is_valid_position(x, y), "Invalid position");
        
        let board = self.get_board(player);
        
        let stone = self.get_stone_at_position(board, x, y);
        assert!(stone == 0, "Position is already occupied");
        
        assert!(!self.is_ko_violation(player, x, y), "Move violates Ko rule");
        
        assert!(!self.would_be_suicide(board, x, y, 1), "Move would be suicide");
        
        let mut updated_board = self.set_stone_at_position(board, x, y, 1);
        
        let (board_after_capture, captured_stones, ko_x, ko_y) = self.capture_surrounded_stones(updated_board, x, y, 1);
        updated_board = board_after_capture;
        
        let white_captures: u32 = self.white_captures.get(player).try_into().unwrap_or(0) + captured_stones;
        let black_captures: u32 = self.black_captures.get(player).try_into().unwrap_or(0);
        
        let last_x = ko_x;
        let last_y = ko_y;
        
        self.player_passed.insert(player, false);
        
        self.update_game(player, updated_board, white_captures, black_captures, last_x, last_y);
        
        self.make_contract_move(player);
    }
    
    pub fn pass_turn(&mut self) {
        let player = self.vm().msg_sender();
        
        assert!(self.has_game(player), "No active game found");
        assert!(!self.game_ended.get(player), "Game already ended");
        
        self.player_passed.insert(player, true);
        
        self.make_contract_move(player);
    }
    
    pub fn is_game_ended(&self, player: Address) -> bool {
        self.game_ended.get(player)
    }
    
    pub fn get_game_result(&self, player: Address) -> (u32, u32, u8) {
        let white_captures = self.white_captures.get(player).try_into().unwrap_or(0);
        let black_captures = self.black_captures.get(player).try_into().unwrap_or(0);
        
        let winner = if white_captures > black_captures {
            1u8 
        } else if white_captures < black_captures {
            2u8 
        } else {
            0u8 
        };
        
        (white_captures, black_captures, winner)
    }

    pub fn get_top_players(&self) -> Vec<(Address, u32)> {
        let max_players_to_fetch = 10u32;
        let total_existing_players = self.total_players.get().try_into().unwrap_or(0);
        let max_players = max_players_to_fetch.min(total_existing_players);
        
        let mut leaderboard = Vec::with_capacity(max_players as usize);
        
        let mut all_players = Vec::with_capacity(total_existing_players as usize);
        
        for i in 1..=total_existing_players {
            let player_addr = self.player_address_by_index.get(Uint::<32, 1>::from(i));
            if player_addr != Address::ZERO {
                let points = self.player_points.get(player_addr).try_into().unwrap_or(0);
                all_players.push((player_addr, points));
            }
        }
        
        all_players.sort_by(|a, b| b.1.cmp(&a.1));
        
        for i in 0..max_players as usize {
            if i < all_players.len() {
                leaderboard.push(all_players[i]);
            }
        }
        
        leaderboard
    }
    
    pub fn get_player_rank(&self, player: Address) -> u32 {
        let player_points = self.player_points.get(player).try_into().unwrap_or(0);
        if player_points == 0 {
            return 0;
        }
        
        let mut rank = 1;
        
        for i in 1..=self.total_players.get().try_into().unwrap_or(0) {
            let other_player = self.player_address_by_index.get(Uint::<32, 1>::from(i));
            if other_player != player && other_player != Address::ZERO {
                let other_points = self.player_points.get(other_player).try_into().unwrap_or(0);
                if other_points > player_points {
                    rank += 1;
                }
            }
        }
        
        rank
    }
}

impl GoGame {
    fn update_game(&mut self, 
                  player: Address, 
                  board: u128,
                  white_captures: u32, 
                  black_captures: u32, 
                  last_move_x: u8, 
                  last_move_y: u8) {
        self.game_boards.insert(player, Uint::<128, 2>::from(board));
        self.white_captures.insert(player, Uint::<32, 1>::from(white_captures));
        self.black_captures.insert(player, Uint::<32, 1>::from(black_captures));
        self.last_move_x.insert(player, Uint::<8, 1>::from(last_move_x));
        self.last_move_y.insert(player, Uint::<8, 1>::from(last_move_y));
    }
    
    fn get_stone_at_position(&self, board: u128, x: u8, y: u8) -> u8 {
        let position = y as usize * BOARD_SIZE + x as usize;
        let shift = position * 2;
        
        if shift >= 98 { 
            return 0;
        }
        
        let value = (board >> shift) & 0b11;
        value as u8
    }
    
    fn set_stone_at_position(&self, board: u128, x: u8, y: u8, stone: u8) -> u128 {
        let position = y as usize * BOARD_SIZE + x as usize;
        let shift = position * 2;
        if shift >= 98 {
             return board;
        }
        let cleared_board = board & !(0b11u128 << shift);
        cleared_board | ((stone as u128) << shift)
    }
    
    fn is_valid_position(&self, x: u8, y: u8) -> bool {
        x < BOARD_SIZE as u8 && y < BOARD_SIZE as u8
    }
    
    fn count_liberties(&self, board: u128, x: u8, y: u8) -> u32 {
        let stone_color = self.get_stone_at_position(board, x, y);
        if stone_color == 0 {
            return 0; 
        }
        
        let mut visited: u128 = 0;
        let mut liberty_set: u128 = 0;
        
        let mut stack = Vec::with_capacity(BOARD_SIZE);
        stack.push((x, y));
        
        while let Some((curr_x, curr_y)) = stack.pop() {
            let pos_idx = curr_y as usize * BOARD_SIZE + curr_x as usize;
            let pos_bit = 1u128 << pos_idx;

            if visited & pos_bit != 0 {
                continue;
            }
            
            visited |= pos_bit;
            
            // Right
            if curr_x + 1 < BOARD_SIZE as u8 {
                let nx = curr_x + 1;
                let neighbor_stone = self.get_stone_at_position(board, nx, curr_y);
                let neighbor_idx = curr_y as usize * BOARD_SIZE + nx as usize;
                let neighbor_bit = 1u128 << neighbor_idx;
                
                if neighbor_stone == 0 {
                    liberty_set |= neighbor_bit;
                } else if neighbor_stone == stone_color && (visited & neighbor_bit) == 0 {
                    stack.push((nx, curr_y));
                }
            }
            
            // Down
            if curr_y + 1 < BOARD_SIZE as u8 {
                let ny = curr_y + 1;
                let neighbor_stone = self.get_stone_at_position(board, curr_x, ny);
                let neighbor_idx = ny as usize * BOARD_SIZE + curr_x as usize;
                let neighbor_bit = 1u128 << neighbor_idx;
                
                if neighbor_stone == 0 {
                    liberty_set |= neighbor_bit;
                } else if neighbor_stone == stone_color && (visited & neighbor_bit) == 0 {
                    stack.push((curr_x, ny));
                }
            }
            
            // Left
            if curr_x > 0 {
                let nx = curr_x - 1;
                let neighbor_stone = self.get_stone_at_position(board, nx, curr_y);
                let neighbor_idx = curr_y as usize * BOARD_SIZE + nx as usize;
                let neighbor_bit = 1u128 << neighbor_idx;
                
                if neighbor_stone == 0 {
                    liberty_set |= neighbor_bit;
                } else if neighbor_stone == stone_color && (visited & neighbor_bit) == 0 {
                    stack.push((nx, curr_y));
                }
            }
            
            // Up
            if curr_y > 0 {
                let ny = curr_y - 1;
                let neighbor_stone = self.get_stone_at_position(board, curr_x, ny);
                let neighbor_idx = ny as usize * BOARD_SIZE + curr_x as usize;
                let neighbor_bit = 1u128 << neighbor_idx;
                
                if neighbor_stone == 0 {
                    liberty_set |= neighbor_bit;
                } else if neighbor_stone == stone_color && (visited & neighbor_bit) == 0 {
                    stack.push((curr_x, ny));
                }
            }
        }
        
        liberty_set.count_ones()
    }
    
    fn make_contract_move(&mut self, player: Address) {
        let board = self.get_board(player);
        
        let mut found_move = false;
        let mut contract_x = 0u8;
        let mut contract_y = 0u8;
        
        let center = BOARD_SIZE as u8 / 2;
        
        for y_offset in 0..=1 {
            for x_offset in 0..=1 {
                let try_y = center.saturating_add(y_offset).min(BOARD_SIZE as u8 - 1);
                let try_x = center.saturating_add(x_offset).min(BOARD_SIZE as u8 - 1);
                
                for (x, y) in &[(try_x, try_y), 
                               (center.saturating_sub(x_offset), try_y),
                               (try_x, center.saturating_sub(y_offset)),
                               (center.saturating_sub(x_offset), center.saturating_sub(y_offset))] {
                    if self.get_stone_at_position(board, *x, *y) == 0 && 
                       !self.would_be_suicide(board, *x, *y, 2) &&
                       !self.is_ko_violation(player, *x, *y) {
                        contract_x = *x;
                        contract_y = *y;
                        found_move = true;
                        break;
                    }
                }
                
                if found_move {
                    break;
                }
            }
            
            if found_move {
                break;
            }
        }
        
        if !found_move {
            for radius in 1..BOARD_SIZE as u8 {
                for y in center.saturating_sub(radius)..=center.saturating_add(radius).min(BOARD_SIZE as u8 - 1) {
                    for x in center.saturating_sub(radius)..=center.saturating_add(radius).min(BOARD_SIZE as u8 - 1) {
                        if x == center.saturating_sub(radius) || 
                           x == center.saturating_add(radius).min(BOARD_SIZE as u8 - 1) ||
                           y == center.saturating_sub(radius) || 
                           y == center.saturating_add(radius).min(BOARD_SIZE as u8 - 1) {
                            
                            if self.get_stone_at_position(board, x, y) == 0 && 
                               !self.would_be_suicide(board, x, y, 2) &&
                               !self.is_ko_violation(player, x, y) {
                                contract_x = x;
                                contract_y = y;
                                found_move = true;
                                break;
                            }
                        }
                    }
                    
                    if found_move {
                        break;
                    }
                }
                
                if found_move {
                    break;
                }
            }
        }
        
        if !found_move {
            for y in 0..BOARD_SIZE {
                for x in 0..BOARD_SIZE {
                    if self.get_stone_at_position(board, x as u8, y as u8) == 0 && 
                       !self.would_be_suicide(board, x as u8, y as u8, 2) &&
                       !self.is_ko_violation(player, x as u8, y as u8) {
                        contract_x = x as u8;
                        contract_y = y as u8;
                        found_move = true;
                        break;
                    }
                }
                if found_move {
                    break;
                }
            }
        }
        
        if found_move {
            let mut updated_board = self.set_stone_at_position(board, contract_x, contract_y, 2);
            
            let (board_after_capture, captured_stones, ko_x, ko_y) = 
                self.capture_surrounded_stones(updated_board, contract_x, contract_y, 2);
            updated_board = board_after_capture;
            
            let white_captures: u32 = self.white_captures.get(player).try_into().unwrap_or(0);
            let black_captures: u32 = self.black_captures.get(player).try_into().unwrap_or(0) + captured_stones;
            
            self.contract_passed.insert(player, false);
            
            self.update_game(player, updated_board, white_captures, black_captures, ko_x, ko_y);
            self.check_for_game_end(player);
        } else {
            self.contract_passed.insert(player, true);
            
            if self.player_passed.get(player) {
                self.end_game(player);
            }
        }
    }
    
    fn check_for_game_end(&mut self, player: Address) {
        if self.player_passed.get(player) && self.contract_passed.get(player) {
            self.end_game(player);
            return;
        }
        
        let board = self.get_board(player);
        let mut is_full = true;
        
        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                if self.get_stone_at_position(board, x as u8, y as u8) == 0 {
                    is_full = false;
                    break;
                }
            }
            if !is_full {
                break;
            }
        }
        
        if is_full {
            self.end_game(player);
        }
    }
    
    fn end_game(&mut self, player: Address) {
        assert!(!self.game_ended.get(player), "Game already ended");
        
        let white_captures = self.white_captures.get(player).try_into().unwrap_or(0);
        let black_captures = self.black_captures.get(player).try_into().unwrap_or(0);
        
        let player_points = self.player_points.get(player).try_into().unwrap_or(0);
        
        let new_points = if white_captures > black_captures {
            player_points + 3
        } else if white_captures < black_captures {
            player_points + 1
        } else {
            player_points + 2
        };
        
        self.update_player_points(player, new_points);
        
        self.game_boards.insert(player, Uint::<128, 2>::from(0u128));
        self.white_captures.insert(player, Uint::<32, 1>::from(0u32));
        self.black_captures.insert(player, Uint::<32, 1>::from(0u32));
        self.last_move_x.insert(player, Uint::<8, 1>::from(0u8));
        self.last_move_y.insert(player, Uint::<8, 1>::from(0u8));
        self.player_passed.insert(player, false);
        self.contract_passed.insert(player, false);
        self.game_ended.insert(player, true);
    }
    
    fn would_be_suicide(&self, board: u128, x: u8, y: u8, stone_color: u8) -> bool {
        if self.would_capture_opponent_stones(board, x, y, stone_color) {
            return false;
        }
        let temp_board = self.set_stone_at_position(board, x, y, stone_color);
        let liberties = self.count_liberties(temp_board, x, y);
        liberties == 0
    }
    
    fn would_capture_opponent_stones(&self, board: u128, x: u8, y: u8, stone_color: u8) -> bool {
        let opponent_color = if stone_color == 1 { 2 } else { 1 };
        let neighbors = [
            (x + 1, y),     // Right
            (x, y + 1),     // Down
            (x.wrapping_sub(1), y), // Left
            (x, y.wrapping_sub(1)), // Up
        ];
        
        for (nx, ny) in neighbors.iter() {
            if !self.is_valid_position(*nx, *ny) {
                continue;
            }
            
            let neighbor_stone = self.get_stone_at_position(board, *nx, *ny);
            if neighbor_stone == opponent_color {
                let opponent_liberties = self.count_liberties(board, *nx, *ny);
                if opponent_liberties == 1 {
                    let mut is_our_position_liberty = false;
                    if self.get_stone_at_position(board, x, y) == 0 {
                        for (check_x, check_y) in neighbors.iter() {
                            if self.is_valid_position(*check_x, *check_y) && 
                               *check_x == *nx && *check_y == *ny {
                                is_our_position_liberty = true;
                                break;
                            }
                        }
                    }
                    
                    if is_our_position_liberty {
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    fn capture_surrounded_stones(&mut self, board: u128, x: u8, y: u8, stone_color: u8) -> (u128, u32, u8, u8) {
        let opponent_color = if stone_color == 1 { 2 } else { 1 };
        let mut captured_count = 0;
        let mut updated_board = board;
        let mut ko_x = 0;
        let mut ko_y = 0;
        
        let neighbors = [
            (x + 1, y),     // Right
            (x, y + 1),     // Down
            (x.wrapping_sub(1), y), // Left
            (x, y.wrapping_sub(1)), // Up
        ];
        
        for (nx, ny) in neighbors.iter() {
            if !self.is_valid_position(*nx, *ny) {
                continue;
            }
            
            let neighbor_stone = self.get_stone_at_position(updated_board, *nx, *ny);
            if neighbor_stone == opponent_color {
                let liberties = self.count_liberties(updated_board, *nx, *ny);
                
                if liberties == 0 {
                    let (new_board, stones_removed, removed_positions) = self.remove_group(updated_board, *nx, *ny);
                    updated_board = new_board;
                    captured_count += stones_removed;
                    
                    if stones_removed == 1 && !removed_positions.is_empty() {
                        let captured_pos = removed_positions[0];
                        ko_x = captured_pos.0;
                        ko_y = captured_pos.1;
                    }
                }
            }
        }
        
        (updated_board, captured_count, ko_x, ko_y)
    }
    
    fn remove_group(&self, board: u128, x: u8, y: u8) -> (u128, u32, Vec<(u8, u8)>) {
        let stone_color = self.get_stone_at_position(board, x, y);
        if stone_color == 0 {
            return (board, 0, Vec::new());
        }
        
        let mut visited: u128 = 0;
        let mut group_stones: u128 = 0;
        let mut count = 0;
        let mut removed_positions = Vec::new();
        
        let mut stack = Vec::with_capacity(BOARD_SIZE);
        stack.push((x, y));
        
        while let Some((curr_x, curr_y)) = stack.pop() {
            let pos_idx = curr_y as usize * BOARD_SIZE + curr_x as usize;
            let pos_bit = 1u128 << pos_idx;
            
            if visited & pos_bit != 0 {
                continue;
            }
            
            visited |= pos_bit;
            
            let curr_stone = self.get_stone_at_position(board, curr_x, curr_y);
            if curr_stone != stone_color {
                continue;
            }
            
            group_stones |= pos_bit;
            count += 1;
            removed_positions.push((curr_x, curr_y));
            
            let directions = [
                (curr_x + 1, curr_y),     // Right
                (curr_x, curr_y + 1),     // Down
                (curr_x.wrapping_sub(1), curr_y), // Left
                (curr_x, curr_y.wrapping_sub(1)), // Up
            ];
            
            for (nx, ny) in directions.iter() {
                if self.is_valid_position(*nx, *ny) {
                    let next_stone = self.get_stone_at_position(board, *nx, *ny);
                    if next_stone == stone_color {
                        stack.push((*nx, *ny));
                    }
                }
            }
        }
        
        let mut new_board = board;
        
        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                let pos_idx = y * BOARD_SIZE + x;
                let pos_bit = 1u128 << pos_idx;
                
                if (group_stones & pos_bit) != 0 {
                    new_board = self.set_stone_at_position(new_board, x as u8, y as u8, 0);
                }
            }
        }
        
        (new_board, count, removed_positions)
    }
    
    fn is_ko_violation(&self, player: Address, x: u8, y: u8) -> bool {
        let last_x = self.last_move_x.get(player).try_into().unwrap_or(0);
        let last_y = self.last_move_y.get(player).try_into().unwrap_or(0);
        
        if last_x == 0 && last_y == 0 {
            return false;
        }
        
        x == last_x && y == last_y
    }
    
    fn update_player_points(&mut self, player: Address, new_points: u32) {
        let current_points = self.player_points.get(player).try_into().unwrap_or(0);
        
        if self.player_index.get(player) == Uint::<32, 1>::from(0) && current_points == 0 {
            let index = self.total_players.get().try_into().unwrap_or(0) + 1;
            self.player_index.insert(player, Uint::<32, 1>::from(index));
            self.player_address_by_index.insert(Uint::<32, 1>::from(index), player);
            self.total_players.set(Uint::<32, 1>::from(index));
        }
        
        self.player_points.insert(player, Uint::<32, 1>::from(new_points));
    }
}