use std::collections::HashMap;
use std::io::BufRead;

use crate::entity::game::component::bitboard::{self, zobrist};
use crate::ui::notation::fen::{self, EncodeUserInput};

pub type FenLineMap = HashMap<zobrist::ZobristHash, FenLineEnriched>;

#[derive(Debug, Clone)]
pub struct FenLine {
    fen: String,
    mat_in: String,
    move_str: String,
}

impl FenLine {
    pub fn new(fen: &str, mat_in: u8, move_str: &str) -> FenLine {
        FenLine {
            fen: fen.to_string(),
            mat_in: mat_in.to_string(),
            move_str: move_str.to_string(),
        }
    }

    pub fn fen(&self) -> &String {
        &self.fen
    }

    pub fn mat_in(&self) -> Option<u8> {
        if let Ok(mat_in) = &self.mat_in.parse::<u8>() {
            Some(*mat_in)
        } else {
            None
        }
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
        write!(f, "{};{};{}", self.fen, self.mat_in, self.move_str)
    }
}

pub fn extract_fields(line: &str) -> FenLine {
    let mut parts = line.split(';');
    let fen = parts.next().unwrap_or("").to_string();
    let mat_in = parts.next().unwrap_or("").to_string();
    let move_str = parts.next().unwrap_or("").to_string();
    FenLine {
        fen,
        mat_in,
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
        .filter(|line| !line.fen_line.mat_in.is_empty())
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
