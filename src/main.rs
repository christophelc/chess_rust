mod board;
mod game;
mod uci;
use actix::Actor;
use board::bitboard::piece_move;
use board::bitboard::BitPosition;
use board::fen;
use board::san;
use fen::EncodeUserInput;
use game::engine;
use game::monitoring::debug;
use game::player;
use piece_move::GenMoves;
use std::io;
use std::sync::Arc;
use std::sync::Mutex;
use uci::event;
use uci::UciRead;
use uci::UciReadWrapper;

use std::env;

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
async fn test(game_manager_actor: &game::game_manager::GameManagerActor) {
    println!("Inital position with move e4");
    let inputs = vec!["position startpos moves e2e4 "];
    let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
    let uci_entity = uci::UciEntity::new(uci_reader, game_manager_actor.clone(), None);
    let uci_entity_actor = uci_entity.start();
    uci_entity_actor.do_send(uci::ReadUserInput);
    let game_state = game_manager_actor
        .send(game::game_manager::GetGameState)
        .await
        .expect("actix error")
        .expect("Error when retrieving game_state");
    let position = game_state.bit_position().to();
    println!("{}", position.chessboard());
    println!();

    println!("Generate moves for white king considering pawn is in e4");
    let bit_position = BitPosition::from(position);
    let moves = bit_position.bit_boards_white_and_black().gen_moves_for_all(
        &board::square::Color::White,
        piece_move::CheckStatus::None,
        &None,
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
    game_manager_actor: &game::game_manager::GameManagerActor,
    stdin: &mut Arc<Mutex<io::Stdin>>,
) {
    let uci_reader = uci::UciReadWrapper::new(stdin.clone());
    let uci_entity = uci::UciEntity::new(uci_reader, game_manager_actor.clone(), None);
    let uci_entity_actor = uci_entity.start();
    uci_entity_actor.do_send(uci::ReadUserInput);
    // // we ignore errors (according to uci specifications)
    // let r = uci::uci_loop(uci_reader, game_actor).await;
    // println!("{:?}", r.err());
}

async fn tui_loop(
    game_manager_actor: &game::game_manager::GameManagerActor,
    stdin: &mut Arc<Mutex<io::Stdin>>,
) {
    // init the game
    let inputs = vec!["position startpos", "quit"];
    let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
    // we don't ignore error in tui mode
    let debug_actor = debug::DebugEntity::default().start();
    let uci_entity = uci::UciEntity::new(uci_reader, game_manager_actor.clone(), Some(debug_actor));
    let uci_entity_actor = uci_entity.start();
    // read all inputs and execute UCI commands
    for _idx in 0..inputs.len() {
        let _ = uci_entity_actor.send(uci::ReadUserInput).await;
    }
    //let r = uci::uci_loop(uci_reader, game_actor).await;
    //println!("{:?}", r.err());
    let mut stdin_reader = UciReadWrapper::new(stdin.clone());
    // loop
    loop {
        let game_state = game_manager_actor
            .send(game::game_manager::GetGameState)
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
                match event::moves_validation(&moves) {
                    Err(err) => println!("Error: {}", err),
                    Ok(long_algebric_moves) => {
                        let result = game_manager_actor
                            .send(game::game_manager::PlayMoves::new(long_algebric_moves))
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
    let mut stdin = Arc::new(Mutex::new(io::stdin()));
    let mut game_manager = game::game_manager::GameManager::new(None);
    let engine_player1 = engine::EngineDummy::new(debug_actor_opt.clone()).set_id_number("white");
    let engine_player1_dispatcher =
        engine::EngineDispatcher::new(Arc::new(engine_player1), debug_actor_opt.clone());
    let engine_player2 = engine::EngineDummy::new(debug_actor_opt.clone()).set_id_number("black");
    let engine_player2_dispatcher =
        engine::EngineDispatcher::new(Arc::new(engine_player2), debug_actor_opt.clone());
    let player1 = player::Player::Human {
        engine_opt: Some(engine_player1_dispatcher.start()),
    };
    let player2 = player::Player::Computer {
        engine: engine_player2_dispatcher.start(),
    };
    let players = player::Players::new(player1, player2);
    game_manager.set_players(players);
    let game_manager_actor = game_manager.start();
    let uci_reader = uci::UciReadWrapper::new(stdin.clone());
    let uci_entity = uci::UciEntity::new(
        uci_reader,
        game_manager_actor.clone(),
        debug_actor_opt.clone(),
    );
    let uci_entity_actor = uci_entity.start();

    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Entering in uci mode");
        //fen();
        //test(&game_actor).await;
        println!("Enter an uci command:");
        uci_entity_actor.do_send(uci::ReadUserInput);
    } else {
        println!("Entering in tui mode");
        println!("{:?}", args);
        tui_loop(&game_manager_actor, &mut stdin).await;
    }
}
