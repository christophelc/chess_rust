use crate::{entity::game::component::bitboard::zobrist, training::data_loader::fen_loader};
use rand::{
    rngs::StdRng,
    seq::SliceRandom,
    SeedableRng,
};
use std::collections::HashMap;

pub struct DataSplit {
    pub train: fen_loader::FenLineMap,
    pub validation: fen_loader::FenLineMap,
    pub test: fen_loader::FenLineMap,
}

pub fn split(
    fen_lines_map: fen_loader::FenLineMap,
    seed: u64,
) -> DataSplit {
    let mut groups: HashMap<u8, Vec<(zobrist::ZobristHash, fen_loader::FenLineEnriched)>> =
        HashMap::new();

    // Regroupement par mat_in.
    for (hash, enriched) in fen_lines_map {
        let mat_in = enriched.fen_line().mat_in().unwrap();

        groups
            .entry(mat_in)
            .or_default()
            .push((hash, enriched));
    }

    let mut train = fen_loader::FenLineMap::new();
    let mut validation = fen_loader::FenLineMap::new();
    let mut test = fen_loader::FenLineMap::new();

    let mut rng = StdRng::seed_from_u64(seed);

    for (_mat_in, mut entries) in groups {
        entries.shuffle(&mut rng);

        let len = entries.len();

        let train_len = len * 80 / 100;
        let validation_len = len * 10 / 100;

        for (index, (hash, enriched)) in entries.into_iter().enumerate() {
            if index < train_len {
                train.insert(hash, enriched);
            } else if index < train_len + validation_len {
                validation.insert(hash, enriched);
            } else {
                test.insert(hash, enriched);
            }
        }
    }

    DataSplit {
        train,
        validation,
        test,
    }
}
