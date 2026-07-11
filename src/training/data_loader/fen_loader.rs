use std::collections::HashMap;
use std::io::BufRead;

use crate::entity::game::component::bitboard::{self, zobrist};
use crate::ui::notation::fen::{self, EncodeUserInput};
use crate::training::nn::topology::{self, KrkItem};
use crate::training::nn::dataset::KrkDataset;
use crate::ui::board;
use crate::entity::game::component::{square, coord};

pub type FenLineMap = HashMap<zobrist::ZobristHash, FenLineEnriched>;

#[derive(Debug, Clone)]
pub struct FenLine {
    fen: String,
    mat_in: Option<u8>,
    move_str: String,
}

impl FenLine {
    pub fn new(fen: &str, mat_in: Option<u8>, move_str: &str) -> FenLine {
        FenLine {
            fen: fen.to_string(),
            mat_in,
            move_str: move_str.to_string(),
        }
    }

    pub fn fen(&self) -> &String {
        &self.fen
    }

    pub fn mat_in(&self) -> Option<u8> {
        self.mat_in
    }

    pub fn move_str(&self) -> &String {
        &self.move_str
    }

    pub fn append_move_str(&mut self, new_move_str: &str) {
        self.move_str = format!("{},{}", self.move_str, new_move_str)
    }
}

#[derive(Debug, Clone)]
pub struct FenLineEnriched {
    fen_line: FenLine,
    hash: zobrist::ZobristHash,
}

impl FenLineEnriched {
    pub fn new(fen_line: FenLine, hash: &zobrist::ZobristHash) -> FenLineEnriched {
        FenLineEnriched {
            fen_line,
            hash: hash.clone(),
        }
    }

    pub fn hash(&self) -> &zobrist::ZobristHash {
        &self.hash
    }

    pub fn fen_line(&self) -> &FenLine {
        &self.fen_line
    }

    pub fn append_move_str(&mut self, new_move_str: &str) {
        self.fen_line.append_move_str(new_move_str);
    }
}

impl std::fmt::Display for FenLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{};{};{}", self.fen, self.mat_in.map(|v| v.to_string()).unwrap_or_default(), self.move_str)
    }
}

fn parse_mat_in(mat_in_str: &str) -> Option<u8> {
    mat_in_str.parse::<u8>().ok()
}

pub fn extract_fields(line: &str) -> FenLine {
    let mut parts = line.split(';');
    let fen = parts.next().unwrap_or("").to_string();
    let mat_in = parts.next().unwrap_or("").to_string();
    let move_str = parts.next().unwrap_or("").to_string();
    FenLine {
        fen,
        mat_in: parse_mat_in(&mat_in),
        move_str,
    }
}

pub fn to_fen_lines_enriched(
    lines: &[String],
    zobrist_table: &zobrist::Zobrist,
) -> Vec<FenLineEnriched> {
    let mut v: Vec<FenLineEnriched> = vec![];
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let fen_line = extract_fields(line);
        let hash = fen_hash(&fen_line.fen, zobrist_table);
        v.push(FenLineEnriched { fen_line, hash });
    }
    v
}

pub fn fen_hash(fen: &str, zobrist_table: &zobrist::Zobrist) -> zobrist::ZobristHash {
    let position = fen::Fen::decode(fen).unwrap();
    let bit_position = bitboard::BitPosition::from(position);
    zobrist::ZobristHash::zobrist_partial_hash_from_position(&bit_position, zobrist_table)
}

pub fn known_mat_to_map(fen_lines_enriched: &[FenLineEnriched]) -> FenLineMap {
    let zobrist_table = zobrist::Zobrist::new();
    let mut m: FenLineMap = HashMap::new();
    for fen_line_enriched in fen_lines_enriched
        .iter()
        .filter(|line| line.fen_line.mat_in.is_some())
    {
        let hash = fen_hash(&fen_line_enriched.fen_line.fen, &zobrist_table);
        m.insert(hash, fen_line_enriched.clone());
    }
    m
}

pub fn fens_to_map(
    fens: &[String],
    zobrist_table: &zobrist::Zobrist,
) -> HashMap<zobrist::ZobristHash, String> {
    let mut m: HashMap<zobrist::ZobristHash, String> = HashMap::new();
    for fen in fens {
        let hash = fen_hash(fen, zobrist_table);
        m.insert(hash, fen.to_string());
    }
    m
}

pub fn read_lines<R>(reader: R) -> std::io::Result<Vec<FenLineEnriched>>
where
    R: BufRead,
{
    let lines: Vec<String> = reader.lines().collect::<std::io::Result<Vec<_>>>()?;

    let zobrist_table = zobrist::Zobrist::new();
    let fen_lines = to_fen_lines_enriched(&lines, &zobrist_table);

    Ok(fen_lines)
}

pub fn to_fen_lines_map(fen_lines_enriched: &[FenLineEnriched]) -> FenLineMap {
    let mut fen_lines_map: FenLineMap = HashMap::new();
    for fen_line_enriched in fen_lines_enriched {
        fen_lines_map.insert(fen_line_enriched.hash.clone(), fen_line_enriched.clone());
    }
    fen_lines_map
}

pub fn read_fens_as_map<R>(reader: R) -> std::io::Result<FenLineMap>
where
    R: BufRead,
{
    let fen_lines_enriched = read_lines(reader)?;
    Ok(to_fen_lines_map(&fen_lines_enriched))
}

pub fn fen_to_bitboards(fen: &str) -> bitboard::BitBoardsWhiteAndBlack {
    let position: fen::Position = fen::Fen::decode(fen).unwrap();
    let chessboard = position.chessboard();
    bitboard::BitBoardsWhiteAndBlack::from(*chessboard)
}

fn krk_fen_to_prob_moves(fen_line: &FenLine) -> Result<HashMap<square::TypePiece, f32>, String> {
    let position: fen::Position = fen::Fen::decode(fen_line.fen()).unwrap();
    let chessboard: &board::ChessBoard = position.chessboard();
    let moves: Vec<&str> = fen_line.move_str().split(",").collect();
    let mut counts: HashMap<square::TypePiece, f32> = HashMap::new();
    let n_moves = moves.len();

    for m in &moves {
        let square = chessboard.at(coord::Coord::from_bytes(&m[0..2]));
        match square {
            square::Square::Empty => {
                    return Err(format!("Internal error. Invalid move {} associated with fen_line: {}", m, fen_line));
            },
            square::Square::NonEmpty(piece) => {
                    *counts.entry(piece.type_piece()).or_insert(0.0) += 1.0/n_moves as f32;
            }
        }
    }
    Ok(counts)
}

pub fn fen_line_to_krk_item(fen_line: &FenLine) -> KrkItem {
    let bitboards: [u64; 3] = fen_krk_to_bitboards(&fen_line.fen);
    let features: [f32; 192] = bitboards_to_features(bitboards);

    let probs: HashMap<square::TypePiece, f32> = krk_fen_to_prob_moves(&fen_line).unwrap();
    topology::create_item(features, probs, fen_line.mat_in.unwrap())
}

pub fn fens_to_dataset(fen_map: FenLineMap) -> KrkDataset {
    let mut items: Vec<KrkItem> = vec![];
    for (_hash, fen_line_enriched) in fen_map {
        let item = fen_line_to_krk_item(&fen_line_enriched.fen_line);
        items.push(item);
    }
    KrkDataset::new(items)
}

//////////////////////////
// nn encoding preparation

pub fn fen_to_full_bitboards(fen: &str) -> [u64; 12] {
    let bitboards = fen_to_bitboards(fen);
    [
        bitboards.bit_board_white().king().bitboard().value(),        
        bitboards.bit_board_white().rooks().bitboard().value(),
        bitboards.bit_board_white().bishops().bitboard().value(),        
        bitboards.bit_board_white().knights().bitboard().value(),        
        bitboards.bit_board_white().queens().bitboard().value(),        
        bitboards.bit_board_white().pawns().bitboard().value(),        
        bitboards.bit_board_black().king().bitboard().value(),        
        bitboards.bit_board_black().rooks().bitboard().value(),
        bitboards.bit_board_black().bishops().bitboard().value(),        
        bitboards.bit_board_black().knights().bitboard().value(),        
        bitboards.bit_board_black().queens().bitboard().value(),        
        bitboards.bit_board_black().pawns().bitboard().value(),        
    ]
}

pub fn fen_krk_to_bitboards(fen: &str) -> [u64; 3] {
    let bitboards = fen_to_bitboards(fen);
    [
        bitboards.bit_board_white().king().bitboard().value(),        
        bitboards.bit_board_white().rooks().bitboard().value(),
        bitboards.bit_board_black().rooks().bitboard().value(),
    ]
}

pub fn bitboards_to_features(bitboards: [u64; 3]) -> [f32; 192] {
    let mut features = [0.0_f32; 192];

    for channel in 0..3 {
        for square in 0..64 {
            let occupied = (bitboards[channel] >> square) & 1;
            features[channel * 64 + square] = occupied as f32;
        }
    }

    features
}



#[cfg(test)]

#[test]
fn test_krk_fen_to_prob_moves() {
    use crate::entity::game::component::square;

    let line = "8/8/3k4/8/8/8/8/KR6 w - - 0 0;13;a1a2,b1b4,b1b5,b1h1,b1d1,a1b2";
    let fen_line = extract_fields(line);    
    
    let expected_king = 2.0 / 6.0;
    let expected_rook = 4.0 / 6.0;
    let probs: HashMap<square::TypePiece, f32> = krk_fen_to_prob_moves(&fen_line).unwrap();

    assert!((probs[&square::TypePiece::King] - expected_king).abs() < 1e-6);
    assert!((probs[&square::TypePiece::Rook] - expected_rook).abs() < 1e-6);
}