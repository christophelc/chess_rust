use chess_actix::entity::engine::component::{
    engine_alphabeta, engine_alphabeta_iterative, engine_minimax,
};
use chess_actix::entity::stat::actor::stat_entity;
use chess_actix::{entity, monitoring, ui};

use actix::Actor;
use entity::game::actor::game_manager;
use entity::game::component::square;
use std::env;
use std::io;
use std::sync::Arc;
use std::sync::Mutex;

use entity::engine::actor::engine_dispatcher as dispatcher;
use entity::engine::component::engine_dummy as dummy;
use entity::game::component::bitboard::{
    piece_move::{self, GenMoves},
    BitPosition,
};
use entity::game::component::player;
use entity::uci::actor::uci_entity::{self, UciRead};
use fen::EncodeUserInput;
use monitoring::debug;
use ui::notation::{fen, san};

const DEPTH: u8 = 3;

#[allow(dead_code)]
fn fen() {
    println!("chessboard generated from initial position encoded with FEN");
    let position: fen::Position = fen::Position::build_initial_position();
    println!("{}", position.chessboard());
    let fen_str = fen::Fen::encode(&position).expect("Error when decoding position to FEN format.");
    println!("Encode initial position to FEN position:");
    println!("{}", fen_str);
}

#[allow(dead_code)]
async fn test(game_manager_actor: &game_manager::GameManagerActor) {
    println!("Inital position with move e4");
    let inputs = vec!["position startpos moves e2e4 "];
    let uci_reader = Box::new(uci_entity::UciReadVecStringWrapper::new(&inputs));
    let uci_entity = uci_entity::UciEntity::new(uci_reader, game_manager_actor.clone(), None, None);
    let uci_entity_actor = uci_entity.start();
    uci_entity_actor.do_send(uci_entity::handler_read::ReadUserInput);
    let game_state = game_manager_actor
        .send(game_manager::handler_game::GetGameState)
        .await
        .expect("actix error")
        .expect("Error when retrieving game_state");
    let position = game_state.bit_position().to();
    println!("{}", position.chessboard());
    println!();

    println!("Generate moves for white king considering pawn is in e4");
    let bit_position = BitPosition::from(position);
    let moves = bit_position.bit_boards_white_and_black().gen_moves_for_all(
        &square::Color::White,
        piece_move::CheckStatus::None,
        None,
        bit_position.bit_position_status(),
    );
    let moves_as_str: Vec<String> = moves
        .iter()
        .map(|m| {
            san::san_to_str(m, &moves, &san::Lang::LangFr)
                .info()
                .clone()
        })
        .collect();
    println!("{:?}", moves_as_str);
}

#[allow(dead_code)]
fn uci_loop(
    game_manager_actor: &game_manager::GameManagerActor,
    stdin: &mut Arc<Mutex<io::Stdin>>,
) {
    let uci_reader = Box::new(uci_entity::UciReadWrapper::new(stdin.clone()));
    let uci_entity = uci_entity::UciEntity::new(uci_reader, game_manager_actor.clone(), None, None);
    let uci_entity_actor = uci_entity.start();
    uci_entity_actor.do_send(uci_entity::handler_read::ReadUserInput);
    // // we ignore errors (according to uci specifications)
    // let r = uci::uci_loop(uci_reader, game_actor).await;
    // println!("{:?}", r.err());
}

async fn tui_loop(
    game_manager_actor: &game_manager::GameManagerActor,
    stdin: &mut Arc<Mutex<io::Stdin>>,
) {
    // init the game
    let inputs = vec!["position startpos"];
    let uci_reader = Box::new(uci_entity::UciReadVecStringWrapper::new(&inputs));
    // we don't ignore error in tui mode
    let debug_actor = debug::DebugEntity::default().start();
    let uci_entity = uci_entity::UciEntity::new(
        uci_reader,
        game_manager_actor.clone(),
        Some(debug_actor),
        None,
    );
    let uci_entity_actor = uci_entity.start();
    // read all inputs and execute UCI commands
    for _idx in 0..inputs.len() {
        let _ = uci_entity_actor
            .send(uci_entity::handler_read::ReadUserInput)
            .await;
    }
    //let r = uci::uci_loop(uci_reader, game_actor).await;
    //println!("{:?}", r.err());
    let mut stdin_reader = uci_entity::UciReadWrapper::new(stdin.clone());
    // loop
    loop {
        let game_state = game_manager_actor
            .send(game_manager::handler_game::GetGameState)
            .await
            .expect("actix error")
            .expect("Error when retrieving game_state");
        println!("\n{}", game_state.bit_position().to().chessboard());
        let input_opt = stdin_reader.uci_read();
        match input_opt.as_deref() {
            None => {}
            Some("quit") => break,
            // e2e4 for example
            Some(input) if input.len() == 4 => {
                let moves = vec![input.to_string()];
                match uci_entity::handler_event::moves_validation(&moves) {
                    Err(err) => println!("Error: {}", err),
                    Ok(long_algebric_moves) => {
                        let result = game_manager_actor
                            .send(game_manager::handler_game::PlayMoves::new(
                                long_algebric_moves,
                            ))
                            .await
                            .unwrap();
                        if let Some(err) = result.err() {
                            println!("Move error: {}", err);
                        }
                    }
                }
            }
            _ => println!("Please enter a move to a format like e2e4"),
        }
    }
}

#[actix::main]
async fn main() {
    let debug_actor_opt: Option<debug::DebugActor> = None;
    let stat_actor_opt = Some(stat_entity::StatEntity::new(None).start());
    //let debug_actor_opt: Option<debug::DebugActor> = Some(debug::DebugEntity::new(true).start());
    let mut stdin = Arc::new(Mutex::new(io::stdin()));
    let mut game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
    //let mut engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
    let mut engine_player1 = engine_alphabeta_iterative::EngineAlphaBetaIterative::new(
        debug_actor_opt.clone(),
        game_manager.zobrist_table(),
        DEPTH,
    );
    engine_player1.set_id_number("white");
    let engine_player1_dispatcher =
        dispatcher::EngineDispatcher::new(Arc::new(engine_player1), debug_actor_opt.clone(), None);
    //let mut engine_player2 = dummy::EngineDummy::new(debug_actor_opt.clone());
    let mut engine_player2 = engine_alphabeta_iterative::EngineAlphaBetaIterative::new(
        debug_actor_opt.clone(),
        game_manager.zobrist_table(),
        DEPTH,
    );
    engine_player2.set_id_number("black");
    let engine_player2_dispatcher =
        dispatcher::EngineDispatcher::new(Arc::new(engine_player2), debug_actor_opt.clone(), None);
    let player1 = player::Player::Human {
        engine_opt: Some(engine_player1_dispatcher.start()),
    };
    let player2 = player::Player::Computer {
        engine: engine_player2_dispatcher.start(),
    };
    let players = player::Players::new(player1, player2);
    game_manager.set_players(players);
    let game_manager_actor = game_manager.start();
    let uci_reader = Box::new(uci_entity::UciReadWrapper::new(stdin.clone()));
    let uci_entity = uci_entity::UciEntity::new(
        uci_reader,
        game_manager_actor.clone(),
        debug_actor_opt.clone(),
        stat_actor_opt.clone(),
    );
    let uci_entity_actor = uci_entity.start();

    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Entering in uci mode");
        //fen();
        //test(&game_actor).await;
        println!("Enter an uci command:");

        loop {
            let _r = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await
                .expect("Actix error");
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    } else {
        println!("Entering in tui mode");
        println!("{:?}", args);
        tui_loop(&game_manager_actor, &mut stdin).await;
    }
}
