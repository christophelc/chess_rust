mod board;
mod game;
mod uci;
use board::bitboard::piece_move;
use board::bitboard::BitPosition;
use board::fen;
use board::san;
use fen::EncodeUserInput;
use piece_move::GenMoves;
use std::io;
use uci::command::parser;

use actix::prelude::*;

fn fen() {
    println!("chessboard generated from initial position encoded with FEN");
    let position: fen::Position = fen::Position::build_initial_position();
    println!("{}", position.chessboard());
    let fen_str = fen::FEN::encode(&position).expect("Error when decoding position to FEN format.");
    println!("Encode initial position to FEN position:");
    println!("{}", fen_str);
}

async fn test(game_actor: &game::GameActor) {
    let mut stdout = io::stdout();
    println!("Inital position with move e4");
    let input = "position startpos moves e2e4 ";
    let parser = parser::InputParser::new(&input);
    let command = parser.parse_input().expect("Invalid command");
    let _ = uci::execute_command(&game_actor, command, &mut stdout, true).await;
    let configuration = game_actor
        .send(game::GetConfiguration)
        .await
        .unwrap()
        .unwrap();
    let position = configuration.opt_position().unwrap();
    println!("{}", position.chessboard());
    println!();

    println!("Generate moves for white king considering pawn is in e4");
    let bit_position = BitPosition::from(position);
    let moves = bit_position.bit_boards_white_and_black().gen_moves_for_all(
        &board::square::Color::White,
        piece_move::CheckStatus::NoCheck,
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

async fn uci_loop(game_actor: &game::GameActor, stdin: &mut io::Stdin) {
    let uci_reader = uci::UciReadWrapper::new(stdin);
    uci::uci_loop(uci_reader, &game_actor).await;
}

#[actix::main]
async fn main() {
    let mut stdin = io::stdin();
    let game_actor = game::Game::new().start();

    fen();
    test(&game_actor).await;
    println!("Enter an uci command:");
    uci_loop(&game_actor, &mut stdin).await;
}
