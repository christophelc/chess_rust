use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use actix::Actor;

use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::{
    entity::{
        engine::component::engine_iddfs,
        game::{
            actor::game_manager,
            component::{
                bitboard::{self, zobrist},
                game_state,
            },
        },
    },
    ui::notation::{self, epd},
};

use super::launcher;

#[derive(Debug)]
pub struct MovePlayed {
    mv: String,
    is_played: bool,
}

#[derive(Debug)]
pub struct EpdEval {
    id: String,
    am: Option<MovePlayed>,
    bm: Option<MovePlayed>,
    duration_ms: u128,
}
impl EpdEval {
    pub fn new(
        id: String,
        am: Option<MovePlayed>,
        bm: Option<MovePlayed>,
        duration_ms: u128,
    ) -> Self {
        Self {
            id,
            am,
            bm,
            duration_ms,
        }
    }
}

#[derive(Debug, Default)]
pub struct EpdScore {
    am_count: u64,
    am_ok: u64,
    bm_count: u64,
    bm_ok: u64,
    time_bonus: u64,
}
impl EpdScore {
    pub fn score(&self) -> f64 {
        0.7 * self.bm_ok as f64 + 0.2 * self.am_ok as f64 + 0.1 * self.time_bonus as f64
    }
}
impl std::fmt::Display for EpdScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "score: {}", self.score())
    }
}

fn init_game_params(depth: u8) -> engine_iddfs::EngineIddfs {
    let game_manager = game_manager::GameManager::new(None);
    let mut engine_player =
        engine_iddfs::EngineIddfs::new(None, game_manager.zobrist_table(), depth);
    engine_player.set_id_number("computer");
    engine_player
}
pub fn scoring(epd_data: &launcher::EpdData) -> f64 {
    let zobrist_table = zobrist::Zobrist::new();
    // TODO: replace by a config
    let conf_depth = 3;
    let conf_max_time_sec = 3;
    let engine = init_game_params(conf_depth);

    let epd_evals: Vec<EpdEval> = epd_data
        .epds()
        .into_iter()
        .map(|epd| epd_eval(epd, conf_max_time_sec, &zobrist_table, &engine))
        .collect();
    let scores: Vec<EpdScore> = epd_evals
        .into_iter()
        .map(|epd_eval| epd_score(&epd_eval, conf_max_time_sec))
        .collect();
    let total = scores.iter().map(|epd_score| epd_score.score()).sum();
    total
}

fn epd_score(epd_eval: &EpdEval, max_duration_sec: u64) -> EpdScore {
    let mut epd_score = EpdScore::default();
    if let Some(am) = &epd_eval.am {
        epd_score.am_count += 1;
        if !am.is_played {
            epd_score.am_ok += 1;
        }
    }
    if let Some(bm) = &epd_eval.bm {
        epd_score.bm_count += 1;
        if bm.is_played {
            epd_score.bm_ok += 1;
        }
    }
    if epd_eval.duration_ms < max_duration_sec as u128 * 1000 {
        epd_score.time_bonus += 1;
    }
    epd_score
}

fn epd_eval(
    epd_el: &epd::Epd,
    max_duration_sec: u64,
    zobrist_table: &zobrist::Zobrist,
    engine_iddfs: &engine_iddfs::EngineIddfs,
) -> EpdEval {
    let position = epd_el.position();
    let game = game_state::GameState::new(*position, &zobrist_table);
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = Arc::clone(&stop_flag);
    let engine_dispatcher =
        dispatcher::EngineDispatcher::new(Arc::new(engine_iddfs.clone()), None, None);
    let self_actor = engine_dispatcher.start();

    // Start a thread to set the flag to true after 1 minute
    thread::spawn({
        let stop_flag_clone = Arc::clone(&stop_flag); // Clone for use in the thread
        move || {
            thread::sleep(std::time::Duration::from_secs(max_duration_sec)); // Wait for 1 minute
            stop_flag_clone.store(true, Ordering::SeqCst); // Set the flag to true
            println!("Flag set to true after {} seconds.", max_duration_sec);
        }
    });

    let mut b_move_opt: Option<bitboard::BitBoardMove> = None;
    let start = std::time::Instant::now();
    while !stop_flag.load(Ordering::SeqCst) {
        let b_move = engine_iddfs.iddfs_init(&game, self_actor.clone(), None, &stop_flag_clone);
        b_move_opt = Some(b_move);
        stop_flag_clone.store(true, Ordering::SeqCst); // Set the flag to true
    }
    let duration = start.elapsed();

    let id = epd_el
        .operations()
        .iter()
        .filter_map(|op| match &op {
            &epd::EpdOperation::Id(id) => Some(id.clone()),
            _ => None,
        })
        .collect();
    let ams: Vec<String> = epd_el
        .operations()
        .iter()
        .filter_map(|op| match &op {
            &epd::EpdOperation::Am {
                san: _,
                long_notation,
            } => Some(long_notation.clone()),
            _ => None,
        })
        .collect();
    let bms: Vec<String> = epd_el
        .operations()
        .iter()
        .filter_map(|op| match &op {
            &epd::EpdOperation::Bm {
                san: _,
                long_notation,
            } => Some(long_notation.clone()),
            _ => None,
        })
        .collect();

    let am_moved_played: Option<MovePlayed>;
    let bm_moved_played: Option<MovePlayed>;
    match b_move_opt {
        Some(b_move) => {
            let move_str =
                notation::long_notation::LongAlgebricNotationMove::build_from_b_move(b_move).cast();
            let am_played = ams.contains(&move_str);
            am_moved_played = if ams.is_empty() {
                None
            } else {
                Some(MovePlayed {
                    mv: ams.join(" "),
                    is_played: am_played,
                })
            };
            let bm_played = bms.contains(&move_str);
            bm_moved_played = if bms.is_empty() {
                None
            } else {
                Some(MovePlayed {
                    mv: bms.join(" "),
                    is_played: bm_played,
                })
            };
        }
        None => {
            am_moved_played = if ams.is_empty() {
                None
            } else {
                Some(MovePlayed {
                    mv: ams.join(" "),
                    is_played: false,
                })
            };
            bm_moved_played = if bms.is_empty() {
                None
            } else {
                Some(MovePlayed {
                    mv: bms.join(" "),
                    is_played: false,
                })
            };
        }
    }
    let epd_eval = EpdEval::new(id, am_moved_played, bm_moved_played, duration.as_millis());
    epd_eval
}
