use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use actix::Actor;
use chess_actix::entity;
use chess_actix::entity::engine::component::config::config;
use chess_actix::entity::engine::component::engine_mat;
use chess_actix::entity::engine::component::evaluation::stat_eval;
use chess_actix::entity::game::actor::game_manager;
use chess_actix::entity::game::component::bitboard::zobrist;
use chess_actix::entity::game::component::game_state;
use chess_actix::monitoring::debug;
use chess_actix::ui::notation::fen::{self, EncodeUserInput, Position};
use entity::engine::actor::engine_dispatcher as dispatcher;

async fn try_mat_in(max_depth: u8, position: &Position) -> Option<u8> {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        // mat in 3
        let game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
        //let mut engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let engine_player1 = engine_mat::EngineMat::new(
            debug_actor_opt.clone(),
            game_manager.zobrist_table(),
            &config::MatConfig::new(max_depth),
        );
        let engine_player1_dispatcher = dispatcher::EngineDispatcher::new(
            Arc::new(engine_player1.clone()),
            debug_actor_opt.clone(),
            None,
        );
        let self_actor = engine_player1_dispatcher.start();
        let zobrist_table = &zobrist::Zobrist::new();
        let game = game_state::GameState::new(*position, zobrist_table);
        let mut stat_eval = stat_eval::StatEval::default();
        let flag_stop = Arc::new(AtomicBool::new(false));
        let mat_move_opt =
            engine_player1.mat_solver_init(&game, self_actor, None, &config::MatConfig::new(max_depth), &mut stat_eval, &flag_stop);
        mat_move_opt.map(|move_mat| move_mat.mat_in())
}

async fn update_line(line: &str) -> String {
    let mut parts = line.split(';');
    let fen = parts.next().unwrap_or("");
    let mat_in = parts.next().unwrap_or("");
    if mat_in.is_empty() {
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        //println!("{}", position.clone().chessboard());
        if let Some(mat_in) = try_mat_in(1, &position).await {
            format!("{};{}", fen, mat_in)
        } else {
            line.to_string()
        }
    } else {
        line.to_string()
    }
}

async fn update_file(input: &str, output: &str) -> std::io::Result<()> {
    let infile = File::open(input)?;
    let reader = BufReader::new(infile);

    let outfile = File::create(output)?;
    let mut writer = BufWriter::new(outfile);

    for (i, line) in reader.lines().enumerate() {
        let line = line?;

        if i == 0 {
            // header
            writeln!(writer, "{}", line)?;
            continue;
        }

        let updated = update_line(&line).await;
        writeln!(writer, "{}", updated)?;
    }

    writer.flush()?;
    Ok(())
}

#[actix::main]
async fn main() {
    let input = "krk_position.csv";
    let output = "krk_position_updated.csv";

    if let Err(e) = update_file(input, output).await {
        eprintln!("Error updating file: {}", e);
    } else {
        println!("File updated successfully.");
    }
}

#[actix::test]
async fn test_update_line() {
    let line = "6k1/8/6K1/8/8/8/8/R7 w - - 0 1";
    let updated = update_line(&line).await;
    assert_eq!(updated, "6k1/8/6K1/8/8/8/8/R7 w - - 0 1;1");
}