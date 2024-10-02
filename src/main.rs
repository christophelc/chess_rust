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
use game::player;
use piece_move::GenMoves;
use std::io;
use uci::command::parser;
use uci::event;
use uci::UciRead;
use uci::UciReadWrapper;

use std::env;

fn fen() {
    println!("chessboard generated from initial position encoded with FEN");
    let position: fen::Position = fen::Position::build_initial_position();
    println!("{}", position.chessboard());
    let fen_str = fen::Fen::encode(&position).expect("Error when decoding position to FEN format.");
    println!("Encode initial position to FEN position:");
    println!("{}", fen_str);
}

async fn test(game_actor: &game::GameActor<engine::EngineDummy>) {
    let mut stdout = io::stdout();
    println!("Inital position with move e4");
    let input = "position startpos moves e2e4 ";
    let parser = parser::InputParser::new(input);
    let command = parser.parse_input().expect("Invalid command");
    let _ = uci::execute_command(game_actor, command, &mut stdout, true).await;
    let configuration = game_actor
        .send(game::GetConfiguration)
        .await
        .expect("actix error")
        .expect("Error when retrieving configuration");
    let position = configuration.opt_position().expect("No position defined.");
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

async fn uci_loop(game_actor: &game::GameActor<engine::EngineDummy>, stdin: &mut io::Stdin) {
    let uci_reader = uci::UciReadWrapper::new(stdin);
    // we ignore errors (according to uci specifications)
    let _ = uci::uci_loop(uci_reader, game_actor).await;
}

async fn tui_loop<T: engine::EngineActor>(game_actor: &game::GameActor<T>, stdin: &mut io::Stdin) {
    // init the game
    let inputs = vec!["position startpos", "quit"];
    let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
    // we don't ignore error in tui mode
    uci::uci_loop(uci_reader, game_actor).await.unwrap();
    let mut stdin_reader = UciReadWrapper::new(stdin);
    // loop
    loop {
        let configuration = game_actor
            .send(game::GetConfiguration)
            .await
            .expect("actix error")
            .expect("Error when retrieving configuration");
        println!("\n{}", configuration.opt_position().unwrap().chessboard());
        let input = stdin_reader.uci_read();
        let input = input.trim();
        match input {
            "quit" => break,
            // e2e4 for example
            _ if input.len() == 4 => {
                let moves = vec![input.to_string()];
                match event::moves_validation(&moves) {
                    Err(err) => println!("Error: {}", err),
                    Ok(long_algebric_moves) => {
                        let result = game_actor
                            .send(game::PlayMoves(long_algebric_moves))
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
    let mut stdin = io::stdin();
    let mut game = game::Game::<engine::EngineDummy>::new();
    let engine_player1 = engine::EngineDummy::default().start();
    let engine_player2 = engine::EngineDummy::default().start();
    let player1 = player::Player::Human {
        engine_opt: Some(engine_player1),
    };
    let player2 = player::Player::Computer {
        engine: engine_player2,
    };
    let players = player::Players::new(player1, player2);
    game.set_players(players);
    let game_actor = game.start();

    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Entering in uci mode");
        //fen();
        //test(&game_actor).await;
        println!("Enter an uci command:");
        uci_loop(&game_actor, &mut stdin).await;
    } else {
        println!("Entering in tui mode");
        println!("{:?}", args);
        tui_loop(&game_actor, &mut stdin).await;
    }
}
