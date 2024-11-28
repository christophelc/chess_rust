pub mod table;
use table::table_bishop;
use table::table_rook;

use super::zobrist;
use super::BitBoard;
use super::BitBoardMove;
use super::BitBoardsWhiteAndBlack;
use crate::entity::game::component::{
    bitboard,
    square::{self, Switch, TypePiece},
};
use std::ops::BitOrAssign;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CheckStatus {
    Simple {
        attacker: square::TypePiece,
        attacker_index: bitboard::BitIndex,
    },
    Double,
    None,
}
impl CheckStatus {
    pub fn is_check(&self) -> bool {
        *self != CheckStatus::None
    }
    pub fn build_simple_check(
        attacker: square::TypePiece,
        attacker_index: bitboard::BitIndex,
    ) -> CheckStatus {
        if attacker_index.0 < 64 {
            CheckStatus::Simple {
                attacker,
                attacker_index,
            }
        } else {
            panic!(
                "Invalid attacker_index for SimpleCheck: found {}. Should be in [0-63]",
                attacker_index.0
            )
        }
    }
}

#[derive(Debug)]
pub struct Attackers {
    rooks: BitBoard,
    knights: BitBoard,
    bishops: BitBoard,
    queens: BitBoard,
    king: BitBoard,
    pawns: BitBoard,
}
pub struct AttackersIterator<'a> {
    attackers: &'a Attackers,
    index: usize,
}
impl<'a> Iterator for AttackersIterator<'a> {
    type Item = (TypePiece, BitBoard);

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.index {
            0 => Some((TypePiece::Rook, self.attackers.rooks)),
            1 => Some((TypePiece::Knight, self.attackers.knights)),
            2 => Some((TypePiece::Bishop, self.attackers.bishops)),
            3 => Some((TypePiece::Queen, self.attackers.queens)),
            4 => Some((TypePiece::King, self.attackers.king)),
            5 => Some((TypePiece::Pawn, self.attackers.pawns)),
            _ => None,
        };
        self.index += 1;
        result
    }
}
impl Attackers {
    pub fn iter(&self) -> AttackersIterator {
        AttackersIterator {
            attackers: self,
            index: 0,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.rooks.empty()
            && self.knights.empty()
            && self.bishops.empty()
            && self.pawns.empty()
            && self.king.empty()
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct RooksBitBoard {
    bitboard: BitBoard,
}
impl RooksBitBoard {
    pub fn new(bitboard: BitBoard) -> Self {
        RooksBitBoard { bitboard }
    }
    pub fn bitboard(&self) -> &BitBoard {
        &self.bitboard
    }
    pub fn xor_mut(&mut self, mask_xor: BitBoard) {
        self.bitboard.xor_mut(mask_xor);
    }
    pub fn switch(&mut self, mask_switch: BitBoard, mask_promotion: BitBoard) {
        self.bitboard.switch(mask_switch, mask_promotion);
    }
    pub fn gen_moves_no_check(
        &self,
        _color: &square::Color,
        bit_board: &bitboard::BitBoards,
        bit_board_opponent: &bitboard::BitBoards,
    ) -> Vec<PieceMoves> {
        let mut moves = vec![];
        for lsb in self.bitboard.iter() {
            if let Some(moves_for_piece) = gen_moves_for_rook(
                false,
                lsb,
                &bit_board.concat_bit_boards(),
                &bit_board_opponent.concat_bit_boards(),
            ) {
                moves.push(moves_for_piece);
            }
        }
        moves
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct BishopsBitBoard {
    bitboard: BitBoard,
}
impl BishopsBitBoard {
    pub fn new(bitboard: BitBoard) -> Self {
        BishopsBitBoard { bitboard }
    }
    pub fn bitboard(&self) -> &BitBoard {
        &self.bitboard
    }
    pub fn xor_mut(&mut self, mask_xor: BitBoard) {
        self.bitboard.xor_mut(mask_xor);
    }
    pub fn switch(&mut self, mask_switch: BitBoard, mask_promotion: BitBoard) {
        self.bitboard.switch(mask_switch, mask_promotion);
    }
    pub fn gen_moves_no_check(
        &self,
        _color: &square::Color,
        bit_board: &bitboard::BitBoards,
        bit_board_opponent: &bitboard::BitBoards,
    ) -> Vec<PieceMoves> {
        let mut moves = vec![];
        for lsb in self.bitboard.iter() {
            if let Some(moves_for_piece) = gen_moves_for_bishop(
                false,
                lsb,
                &bit_board.concat_bit_boards(),
                &bit_board_opponent.concat_bit_boards(),
            ) {
                moves.push(moves_for_piece);
            }
        }
        moves
    }
}
#[derive(Debug, PartialEq, Clone, Default)]
pub struct KnightsBitBoard {
    bitboard: BitBoard,
}
impl KnightsBitBoard {
    pub fn new(bitboard: BitBoard) -> Self {
        KnightsBitBoard { bitboard }
    }
    pub fn bitboard(&self) -> &BitBoard {
        &self.bitboard
    }
    pub fn xor_mut(&mut self, mask_xor: BitBoard) {
        self.bitboard.xor_mut(mask_xor);
    }
    pub fn switch(&mut self, mask_switch: BitBoard, mask_promotion: BitBoard) {
        self.bitboard.switch(mask_switch, mask_promotion);
    }
    pub fn gen_moves_no_check(
        &self,
        _color: &square::Color,
        bit_board: &bitboard::BitBoards,
        _bit_board_opponent: &bitboard::BitBoards,
    ) -> Vec<PieceMoves> {
        let mut moves = vec![];
        for lsb in self.bitboard.iter() {
            if let Some(moves_for_piece) = gen_moves_for_knight(lsb, &bit_board.concat_bit_boards())
            {
                moves.push(moves_for_piece);
            }
        }
        moves
    }
}
#[derive(Debug, PartialEq, Clone, Default)]
pub struct KingBitBoard {
    bitboard: BitBoard,
}
impl KingBitBoard {
    pub fn new(bitboard: BitBoard) -> Self {
        KingBitBoard { bitboard }
    }
    pub fn bitboard(&self) -> &BitBoard {
        &self.bitboard
    }
    pub fn xor_mut(&mut self, mask_xor: BitBoard) {
        self.bitboard.xor_mut(mask_xor);
    }
    pub fn switch(&mut self, mask_switch: BitBoard, mask_promotion: BitBoard) {
        self.bitboard.switch(mask_switch, mask_promotion);
    }
    pub fn gen_moves_no_check(
        &self,
        color: &square::Color,
        bit_board: &bitboard::BitBoards,
        bit_board_opponent: &bitboard::BitBoards,
        can_castle_king_side: Option<(bitboard::BitIndex, bitboard::BitIndex)>,
        can_castle_queen_side: Option<(bitboard::BitIndex, bitboard::BitIndex, bitboard::BitIndex)>,
    ) -> Vec<PieceMoves> {
        let mut moves = vec![];
        for lsb in self.bitboard.iter() {
            let bit_moves = gen_moves_for_king_except_castle(lsb, &bit_board.concat_bit_boards())
                | gen_moves_for_king_castle(
                    color,
                    bit_board,
                    bit_board_opponent,
                    can_castle_king_side,
                    can_castle_queen_side,
                );
            if let Some(moves_for_piece) = moves_non_empty(
                TypePiece::King,
                lsb,
                bit_moves,
                &bit_board.concat_bit_boards(),
            ) {
                moves.push(moves_for_piece);
            }
        }
        moves
    }
}
#[derive(Debug, PartialEq, Clone, Default)]
pub struct QueensBitBoard {
    bitboard: BitBoard,
}
impl QueensBitBoard {
    pub fn new(bitboard: BitBoard) -> Self {
        QueensBitBoard { bitboard }
    }
    pub fn bitboard(&self) -> &BitBoard {
        &self.bitboard
    }
    pub fn xor_mut(&mut self, mask_xor: BitBoard) {
        self.bitboard.xor_mut(mask_xor);
    }
    pub fn switch(&mut self, mask_switch: BitBoard, mask_promotion: BitBoard) {
        self.bitboard.switch(mask_switch, mask_promotion);
    }
    pub fn gen_moves_no_check(
        &self,
        _color: &square::Color,
        bit_board: &bitboard::BitBoards,
        bit_board_opponent: &bitboard::BitBoards,
    ) -> Vec<PieceMoves> {
        let mut moves = vec![];
        for lsb in self.bitboard.iter() {
            if let Some(moves_for_piece) = gen_moves_for_queen(
                lsb,
                &bit_board.concat_bit_boards(),
                &bit_board_opponent.concat_bit_boards(),
            ) {
                moves.push(moves_for_piece);
            }
        }
        moves
    }
}
#[derive(Debug, PartialEq, Clone, Default)]
pub struct PawnsBitBoard {
    bitboard: BitBoard,
}
impl PawnsBitBoard {
    pub fn new(bitboard: BitBoard) -> Self {
        PawnsBitBoard { bitboard }
    }
    pub fn bitboard(&self) -> &BitBoard {
        &self.bitboard
    }
    pub fn xor_mut(&mut self, mask_xor: BitBoard) {
        self.bitboard.xor_mut(mask_xor);
    }
    pub fn switch(&mut self, mask_switch: BitBoard, mask_promotion: BitBoard) {
        self.bitboard.switch(mask_switch, mask_promotion);
    }
    pub fn gen_moves_no_check(
        &self,
        color: &square::Color,
        bit_board: &bitboard::BitBoards,
        bit_board_opponent: &bitboard::BitBoards,
        capture_en_passant: Option<&bitboard::BitIndex>,
    ) -> Vec<PieceMoves> {
        let mut moves = vec![];
        for lsb in self.bitboard.iter() {
            if let Some(moves_for_piece) = gen_moves_for_pawn(
                lsb,
                color,
                &bit_board.concat_bit_boards(),
                &bit_board_opponent.concat_bit_boards(),
                capture_en_passant,
            ) {
                moves.push(moves_for_piece);
            }
        }
        moves
    }
}

impl BitOrAssign<u64> for RooksBitBoard {
    fn bitor_assign(&mut self, rhs: u64) {
        self.bitboard |= BitBoard(rhs);
    }
}
impl BitOrAssign<u64> for BishopsBitBoard {
    fn bitor_assign(&mut self, rhs: u64) {
        self.bitboard |= BitBoard(rhs);
    }
}
impl BitOrAssign<u64> for KnightsBitBoard {
    fn bitor_assign(&mut self, rhs: u64) {
        self.bitboard |= BitBoard(rhs);
    }
}
impl BitOrAssign<u64> for KingBitBoard {
    fn bitor_assign(&mut self, rhs: u64) {
        self.bitboard |= BitBoard(rhs);
    }
}
impl BitOrAssign<u64> for QueensBitBoard {
    fn bitor_assign(&mut self, rhs: u64) {
        self.bitboard |= BitBoard(rhs);
    }
}
impl BitOrAssign<u64> for PawnsBitBoard {
    fn bitor_assign(&mut self, rhs: u64) {
        self.bitboard |= BitBoard(rhs);
    }
}

pub trait GenMoves {
    fn gen_moves_for_all(
        &self,
        color: &square::Color,
        check_status: CheckStatus,
        capture_en_passant: Option<&bitboard::BitIndex>,
        bit_position_status: &bitboard::BitPositionStatus,
    ) -> Vec<bitboard::BitBoardMove>;

    fn check_status(&self, color: &square::Color) -> CheckStatus;
    fn can_move(
        &self,
        color: &square::Color,
        check_status: CheckStatus,
        capture_en_passant: Option<&bitboard::BitIndex>,
        bit_position_status: &bitboard::BitPositionStatus,
    ) -> bool;
}

fn moves2bitboard_moves(
    color: square::Color,
    moves: Vec<PieceMoves>,
    bit_boards_white_and_black: &BitBoardsWhiteAndBlack,
) -> Vec<bitboard::BitBoardMove> {
    let mut bitboard_moves: Vec<bitboard::BitBoardMove> = vec![];
    for piece_moves in &moves {
        for to in piece_moves.moves().iter() {
            let bitboard_move = bitboard::BitBoardMove::from(
                color,
                piece_moves.type_piece(),
                piece_moves.index(),
                to,
                bit_boards_white_and_black,
            );
            // check that destination square for king are free of attackers
            match piece_moves.type_piece() {
                TypePiece::King => {
                    let bitboard_move: Vec<&BitBoardMove> = bitboard_move
                        .iter()
                        .filter(|b_move| {
                            // simulate the move
                            let mut updated_board = bit_boards_white_and_black.clone();
                            updated_board.move_piece(
                                b_move,
                                &mut zobrist::ZobristHash::default(),
                                None,
                            );
                            check_status(&color, &updated_board) == CheckStatus::None
                        })
                        .collect();
                    bitboard_moves.extend(bitboard_move);
                }
                type_piece => {
                    if !control_discover_king_no_check(
                        &color,
                        type_piece,
                        piece_moves.index(),
                        to,
                        bit_boards_white_and_black,
                    ) {
                        bitboard_moves.extend(bitboard_move);
                    }
                }
            }
        }
    }
    bitboard_moves
}

// Check that the moves does not discover King
fn control_discover_king_no_check(
    color: &square::Color,
    type_piece: TypePiece,
    p_start: bitboard::BitIndex,
    p_end: bitboard::BitIndex,
    bit_boards_white_and_black: &BitBoardsWhiteAndBlack,
) -> bool {
    let k = bit_boards_white_and_black
        .bit_board(color)
        .king()
        .bitboard()
        .index();
    // check there is no check if we remove the piece
    match (k.direction(p_start), p_start.direction(p_end)) {
        (bitboard::Direction::None, _) => false,
        (dir1, dir2) if dir1 == dir2 => false,
        (bitboard::Direction::BishopBottomLeftTopRight, _)
        | (bitboard::Direction::BishopTopLeftBottomRight, _) => {
            // remove piece at start and add it at the end (not full move which is more costly)
            let capture = match bit_boards_white_and_black.peek(p_end) {
                square::Square::NonEmpty(piece) => Some(piece.type_piece()),
                _ => None,
            };
            let b_move =
                &bitboard::BitBoardMove::new(*color, type_piece, p_start, p_end, capture, None);
            let mut bb = bit_boards_white_and_black.clone();
            bb.move_piece(b_move, &mut zobrist::ZobristHash::default(), None);
            let bit_board = bb.bit_board(color);
            let bit_board_opponent = bb.bit_board(&color.switch());
            // generate king moves as Bishop
            let moves = BishopsBitBoard::new(k.bitboard()).gen_moves_no_check(
                color,
                bit_board,
                bit_board_opponent,
            );
            // result: intersect with opponent Bishop / Q
            let opponent_bishops_queens =
                *bit_board_opponent.bishops().bitboard() | *bit_board_opponent.queens().bitboard();
            moves
                .iter()
                .any(|m| (m.moves & opponent_bishops_queens).non_empty())
        }
        (bitboard::Direction::RookVertical, _) | (bitboard::Direction::RookHorizontal, _) => {
            // remove piece at start and add it at the end (not full move which is more costly)
            let capture = match bit_boards_white_and_black.peek(p_end) {
                square::Square::NonEmpty(piece) => Some(piece.type_piece()),
                _ => None,
            };
            let b_move =
                &bitboard::BitBoardMove::new(*color, type_piece, p_start, p_end, capture, None);
            let mut bb = bit_boards_white_and_black.clone();
            bb.move_piece(b_move, &mut zobrist::ZobristHash::default(), None);
            let bit_board = bb.bit_board(color);
            let bit_board_opponent = bb.bit_board(&color.switch());
            // generate king moves as Bishop
            let moves = RooksBitBoard::new(k.bitboard()).gen_moves_no_check(
                color,
                bit_board,
                bit_board_opponent,
            );
            // result: intersect with opponent Bishop / Q
            let opponent_rooks_queens =
                *bit_board_opponent.rooks().bitboard() | *bit_board_opponent.queens().bitboard();
            moves
                .iter()
                .any(|m| (m.moves & opponent_rooks_queens).non_empty())
        }
    }
}

// Generate moves for king in case of simple check
fn gen_moves_for_all_simple_check(
    color: &square::Color,
    attacker_index: bitboard::BitIndex,
    bit_board: &bitboard::BitBoards,
    bit_board_opponent: &bitboard::BitBoards,
    capture_en_passant: Option<&bitboard::BitIndex>,
) -> Vec<PieceMoves> {
    assert!(attacker_index.bitboard().non_empty());
    let attackers_opponent_check = attackers(
        attacker_index,
        &color.switch(),
        bit_board_opponent,
        bit_board,
    );
    let king_index = bit_board.king().bitboard().index();
    let moves_king = {
        let moves_bitboard =
            gen_moves_for_king_except_castle(king_index, &bit_board.concat_bit_boards());
        moves_non_empty(
            TypePiece::King,
            king_index,
            moves_bitboard,
            &bit_board.concat_bit_boards(),
        )
    };
    let mut moves: Vec<PieceMoves> = moves_king.into_iter().collect();
    // capture attacker to remove check
    for (type_piece, bit_board_attacker_of_opponent) in attackers_opponent_check.iter() {
        if bit_board_attacker_of_opponent.non_empty() {
            moves.push(PieceMoves {
                type_piece,
                index: bit_board_attacker_of_opponent.index(),
                moves: attacker_index.bitboard(),
            })
        }
    }
    // move a piece to block attack
    match king_index.direction(attacker_index) {
        bitboard::Direction::RookHorizontal | bitboard::Direction::RookVertical => {
            // generate moves for king seen as a rook
            let maybe_king_as_rook_moves = RooksBitBoard::new(*bit_board.king().bitboard())
                .gen_moves_no_check(color, bit_board, bit_board_opponent);
            // generate moves for attacking rook as if the same color as the king
            let maybe_rook_moves = RooksBitBoard::new(attacker_index.bitboard())
                .gen_moves_no_check(color, bit_board, bit_board_opponent);
            // compute the mask: intersection of the moves
            if let (Some(king_as_rook_moves), Some(rook_moves)) =
                (maybe_king_as_rook_moves.first(), maybe_rook_moves.first())
            {
                let mask = *king_as_rook_moves.moves() & *rook_moves.moves();
                // generate moves for R, B, Q, K
                let rook_moves =
                    bit_board
                        .rooks()
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                let bishop_moves =
                    bit_board
                        .bishops()
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                let queen_moves =
                    bit_board
                        .queens()
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                let knight_moves =
                    bit_board
                        .knights()
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                let pawn_moves = bit_board.pawns().gen_moves_no_check(
                    color,
                    bit_board,
                    bit_board_opponent,
                    capture_en_passant,
                );
                // and intersect with mask
                let iter = rook_moves.into_iter().chain(
                    bishop_moves
                        .into_iter()
                        .chain(queen_moves.into_iter().chain(knight_moves))
                        .chain(pawn_moves),
                );
                let moves_blocking_check: Vec<PieceMoves> = iter
                    .filter(|m| (m.moves & mask).non_empty())
                    .map(|m| m.and_moves(mask))
                    .collect();
                moves.extend(moves_blocking_check);
            }
        }
        bitboard::Direction::BishopBottomLeftTopRight
        | bitboard::Direction::BishopTopLeftBottomRight => {
            // generate moves for king seen as a bishop
            let maybe_king_as_bishop_moves = BishopsBitBoard::new(*bit_board.king().bitboard())
                .gen_moves_no_check(color, bit_board, bit_board_opponent);
            // generate moves for attacking rook as if the same color as the king
            let maybe_bishop_moves = BishopsBitBoard::new(attacker_index.bitboard())
                .gen_moves_no_check(color, bit_board, bit_board_opponent);
            // compute the mask: intersection of the moves
            if let (Some(king_as_bishop_moves), Some(bishop_moves)) = (
                maybe_king_as_bishop_moves.first(),
                maybe_bishop_moves.first(),
            ) {
                let mask = *king_as_bishop_moves.moves() & *bishop_moves.moves();
                // generate moves for R, B, Q, K
                let rook_moves =
                    bit_board
                        .rooks()
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                let bishop_moves =
                    bit_board
                        .bishops()
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                let queen_moves =
                    bit_board
                        .queens()
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                let knight_moves =
                    bit_board
                        .knights()
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                let pawn_moves = bit_board.pawns().gen_moves_no_check(
                    color,
                    bit_board,
                    bit_board_opponent,
                    capture_en_passant,
                );
                // and intersect with mask
                let iter = rook_moves.into_iter().chain(
                    bishop_moves
                        .into_iter()
                        .chain(queen_moves.into_iter().chain(knight_moves))
                        .chain(pawn_moves),
                );
                let moves_blocking_check: Vec<PieceMoves> = iter
                    .filter(|m| (m.moves & mask).non_empty())
                    .map(|m| m.and_moves(mask))
                    .collect();
                moves.extend(moves_blocking_check);
            }
        }
        _ => {}
    }
    moves
}

impl GenMoves for bitboard::BitBoardsWhiteAndBlack {
    fn can_move(
        &self,
        color: &square::Color,
        check_status: CheckStatus,
        capture_en_passant: Option<&bitboard::BitIndex>,
        bit_position_status: &bitboard::BitPositionStatus,
    ) -> bool {
        let bit_board = self.bit_board(color);
        let bit_board_opponent = self.bit_board(&color.switch());
        match check_status {
            CheckStatus::None => {
                let can_castle_king_side =
                    bit_position_status.can_castle_king_side(bit_board.concat_bit_boards(), color);
                let can_castle_queen_side =
                    bit_position_status.can_castle_queen_side(bit_board.concat_bit_boards(), color);
                let moves_piece =
                    bit_board
                        .rooks
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                let mut moves = moves2bitboard_moves(*color, moves_piece, self);
                if moves.is_empty() {
                    let moves_piece =
                        bit_board
                            .bishops
                            .gen_moves_no_check(color, bit_board, bit_board_opponent);
                    moves = moves2bitboard_moves(*color, moves_piece, self);
                }
                if moves.is_empty() {
                    let moves_piece =
                        bit_board
                            .knights
                            .gen_moves_no_check(color, bit_board, bit_board_opponent);
                    moves = moves2bitboard_moves(*color, moves_piece, self);
                }
                if moves.is_empty() {
                    let moves_piece = bit_board.king.gen_moves_no_check(
                        color,
                        bit_board,
                        bit_board_opponent,
                        can_castle_king_side,
                        can_castle_queen_side,
                    );
                    moves = moves2bitboard_moves(*color, moves_piece, self);
                }
                if moves.is_empty() {
                    let moves_piece =
                        bit_board
                            .queens
                            .gen_moves_no_check(color, bit_board, bit_board_opponent);
                    moves = moves2bitboard_moves(*color, moves_piece, self);
                }
                if moves.is_empty() {
                    let moves_piece = bit_board.pawns.gen_moves_no_check(
                        color,
                        bit_board,
                        bit_board_opponent,
                        capture_en_passant,
                    );
                    moves = moves2bitboard_moves(*color, moves_piece, self);
                }
                !moves.is_empty()
            }
            CheckStatus::Simple {
                attacker: _,
                attacker_index,
            } => {
                let moves = gen_moves_for_all_simple_check(
                    color,
                    attacker_index,
                    bit_board,
                    bit_board_opponent,
                    capture_en_passant,
                );
                !moves2bitboard_moves(*color, moves, self).is_empty()
            }
            CheckStatus::Double => {
                let index = bit_board.king().bitboard.index();
                let moves = gen_moves_for_king_except_castle(index, &bit_board.concat_bit_boards());
                let moves = if moves.empty() {
                    vec![]
                } else {
                    vec![PieceMoves {
                        type_piece: TypePiece::King,
                        index,
                        moves,
                    }]
                };
                !moves2bitboard_moves(*color, moves, self).is_empty()
            }
        }
    }
    fn gen_moves_for_all(
        &self,
        color: &square::Color,
        check_status: CheckStatus,
        capture_en_passant: Option<&bitboard::BitIndex>,
        bit_position_status: &bitboard::BitPositionStatus,
    ) -> Vec<bitboard::BitBoardMove> {
        let bit_board = self.bit_board(color);
        let bit_board_opponent = self.bit_board(&color.switch());
        let moves_all = match check_status {
            CheckStatus::None => {
                let can_castle_king_side =
                    bit_position_status.can_castle_king_side(bit_board.concat_bit_boards(), color);
                let can_castle_queen_side =
                    bit_position_status.can_castle_queen_side(bit_board.concat_bit_boards(), color);
                let mut moves_all: Vec<PieceMoves> = vec![];
                let moves =
                    bit_board
                        .rooks
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                moves_all.extend(moves);
                let moves =
                    bit_board
                        .bishops
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                moves_all.extend(moves);
                let moves =
                    bit_board
                        .knights
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                moves_all.extend(moves);
                let moves = bit_board.king.gen_moves_no_check(
                    color,
                    bit_board,
                    bit_board_opponent,
                    can_castle_king_side,
                    can_castle_queen_side,
                );
                moves_all.extend(moves);
                let moves =
                    bit_board
                        .queens
                        .gen_moves_no_check(color, bit_board, bit_board_opponent);
                moves_all.extend(moves);
                let moves = bit_board.pawns.gen_moves_no_check(
                    color,
                    bit_board,
                    bit_board_opponent,
                    capture_en_passant,
                );
                moves_all.extend(moves);
                moves_all
            }
            CheckStatus::Simple {
                attacker: _,
                attacker_index,
            } => gen_moves_for_all_simple_check(
                color,
                attacker_index,
                bit_board,
                bit_board_opponent,
                capture_en_passant,
            ),
            CheckStatus::Double => {
                let index = bit_board.king().bitboard.index();
                let moves = gen_moves_for_king_except_castle(index, &bit_board.concat_bit_boards());
                if moves.empty() {
                    vec![]
                } else {
                    vec![PieceMoves {
                        type_piece: TypePiece::King,
                        index,
                        moves,
                    }]
                }
            }
        };
        moves2bitboard_moves(*color, moves_all, self)
    }

    /// check if the current king of color 'color' is under check
    fn check_status(&self, color: &square::Color) -> CheckStatus {
        check_status(color, self)
    }
}

fn check_status(
    color: &square::Color,
    bit_boards_white_and_black: &BitBoardsWhiteAndBlack,
) -> CheckStatus {
    let (bit_board, bit_board_opponent) = match *color {
        square::Color::White => (
            bit_boards_white_and_black.bit_board_white(),
            bit_boards_white_and_black.bit_board_black(),
        ),
        square::Color::Black => (
            bit_boards_white_and_black.bit_board_black(),
            bit_boards_white_and_black.bit_board_white(),
        ),
    };

    let king_index = bit_board.king().bitboard.index();
    let attackers = attackers(king_index, color, bit_board, bit_board_opponent);
    match (
        attackers.rooks.count_ones(),
        attackers.bishops.count_ones(),
        attackers.queens.count_ones(),
        attackers.knights.count_ones(),
        attackers.pawns.count_ones(),
        attackers.king.count_ones(),
    ) {
        (0, 0, 0, 0, 0, 0) => CheckStatus::None,
        (1, 0, 0, 0, 0, 0) => {
            CheckStatus::build_simple_check(TypePiece::Rook, attackers.rooks.index())
        }
        (0, 1, 0, 0, 0, 0) => {
            CheckStatus::build_simple_check(TypePiece::Bishop, attackers.bishops.index())
        }
        (0, 0, 1, 0, 0, 0) => {
            CheckStatus::build_simple_check(TypePiece::Queen, attackers.queens.index())
        }
        (0, 0, 0, 1, 0, 0) => {
            CheckStatus::build_simple_check(TypePiece::Knight, attackers.knights.index())
        }
        (0, 0, 0, 0, 1, 0) => {
            CheckStatus::build_simple_check(TypePiece::Pawn, attackers.pawns.index())
        }
        (0, 0, 0, 0, 0, 1) => {
            CheckStatus::build_simple_check(TypePiece::King, attackers.king.index())
        }
        _ => CheckStatus::Double,
    }
}

/// Identify attackers for piece_index for color
fn attackers(
    piece_index: bitboard::BitIndex,
    color: &square::Color,
    bit_board: &bitboard::BitBoards,
    bit_board_opponent: &bitboard::BitBoards,
) -> Attackers {
    let bb_zero = BitBoard::default();
    let piece_bit_board = piece_index.bitboard();
    // Generate piece moves as if it were a rook, bishop, knight, pawn
    let piece_as_rook = RooksBitBoard::new(piece_bit_board).gen_moves_no_check(
        color,
        bit_board,
        bit_board_opponent,
    );
    let piece_as_rook = piece_as_rook.first().map(|m| m.moves()).unwrap_or(&bb_zero);
    let piece_as_bishop = BishopsBitBoard::new(piece_bit_board).gen_moves_no_check(
        color,
        bit_board,
        bit_board_opponent,
    );
    let piece_as_bishop = piece_as_bishop
        .first()
        .map(|m| m.moves())
        .unwrap_or(&bb_zero);
    let piece_as_knight = KnightsBitBoard::new(piece_bit_board).gen_moves_no_check(
        color,
        bit_board,
        bit_board_opponent,
    );
    let piece_as_knight = piece_as_knight
        .first()
        .map(|m| m.moves())
        .unwrap_or(&bb_zero);
    let piece_as_pawn =
        gen_pawn_squares_attacked(piece_index, color, &bit_board_opponent.concat_bit_boards());
    // Intersect piece moves with rooks, bishops, knights, pawns bitboards of the opposite color
    let rook_attackers: BitBoard = *piece_as_rook & *bit_board_opponent.rooks().bitboard();
    let bishop_attackers: BitBoard = *piece_as_bishop & *bit_board_opponent.bishops().bitboard();
    let queen_attackers: BitBoard =
        (*piece_as_rook | *piece_as_bishop) & *bit_board_opponent.queens().bitboard();
    let knight_attackers: BitBoard = *piece_as_knight & *bit_board_opponent.knights().bitboard();
    let pawn_attackers: BitBoard = piece_as_pawn & *bit_board_opponent.pawns().bitboard();
    // generate moves for king to capture piece_index except when computing check status
    let king_attackers =
        gen_moves_for_king_except_castle(piece_index, &bit_board.concat_bit_boards())
            & *bit_board_opponent.king().bitboard();
    Attackers {
        rooks: rook_attackers,
        bishops: bishop_attackers,
        queens: queen_attackers,
        knights: knight_attackers,
        pawns: pawn_attackers,
        king: king_attackers,
    }
}

fn moves_non_empty(
    type_piece: TypePiece,
    index: bitboard::BitIndex,
    moves_bitboard: BitBoard,
    bit_board: &bitboard::BitBoard,
) -> Option<PieceMoves> {
    let moves_bitboard = moves_bitboard & !*bit_board;
    PieceMoves::new(type_piece, index, moves_bitboard)
}

fn gen_moves_for_king_castle(
    color: &square::Color,
    bit_board: &bitboard::BitBoards,
    bit_board_opponent: &bitboard::BitBoards,
    can_castle_king_side: Option<(bitboard::BitIndex, bitboard::BitIndex)>,
    can_castle_queen_side: Option<(bitboard::BitIndex, bitboard::BitIndex, bitboard::BitIndex)>,
) -> BitBoard {
    let occupied_squares = bit_board.concat_bit_boards() | bit_board_opponent.concat_bit_boards();
    let mut move_short_castle = BitBoard::default();
    if let Some((sq1_idx, sq2_idx)) = can_castle_king_side {
        let are_squares_empty =
            (occupied_squares & (sq1_idx.bitboard() | sq2_idx.bitboard())).empty();
        if are_squares_empty
            && attackers(sq1_idx, color, bit_board, bit_board_opponent).is_empty()
            && attackers(sq2_idx, color, bit_board, bit_board_opponent).is_empty()
        {
            move_short_castle = sq2_idx.bitboard();
        };
    };
    let mut move_long_castle = BitBoard::default();
    if let Some((sq1_idx, sq2_idx, sq3_idx)) = can_castle_queen_side {
        let are_squares_empty = (occupied_squares
            & (sq1_idx.bitboard() | sq2_idx.bitboard() | sq3_idx.bitboard()))
        .empty();
        if are_squares_empty
            && attackers(sq1_idx, color, bit_board, bit_board_opponent).is_empty()
            && attackers(sq2_idx, color, bit_board, bit_board_opponent).is_empty()
            && attackers(sq3_idx, color, bit_board, bit_board_opponent).is_empty()
        {
            move_long_castle = sq2_idx.bitboard();
        };
    };
    move_short_castle | move_long_castle
}
fn gen_moves_for_king_except_castle(
    index: bitboard::BitIndex,
    bit_board: &bitboard::BitBoard,
) -> BitBoard {
    let moves_bitboard = table::table_king::king_moves(index.value());
    BitBoard(moves_bitboard & !bit_board.value())
}

fn gen_moves_for_knight(
    index: bitboard::BitIndex,
    bit_board: &bitboard::BitBoard,
) -> Option<PieceMoves> {
    let moves_bitboard = table::table_knight::knight_moves(index.value());
    moves_non_empty(
        TypePiece::Knight,
        index,
        BitBoard(moves_bitboard),
        bit_board,
    )
}

fn gen_moves_for_rook_horizontal(index: bitboard::BitIndex, blockers_h: BitBoard) -> BitBoard {
    let mask_h = BitBoard::new(255) << index.first_col();
    let blockers_h = blockers_h & mask_h;
    let col = index.col();
    let index_col_a = index.first_col();
    let blockers_first_row = (blockers_h >> index_col_a).0 as u8;
    BitBoard(table_rook::table_rook_h(col, blockers_first_row) as u64) << index_col_a
}

fn gen_moves_for_rook_vertical(index: bitboard::BitIndex, blockers_v: BitBoard) -> BitBoard {
    let mask_v = BitBoard(table::MASK_COL_A) << index.col();
    BitBoard::new(table::table_rook::table_rook_v(
        index.value(),
        blockers_v.value(),
        mask_v.value(),
    ))
}

fn gen_moves_for_rook(
    is_queen: bool,
    index: bitboard::BitIndex,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,
) -> Option<PieceMoves> {
    let blockers = *bit_board | *bit_board_opponent;
    let moves_horizontal = gen_moves_for_rook_horizontal(index, blockers);
    let moves_vertical = gen_moves_for_rook_vertical(index, blockers);
    let moves_bitboard = moves_horizontal | moves_vertical;
    let type_piece = if is_queen {
        TypePiece::Queen
    } else {
        TypePiece::Rook
    };
    moves_non_empty(type_piece, index, moves_bitboard, bit_board)
}

fn gen_moves_for_bishop(
    is_queen: bool,
    index: bitboard::BitIndex,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,
) -> Option<PieceMoves> {
    let blockers = *bit_board | *bit_board_opponent;
    let moves_bitboard = BitBoard(table_bishop::bishop_moves(index.value(), blockers.value()));
    let type_piece = if is_queen {
        TypePiece::Queen
    } else {
        TypePiece::Bishop
    };
    moves_non_empty(type_piece, index, moves_bitboard, bit_board)
}

fn gen_moves_for_queen(
    index: bitboard::BitIndex,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,
) -> Option<PieceMoves> {
    let rook_moves = gen_moves_for_rook(true, index, bit_board, bit_board_opponent);
    let bishop_moves = gen_moves_for_bishop(true, index, bit_board, bit_board_opponent);
    match (rook_moves, bishop_moves) {
        (None, None) => None,
        (left, None) => left,
        (None, right) => right,
        (Some(left), Some(right)) => Some(PieceMoves {
            type_piece: TypePiece::Queen,
            index,
            moves: bitboard::BitBoard(left.moves().value() | right.moves().value()),
        }),
    }
}

fn gen_pawn_squares_attacked(
    index: bitboard::BitIndex,
    color: &square::Color,
    bit_board_opponent: &bitboard::BitBoard,
) -> BitBoard {
    let col = index.col();
    let row = index.row();
    let mut moves = BitBoard::default();
    match color {
        square::Color::White => {
            // capture up left
            if col > 0 && row < 7 {
                let to = index.up().left().bitboard();
                if (to & *bit_board_opponent).non_empty() {
                    moves |= to;
                }
            }
            // capture up right
            if col < 7 && row < 7 {
                let to = index.up().right().bitboard();
                if (to & *bit_board_opponent).non_empty() {
                    moves |= to;
                }
            }
        }
        square::Color::Black => {
            if col > 0 && row > 0 {
                // catpure left down
                let to = index.down().left().bitboard();
                if (to & *bit_board_opponent).non_empty() {
                    moves |= to;
                }
            }
            // catpure right down
            if col < 7 && row > 0 {
                let to = index.down().right().bitboard();
                if (to & *bit_board_opponent).non_empty() {
                    moves |= to;
                }
            }
        }
    }
    moves
}
fn gen_pawn_non_attacker_moves(
    index: bitboard::BitIndex,
    color: &square::Color,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,
    capture_en_passant: Option<&bitboard::BitIndex>,
) -> BitBoard {
    let row = index.row();
    let blockers = *bit_board | *bit_board_opponent;
    let mut moves = BitBoard::default();
    match color {
        square::Color::White => {
            // up x 1
            let to = index.up().bitboard();
            if (to & blockers).empty() {
                moves |= to;
                // up x 2
                if row == 1 {
                    let to = index.upx2().bitboard();
                    if (to & blockers).empty() {
                        moves |= to;
                    }
                }
            }
            // capture en passant
            if let Some(en_passant_idx) = capture_en_passant {
                if index.col() > 0 && index.up().left() == *en_passant_idx
                    || index.col() < 7 && index.up().right() == *en_passant_idx
                {
                    moves |= en_passant_idx.bitboard();
                }
            }
        }
        square::Color::Black => {
            // down x 1
            let to = index.down().bitboard();
            if (to & blockers).empty() {
                moves |= to;
                // down x 2
                if row == 6 {
                    let to = index.downx2().bitboard();
                    if (to & blockers).empty() {
                        moves |= to;
                    }
                }
            }
            // capture en passant
            if let Some(en_passant_idx) = capture_en_passant {
                if index.col() > 7 && index.down().right() == *en_passant_idx
                    || index.col() > 0 && index.down().left() == *en_passant_idx
                {
                    moves |= en_passant_idx.bitboard();
                }
            }
        }
    }
    moves
}

fn gen_moves_for_pawn(
    index: bitboard::BitIndex,
    color: &square::Color,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,
    capture_en_passant: Option<&bitboard::BitIndex>,
) -> Option<PieceMoves> {
    let to = gen_pawn_non_attacker_moves(
        index,
        color,
        bit_board,
        bit_board_opponent,
        capture_en_passant,
    ) | gen_pawn_squares_attacked(index, color, bit_board_opponent);
    PieceMoves::new(TypePiece::Pawn, index, to)
}

#[derive(Debug)]
pub struct PieceMoves {
    type_piece: TypePiece,
    /// where is the piece
    index: bitboard::BitIndex,
    /// BitBoard representing all possible moves    
    moves: bitboard::BitBoard,
}
impl PieceMoves {
    pub fn new(type_piece: TypePiece, index: bitboard::BitIndex, moves: BitBoard) -> Option<Self> {
        if moves.empty() {
            None
        } else {
            Some(PieceMoves {
                type_piece,
                index,
                moves,
            })
        }
    }
    pub fn type_piece(&self) -> TypePiece {
        self.type_piece
    }
    pub fn index(&self) -> bitboard::BitIndex {
        self.index
    }
    pub fn moves(&self) -> &bitboard::BitBoard {
        &self.moves
    }
    pub fn and_moves(&self, mask: BitBoard) -> Self {
        PieceMoves {
            moves: self.moves & mask,
            ..*self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::notation::long_notation;
    use bitboard::BitBoard;

    impl fen::Position {
        /// check status from a position
        pub fn check_status(&self) -> CheckStatus {
            let bit_position = bitboard::BitPosition::from(self.clone());
            bit_position
                .bit_boards_white_and_black()
                .check_status(&self.status().player_turn())
        }
    }

    fn list_index(bit_board: &bitboard::BitBoard) -> Vec<u8> {
        let mut v = Vec::new();
        let mut bb = bit_board.0;
        while bb != 0 {
            let lsb = bitboard::pos2index(bb);
            v.push(lsb);
            bb &= bb - 1; // Remove lsb
        }
        v
    }

    ////////////////////////////////////////////////////////
    /// Bit operations tests
    ////////////////////////////////////////////////////////    ///
    #[test]
    #[ignore]
    fn poc() {
        println!("poc");
        let matrix = 63 << 1;
        let column = 2;
        let mask: u64 = 0x0101010101010101u64 << column;
        let column_bits = matrix & mask;
        let shifted_column_bits = column_bits >> 1;
        println!("{}", bitboard::BitBoard(mask));
        println!("{}", bitboard::BitBoard(shifted_column_bits));
    }

    #[test]
    #[ignore]
    fn test_single_bit() {
        assert_eq!(bitboard::BitBoard(1u64 << 5).index().0, 5);
        assert_eq!((bitboard::BitBoard(1u64 << 0)).index().0, 0);
        assert_eq!((bitboard::BitBoard(1u64 << 15)).index().0, 15);
    }

    #[test]
    #[ignore]
    fn test_zero_value() {
        assert_eq!(bitboard::BitBoard(0u64).index().value(), 64);
    }

    #[test]
    #[ignore]
    fn test_multiple_bits() {
        let value = bitboard::BitBoard::build(bitboard::BitIndex::union(vec![5, 3]));
        assert_eq!(value.index().value(), 3);
    }

    #[test]
    #[ignore]
    fn test_highest_bit() {
        assert_eq!((bitboard::BitBoard(1u64 << 63)).index().value(), 63);
    }

    #[test]
    #[ignore]
    fn test_empty_bitboard() {
        let bitboard = BitBoard(0);
        assert_eq!(list_index(&bitboard), vec![]);
    }

    #[test]
    #[ignore]
    fn test_list_index_single_bit() {
        let bitboard = BitBoard(1u64 << 5); // bit at position 5
        assert_eq!(list_index(&bitboard), vec![5]);
    }

    #[test]
    #[ignore]
    fn test_list_index_multiple_bits() {
        let bitboard = BitBoard((1u64 << 5) | (1u64 << 15) | (1u64 << 30)); // bits at positions 5, 15, 30
        let mut result = list_index(&bitboard);
        result.sort(); // Sorting the result to ensure order for comparison
        assert_eq!(result, vec![5, 15, 30]);
    }

    #[test]
    #[ignore]
    fn test_list_index_bits_at_edges() {
        let bitboard = BitBoard((1u64 << 0) | (1u64 << 63)); // bits at positions 0 and 63
        let mut result = list_index(&bitboard);
        result.sort(); // Sorting to ensure consistent order
        assert_eq!(result, vec![0, 63]);
    }

    ////////////////////////////////////////////////////////
    /// knight moves
    ////////////////////////////////////////////////////////    ///
    #[test]
    fn test_king_center_moves() {
        let king_position = bitboard::BitIndex(27); // Somewhere in the center of the board
        let bit_board = BitBoard::default(); // No friendly pieces blocking
        let result = gen_moves_for_king_except_castle(king_position, &bit_board);
        assert_eq!(result.0, 0x1C141C0000); // Expected moves bitboard for center position
    }

    #[test]
    fn test_king_edge_moves() {
        let king_position = bitboard::BitIndex(8); // On the edge (A file)
        let bit_board = BitBoard::default();
        let result = gen_moves_for_king_except_castle(king_position, &bit_board);
        let expected_moves = BitBoard::build(bitboard::BitIndex::union(vec![0, 1, 9, 16, 17]));
        assert_eq!(result, expected_moves); // Expected moves bitboard for an edge position
    }

    #[test]
    fn test_king_corner_moves() {
        let king_position = bitboard::BitIndex(0); // Top left corner (A1)
        let bit_board = BitBoard::default();
        let result = gen_moves_for_king_except_castle(king_position, &bit_board);
        let expected_moves = BitBoard::build(bitboard::BitIndex::union(vec![1, 8, 9]));
        assert_eq!(result, expected_moves); // Expected moves bitboard for corner position
    }

    #[test]
    fn test_king_blocked_by_friendly_pieces() {
        let king_position = bitboard::BitIndex(27); // Center of the board
        let bit_board = BitBoard::build(bitboard::BitIndex::union(vec![
            18, 19, 20, 26, 28, 34, 35, 36,
        ]));
        let result = gen_moves_for_king_except_castle(king_position, &bit_board);
        assert!(result.empty()); // Expect no moves available
    }

    #[test]
    #[should_panic]
    fn test_invalid_king_position() {
        let king_position = bitboard::BitIndex(64); // Invalid position
        let bit_board = BitBoard::default();
        let _ = gen_moves_for_king_except_castle(king_position, &bit_board);
    }
    #[test]
    fn test_king_corner_h1_moves() {
        let king_position = bitboard::BitIndex(7); // Top right corner (H1)
        let bit_board = BitBoard::default();
        let result = gen_moves_for_king_except_castle(king_position, &bit_board);
        let expected_moves = BitBoard::build(bitboard::BitIndex::union(vec![6, 14, 15])); // Moves: G1, H2, G2
        assert_eq!(result, expected_moves);
    }

    // Test for the bottom-left corner (A8)
    #[test]
    fn test_king_corner_a8_moves() {
        let king_position = bitboard::BitIndex(56); // Bottom left corner (A8)
        let bit_board = BitBoard::default();
        let result = gen_moves_for_king_except_castle(king_position, &bit_board);
        let expected_moves = BitBoard::build(bitboard::BitIndex::union(vec![48, 49, 57])); // Moves: A7, B7, B8
        assert_eq!(result, expected_moves);
    }

    // Test for the bottom-right corner (H8)
    #[test]
    fn test_king_corner_h8_moves() {
        let king_position = bitboard::BitIndex(63); // Bottom right corner (H8)
        let bit_board = BitBoard::default();
        let result = gen_moves_for_king_except_castle(king_position, &bit_board);
        let expected_moves = BitBoard::build(bitboard::BitIndex::union(vec![62, 54, 55])); // Moves: G8, H7, G7
        assert_eq!(result, expected_moves);
    }

    // Test for an arbitrary position in row 1 (B1)
    #[test]
    fn test_king_row1_b1_moves() {
        let king_position = bitboard::BitIndex(1); // B1
        let bit_board = BitBoard::default();
        let result = gen_moves_for_king_except_castle(king_position, &bit_board);
        let expected_moves = BitBoard::build(bitboard::BitIndex::union(vec![0, 2, 8, 9, 10])); // Moves: A1, C1, A2, B2, C2
        assert_eq!(result, expected_moves);
    }

    // Test for an arbitrary position in row 8 (G8)
    #[test]
    fn test_king_row8_g8_moves() {
        let king_position = bitboard::BitIndex(62); // G8
        let bit_board = BitBoard::default();
        let result = gen_moves_for_king_except_castle(king_position, &bit_board);
        let expected_moves =
            bitboard::BitBoard::build(bitboard::BitIndex::union(vec![61, 63, 53, 54, 55])); // Moves: F8, H8, F7, G7, H7
        assert_eq!(result, expected_moves);
    }

    ////////////////////////////////////////////////////////
    /// knight moves
    ////////////////////////////////////////////////////////    ///
    #[test]
    fn knight_center_moves() {
        let knight_index = bitboard::BitIndex(27); // Position at center of the board (d4)
        let empty_board = BitBoard::default();

        let moves = gen_moves_for_knight(knight_index, &empty_board).unwrap();
        // Moves from d4 are to e2, f3, f5, e6, c6, b5, b3, c2 (calculating their respective bit positions)
        let expected_moves = bitboard::BitBoard::build(bitboard::BitIndex::union(vec![
            10, 12, 17, 21, 33, 37, 42, 44,
        ]));
        assert_eq!(moves.moves().0, expected_moves.0);
    }

    #[test]
    fn knight_corner_moves() {
        let knight_index = bitboard::BitIndex(0); // Position at a1
        let empty_board = BitBoard::default();

        let moves = gen_moves_for_knight(knight_index, &empty_board).unwrap();
        // Moves from a1 are to b3 and c2
        let expected_moves = bitboard::BitBoard::build(bitboard::BitIndex::union(vec![10, 17]));
        assert_eq!(moves.moves().0, expected_moves.0); // Moves from a1 should be limited to b3 and c2
    }

    #[test]
    fn knight_edge_moves() {
        let knight_index = bitboard::BitIndex(8); // Position at a2
        let empty_board = BitBoard::default();

        let moves = gen_moves_for_knight(knight_index, &empty_board).unwrap();
        // Moves from a2 are to b4, c3, and c1
        let expected_moves = BitBoard::build(bitboard::BitIndex::union(vec![2, 18, 25]));
        assert_eq!(moves.moves().0, expected_moves.0); // Valid moves from a2
    }

    #[test]
    fn knight_moves_with_blockages() {
        let knight_index = bitboard::BitIndex(27); // d4 again for center moves
                                                   // Block e6 and c2 with own pieces
        let own_pieces = BitBoard::build(bitboard::BitIndex::union(vec![17, 44])); // Block e6 and b3

        let moves = gen_moves_for_knight(knight_index, &own_pieces).unwrap();
        // Adjusted for blockages, valid moves are to e2, f3, f5, c6, b5, b3, c2
        let expected_moves =
            bitboard::BitBoard::build(bitboard::BitIndex::union(vec![10, 12, 21, 33, 37, 42]));
        assert_eq!(moves.moves().value(), expected_moves.0);
    }

    #[test]
    fn knight_capture_moves() {
        let knight_index = bitboard::BitIndex(27); // d4
        let empty_board = BitBoard::new(0);
        // Block e6 and c2 with own pieces

        let moves = gen_moves_for_knight(knight_index, &empty_board).unwrap();
        // Includes potential captures, valid moves are e2, f3, f5, e6, c6, b5, b3, c2
        let expected_moves = bitboard::BitBoard::build(bitboard::BitIndex::union(vec![
            10, 12, 17, 21, 33, 37, 42, 44,
        ]));
        assert_eq!(moves.moves().0, expected_moves.0); // Includes potential captures
    }

    ////////////////////////////////////////////////////////
    /// Rook moves
    ////////////////////////////////////////////////////////    ///
    #[test]
    fn test_rook_no_blockers() {
        let index = bitboard::BitIndex(8); // Position of the rook at the start of the second row
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitBoard::default();
        let expected = 254 << 8
            | (1 | 1u64 << 16 | 1u64 << 24 | 1u64 << 32 | 1u64 << 40 | 1u64 << 48 | 1u64 << 56);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_to_left() {
        let index = bitboard::BitIndex(17); // Rook on the third row, second column
        let bit_board = index.bitboard() | bitboard::BitIndex(16).bitboard();
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 252 << 16 | (2 | 2 << 8 | 2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_opponent_to_left() {
        let index = bitboard::BitIndex(17); // Rook on the third row, second column
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitBoard(1u64 << 16);
        let expected = 253 << 16 | (2 | 2 << 8 | 2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_to_right() {
        let index = bitboard::BitIndex(17); // Rook on the third row, second column
        let bit_board = index.bitboard() | bitboard::BitIndex(23).bitboard();
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 125 << 16 | (2 | 2 << 8 | 2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        println!("{}", bitboard::BitBoard(result));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_to_right2() {
        let index = bitboard::BitIndex(0);
        let bit_board =
            index.bitboard() | bitboard::BitBoard::build(bitboard::BitIndex::union(vec![4, 8]));
        let bit_board_opponent = BitBoard::default();
        let expected = bitboard::BitBoard::build(bitboard::BitIndex::union(vec![1, 2, 3]));
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected.0);
    }

    #[test]
    fn test_rook_blockers_opponent_to_right() {
        let index = bitboard::BitIndex(17); // Rook on the third row, second column
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitIndex(23).bitboard();
        let expected = 253 << 16 | (2 | 2 << 8 | 2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_on_both_sides() {
        let index = bitboard::BitIndex(18); // Rook on the third row, third column
        let bit_board =
            index.bitboard() | bitboard::BitBoard::build(bitboard::BitIndex::union(vec![16, 23]));
        let bit_board_opponent = bitboard::BitBoard::default();
        let expected = 122 << 16 | (4 | 4 << 8 | 4 << 24 | 4 << 32 | 4 << 40 | 4 << 48 | 4 << 56);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_opponent_on_both_sides() {
        let index = bitboard::BitIndex(18); // Rook on the third row, third column
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitBoard::build(bitboard::BitIndex::union(vec![16, 23]));
        let expected =
            251u64 << 16 | (4 | 4 << 8 | 4 << 24 | 4 << 32 | 4 << 40 | 4 << 48 | 4 << 56);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_on_first_column() {
        let index = bitboard::BitIndex(24); // Rook at the start of the fourth row
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitBoard::default();
        let expected = BitBoard(254 << 24)
            | BitBoard::build(bitboard::BitIndex::union(vec![0, 8, 16, 32, 40, 48, 56]));
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected.0);
    }

    #[test]
    fn test_rook_on_last_column() {
        let index = bitboard::BitIndex(31); // Rook at the end of the fourth row
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitBoard::default();
        let expected = 127 << 24
            | (128 | 128 << 8 | 128 << 16 | 128 << 32 | 128 << 40 | 128 << 48 | 128 << 56);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_full_row_of_blockers() {
        let index = bitboard::BitIndex(40); // Rook somewhere in the middle of the fifth row
        let bit_board = bitboard::BitBoard(255 << 40 | 1u64 << 32 | 1u64 << 48);
        let bit_board_opponent = bitboard::BitBoard::default();
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent);
        assert!(result.is_none());
    }

    #[test]
    fn test_rook_blockers_to_up() {
        let index = bitboard::BitIndex(17); // Rook on the third row, second column
        let bit_board = index.bitboard() | bitboard::BitIndex(25).bitboard();
        let bit_board_opponent = bitboard::BitBoard::default();
        let expected = 253 << 16 | (2 | 2 << 8);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_to_down() {
        let index = bitboard::BitIndex(17); // Rook on the third row, second column
        let bit_board = index.bitboard() | bitboard::BitIndex(9).bitboard();
        let bit_board_opponent = bitboard::BitBoard::default();
        let expected = 253 << 16 | (2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_opponent_to_up() {
        let index = bitboard::BitIndex(17); // Rook on the third row, second column
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitIndex(25).bitboard();
        let expected = 253 << 16 | (2 | 2 << 8 | 2 << 24);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_opponent_to_down() {
        let index = bitboard::BitIndex(17); // Rook on the third row, second column
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitIndex(9).bitboard();
        let expected = 253 << 16 | (2 << 8 | 2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    ////////////////////////////////////////////////////////
    /// Bishop moves
    ////////////////////////////////////////////////////////    ///
    #[test]
    fn test_bishop_blockers() {
        let index = bitboard::BitIndex(20);
        let bit_board = index.bitboard() | bitboard::BitIndex(34).bitboard();
        let bit_board_opponent = bitboard::BitBoard(1u64 << 6);
        let expected = bitboard::BitBoard::build(bitboard::BitIndex::union(vec![
            2, 6, 11, 13, 27, 29, 38, 47,
        ]));
        let result = gen_moves_for_bishop(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected.0);
    }
    #[test]
    fn test_queen_no_blockers() {
        let index = bitboard::BitIndex(8); // Position of the rook at the start of the second row
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitBoard::default();
        let result_rook = gen_moves_for_rook(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        let result_bishop = gen_moves_for_bishop(false, index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        let expected = result_rook | result_bishop;
        let result = gen_moves_for_queen(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    ////////////////////////////////////////////////////////
    /// Pawn moves
    ////////////////////////////////////////////////////////    ///
    #[test]
    fn test_pawn_white_no_blockers() {
        let index = bitboard::BitIndex(20);
        let capture_en_passant: Option<&bitboard::BitIndex> = None;
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitBoard::default();
        let expected = 1u64 << 28;
        let result = gen_moves_for_pawn(
            index,
            &square::Color::White,
            &bit_board,
            &bit_board_opponent,
            capture_en_passant,
        )
        .unwrap()
        .moves()
        .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_pawn_white_row1_no_blockers() {
        let index = bitboard::BitIndex(10);
        let capture_en_passant: Option<&bitboard::BitIndex> = None;
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 1u64 << 18 | 1u64 << 26;
        let result = gen_moves_for_pawn(
            index,
            &square::Color::White,
            &bit_board,
            &bit_board_opponent,
            capture_en_passant,
        )
        .unwrap()
        .moves()
        .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_pawn_white_capture() {
        let index = bitboard::BitIndex(18);
        let capture_en_passant: Option<&bitboard::BitIndex> = None;
        let bit_board = index.bitboard() | index.up().bitboard();
        let bit_board_opponent = index.up().left().bitboard() | index.up().right().bitboard();
        let expected = 1u64 << 25 | 1u64 << 27;
        let result = gen_moves_for_pawn(
            index,
            &square::Color::White,
            &bit_board,
            &bit_board_opponent,
            capture_en_passant,
        )
        .unwrap()
        .moves()
        .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_pawn_white_row1_col_a() {
        let index = bitboard::BitIndex(8);
        let capture_en_passant: Option<&bitboard::BitIndex> = None;
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitIndex(17).bitboard();
        let expected = 1u64 << 16 | 1u64 << 17 | 1u64 << 24;
        let result = gen_moves_for_pawn(
            index,
            &square::Color::White,
            &bit_board,
            &bit_board_opponent,
            capture_en_passant,
        )
        .unwrap()
        .moves()
        .0;
        assert_eq!(result, expected);
    }
    #[test]
    fn test_pawn_black_capture() {
        let index = bitboard::BitIndex(50);
        let capture_en_passant: Option<&bitboard::BitIndex> = None;
        let bit_board = index.bitboard() | index.down().bitboard();
        let bit_board_opponent = index.down().right().bitboard() | index.down().left().bitboard();
        let expected = 1u64 << 41 | 1u64 << 43;
        // bitboard::BitBoard(result)
        println!("\n{}", bit_board_opponent);
        let result = gen_moves_for_pawn(
            index,
            &square::Color::Black,
            &bit_board,
            &bit_board_opponent,
            capture_en_passant,
        )
        .unwrap()
        .moves()
        .0;
        assert_eq!(result, expected);
    }
    #[test]
    fn test_pawn_black_row6() {
        let index = bitboard::BitIndex(50);
        let capture_en_passant: Option<&bitboard::BitIndex> = None;
        let bit_board = index.bitboard();
        let bit_board_opponent = bitboard::BitBoard::default();
        let expected = BitBoard::build(bitboard::BitIndex::union(vec![42, 34]));
        let result = gen_moves_for_pawn(
            index,
            &square::Color::Black,
            &bit_board,
            &bit_board_opponent,
            capture_en_passant,
        )
        .unwrap()
        .moves()
        .0;
        assert_eq!(result, expected.0);
    }
    ////////////////////////////////////////////////////////
    /// Check status
    ////////////////////////////////////////////////////////
    use crate::ui::notation::fen::{self, EncodeUserInput};

    #[test]
    fn test_check_rook() {
        let fen = "knbqrbnr/8/8/8/8/8/8/RNBQKBNR w KQkq - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        let check_status = position.check_status();
        let expected = CheckStatus::Simple {
            attacker: TypePiece::Rook,
            attacker_index: bitboard::BitIndex(60),
        };
        assert_eq!(check_status, expected);
    }
    #[test]
    fn test_check_bishop() {
        let fen = "bnbqkbnn/8/8/8/8/8/8/RNBQRBNK w KQkq - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        println!("{}", position.chessboard());
        let check_status = position.check_status();
        let expected = CheckStatus::Simple {
            attacker: TypePiece::Bishop,
            attacker_index: bitboard::BitIndex(56),
        };
        assert_eq!(check_status, expected);
    }
    #[test]
    fn test_check_queen() {
        let fen = "qnbbkbnn/8/8/8/8/8/8/RNBQRBNK w KQkq - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        println!("{}", position.chessboard());
        let check_status = position.check_status();
        let expected = CheckStatus::Simple {
            attacker: TypePiece::Queen,
            attacker_index: bitboard::BitIndex(56),
        };
        assert_eq!(check_status, expected);
    }
    #[test]
    fn test_check_knight() {
        let fen = "qnbbkbnn/8/8/8/8/8/5nPP/RNBQRBNK w KQkq - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        println!("{}", position.chessboard());
        let check_status = position.check_status();
        let expected = CheckStatus::Simple {
            attacker: TypePiece::Knight,
            attacker_index: bitboard::BitIndex(13),
        };
        assert_eq!(check_status, expected);
    }
    #[test]
    fn test_check_pawn() {
        let fen = "qnbbkbnn/8/8/8/8/8/6pP/RNBQRBNK w KQkq - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        println!("{}", position.chessboard());
        let check_status = position.check_status();
        let expected = CheckStatus::Simple {
            attacker: TypePiece::Pawn,
            attacker_index: bitboard::BitIndex(14),
        };
        assert_eq!(check_status, expected);
    }
    #[test]
    fn test_double_check() {
        let fen = "bnbqkbnr/8/8/8/8/8/8/RNBQRBNK w KQkq - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        let check_status = position.check_status();
        let expected = CheckStatus::Double;
        assert_eq!(check_status, expected);
    }
    ////////////////////////////////////////////////////////
    /// Generate moves with SimpleCheck
    ////////////////////////////////////////////////////////
    #[test]
    fn test_moves_check_knight() {
        let fen = "qnbbkbnn/8/8/8/8/8/5nPp/RNBQNRNK w KQkq - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        let check_status = position.check_status();
        let bit_position = bitboard::BitPosition::from(position);
        let (_attacker, attacker_index) = match check_status {
            CheckStatus::Simple {
                attacker,
                attacker_index,
            } => (attacker, attacker_index),
            _ => panic!("Should be check"),
        };
        let moves = gen_moves_for_all_simple_check(
            &square::Color::White,
            attacker_index,
            &bit_position.bit_boards_white_and_black().bit_board_white(),
            bit_position.bit_boards_white_and_black().bit_board_black(),
            None,
        );
        //let moves = moves.filter(|m| m.
        assert_eq!(moves.len(), 2);
        let move_king = moves.get(0).unwrap().moves().0;
        let move_rook = moves.get(1).unwrap().moves().0;
        let move_king_expected = 1u64 << 15;
        let move_rook_expected = 1u64 << 13;
        assert_eq!(move_rook, move_rook_expected);
        assert_eq!(move_king, move_king_expected);
        //assert_eq!(moves, expected);
    }
    ////////////////////////////////////////////////////////
    /// Generate moves for castle
    ////////////////////////////////////////////////////////
    #[test]
    fn test_moves_king_castle() {
        let fen = "qnbbkbnn/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        let bit_position = bitboard::BitPosition::from(position);
        let color = &bit_position.bit_position_status().player_turn();
        let bit_board = bit_position.bit_boards_white_and_black.bit_board(color);
        let bit_board_opponent = bit_position
            .bit_boards_white_and_black
            .bit_board(&color.switch());
        let white_king_bit_board = bit_board.king();
        let moves = KingBitBoard::new(white_king_bit_board.bitboard).gen_moves_no_check(
            &square::Color::White,
            bit_board,
            bit_board_opponent,
            bit_position
                .bit_position_status()
                .can_castle_king_side(bit_board.concat_bit_boards(), color),
            bit_position
                .bit_position_status()
                .can_castle_queen_side(bit_board.concat_bit_boards(), color),
        );
        let result = moves.get(0).unwrap().moves().value();
        let expected: u64 =
            1u64 << 3 | 1u64 << 5 | 1u64 << 11 | 1u64 << 12 | 1u64 << 13 | 1u64 << 2 | 1u64 << 6;
        assert_eq!(result, expected)
    }
    #[test]
    fn test_moves_king_cannot_castle() {
        let fen = "4kB1r/pr3ppp/p7/4P3/2K5/1P2B3/P4P1Q/2RRK3 b k - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        let bit_position = bitboard::BitPosition::from(position);
        let color = &bit_position.bit_position_status().player_turn();
        let bit_board = bit_position.bit_boards_white_and_black.bit_board(color);
        let bit_board_opponent = bit_position
            .bit_boards_white_and_black
            .bit_board(&color.switch());
        let white_king_bit_board = bit_board.king();
        let moves = KingBitBoard::new(white_king_bit_board.bitboard).gen_moves_no_check(
            &square::Color::White,
            bit_board,
            bit_board_opponent,
            bit_position
                .bit_position_status()
                .can_castle_king_side(bit_board.concat_bit_boards(), color),
            bit_position
                .bit_position_status()
                .can_castle_queen_side(bit_board.concat_bit_boards(), color),
        );
        let result = moves.get(0).unwrap().moves().value();
        // check cannot castle
        assert!(result & (1 << 62) == 0);
    }
    /////////////////////////
    #[test]
    fn test_game_play_castle() {
        let fen = "qnbbkbnn/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        let bit_board_position = bitboard::BitPosition::from(position);
        let color = square::Color::White;
        let moves = bit_board_position
            .bit_boards_white_and_black()
            .gen_moves_for_all(
                &color,
                CheckStatus::None,
                None,
                bit_board_position.bit_position_status(),
            );
        let mut m: Vec<(u8, u8)> = moves
            .iter()
            .map(|v| (v.start().value(), v.end().value()))
            .collect();
        let mut expected = vec![
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 8),
            (0, 16),
            (0, 24),
            (0, 32),
            (0, 40),
            (0, 48),
            (0, 56),
            (7, 5),
            (7, 6),
            (7, 15),
            (7, 23),
            (7, 31),
            (7, 39),
            (7, 47),
            (7, 55),
            (7, 63),
            (4, 2),
            (4, 3),
            (4, 5),
            (4, 6),
            (4, 11),
            (4, 12),
            (4, 13),
        ];
        m.sort();
        expected.sort();
        assert_eq!(m, expected);
        let short_castle_move: Vec<&bitboard::BitBoardMove> = moves
            .iter()
            .filter(|v| {
                v.type_piece() == TypePiece::King
                    && v.start() < v.end()
                    && v.end().value() - v.start().value() == 2
            })
            .collect();
        let short_castle = *short_castle_move.get(0).unwrap();
        assert_eq!(
            (short_castle.start().value(), short_castle.end().value()),
            (4u8, 6u8)
        );
        let expected = bitboard::BitBoardMove::new(
            square::Color::White,
            TypePiece::King,
            bitboard::BitIndex(4u8),
            bitboard::BitIndex(6u8),
            None,
            None,
        );
        let zobrist_table = zobrist::Zobrist::default();
        let mut hash = zobrist::ZobristHash::default();
        let bit_board_move = *short_castle;
        assert_eq!(bit_board_move, expected);
        let mut bit_board_position2 = bit_board_position.clone();
        bit_board_position2.move_piece(&bit_board_move, &mut hash, &zobrist_table);
        let position = bit_board_position2.to();
        let fen = fen::Fen::encode(&position).expect("Failed to encode position");
        println!("{}", position.chessboard());
        assert_eq!(fen, "qnbbkbnn/8/8/8/8/8/8/R4RK1 b kq - 1 1");
    }

    #[test]
    fn test_game_play_promotion() {
        let fen = "7k/P7/8/8/8/8/8/7K w - - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        let bit_board_position = bitboard::BitPosition::from(position);
        let color = square::Color::White;
        let moves = bit_board_position
            .bit_boards_white_and_black()
            .gen_moves_for_all(
                &color,
                CheckStatus::None,
                None,
                bit_board_position.bit_position_status(),
            );
        let promotion_moves: Vec<&bitboard::BitBoardMove> = moves
            .iter()
            .filter(|m| m.type_piece() == TypePiece::Pawn)
            .collect();
        let new_pieces: Vec<square::TypePiecePromotion> =
            promotion_moves.iter().flat_map(|p| p.promotion()).collect();
        assert_eq!(new_pieces.len(), 4);
        let promotion_move = promotion_moves.get(0).unwrap();
        let zobrist_table = zobrist::Zobrist::default();
        let mut hash = zobrist::ZobristHash::default();
        let mut bit_board_position2 = bit_board_position.clone();
        bit_board_position2.move_piece(&promotion_move, &mut hash, &zobrist_table);
        let position = bit_board_position2.to();
        let fen = fen::Fen::encode(&position).expect("Failed to encode position");
        println!("{}", position.chessboard());
        println!("{}", fen);
        assert_eq!(fen, "R6k/8/8/8/8/8/8/7K b - - 0 1");
    }

    // FIXME: do not use sleep.
    #[test]
    #[ignore]
    fn test_game_play_discovered_check() {
        let fen = "2b1k2r/2p5/2P1p2p/2b1ppP1/p3P2q/P2n4/Rr1N2K1/3q2R1 w - - 0 33";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        println!("{}", position.chessboard());
        let bit_board_position = bitboard::BitPosition::from(position);
        let color = square::Color::White;
        let moves = bit_board_position
            .bit_boards_white_and_black()
            .gen_moves_for_all(
                &color,
                CheckStatus::None,
                None,
                bit_board_position.bit_position_status(),
            );
        let moves: Vec<String> = moves
            .into_iter()
            .map(|m| long_notation::LongAlgebricNotationMove::build_from_b_move(m).cast())
            .collect();
        // cannot play this move since it would discover the king
        assert!(!moves.contains(&"d2b1".to_string()))
    }
}
