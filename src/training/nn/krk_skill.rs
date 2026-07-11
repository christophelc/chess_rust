use std::{fs::File, io::BufReader};

use crate::training::data_loader::fen_loader;

fn read_krk_positions(filename: &str) ->std::io::Result<fen_loader::FenLineMap> {
    let in_file = File::open(filename).unwrap_or_else(|_| panic!("Cannot open file {}", filename));
    let reader = BufReader::new(in_file);
    fen_loader::read_fens_as_map(reader)
}

fn extract_mat_in_n(data: fen_loader::FenLineMap, mat_in: u8) -> Vec<fen_loader::FenLine> {
    let mut v: Vec<fen_loader::FenLine> = vec![];
    for (_k, fen_line_enriched) in data.iter() {
        if fen_line_enriched.fen_line().mat_in() == Some(mat_in) {
            v.push(fen_line_enriched.fen_line().clone());
        }
    }
    v
}

#[cfg(test)]

#[actix::test]
async fn test_load_fen_lines_as_map() {
    let fens_map = read_krk_positions("database/krk.csv").expect("Cannot read krk.csv");
    assert!(!fens_map.is_empty());
    let key = fens_map.keys().next();
    assert!(key.is_some());
    let mat_in_one = extract_mat_in_n(fens_map, 1);
    assert_eq!(mat_in_one.len(), 1512);
}