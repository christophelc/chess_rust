pub mod command;
pub mod event;
pub mod notation;
use crate::game::{self, engine, game_manager};
use command::parser;
use std::io::{self, BufRead, Stdin, Stdout, Write};

pub enum UciResult {
    Quit,
    Continue,
    BestMove,
}

fn best_move_action(
    stdout: &mut Stdout,
    best_move: notation::LongAlgebricNotationMove,
) -> Result<(), io::Error> {
    let res = writeln!(stdout, "bestmove {}", best_move.cast());
    stdout.flush().unwrap();
    res
}

pub async fn uci_loop<T: UciRead, E: engine::EngineActor>(
    mut uci_reader: T,
    game_manager_actor: &game::game_manager::GameManagerActor<E>,
) -> Result<(), Vec<String>> {
    let mut stdout = io::stdout();
    let mut errors: Vec<String> = vec![];

    while let Some(input) = uci_reader.uci_read() {
        let parser = parser::InputParser::new(&input);
        let command_or_error = parser.parse_input();
        println!("{} ==> {:?}", input, command_or_error);
        match command_or_error {
            Ok(command) => {
                let r = execute_command(game_manager_actor, command, &mut stdout, true).await;
                if let Some(error) = r.clone().err() {
                    errors.extend(error);
                } else if r.unwrap() {
                    break;
                }
            }
            Err(err) => {
                println!("xxxx {}", err.to_string());
                errors.push(err.to_string())
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub trait UciRead {
    fn uci_read(&mut self) -> Option<String>;
}
pub struct UciReadWrapper<'a> {
    stdin: &'a mut Stdin,
}
impl<'a> UciReadWrapper<'a> {
    pub fn new(stdin: &'a mut Stdin) -> Self {
        UciReadWrapper { stdin }
    }
}

impl<'a> UciRead for UciReadWrapper<'a> {
    fn uci_read(&mut self) -> Option<String> {
        let mut input = String::new();
        self.stdin
            .lock()
            .read_line(&mut input)
            .expect("Failed to read line");
        // useful for testing purpose
        let s = input.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

pub struct UciReadVecStringWrapper<'a> {
    idx: usize,
    inputs: &'a [&'a str],
}
impl<'a> UciReadVecStringWrapper<'a> {
    pub fn new(inputs: &'a [&str]) -> Self {
        UciReadVecStringWrapper { idx: 0, inputs }
    }
}
impl<'a> UciRead for UciReadVecStringWrapper<'a> {
    fn uci_read(&mut self) -> Option<String> {
        if self.idx < self.inputs.len() {
            let result = self.inputs[self.idx];
            self.idx += 1;
            Some(result.to_string())
        } else {
            None
        }
    }
}

pub async fn execute_command<T: engine::EngineActor>(
    game_manager_actor: &game::game_manager::GameManagerActor<T>,
    command: command::Command,
    stdout: &mut Stdout,
    show_errors: bool,
) -> Result<bool, Vec<String>> {
    let mut is_quit = false;
    let mut errors: Vec<String> = vec![];
    let events = command.handle_command(game_manager_actor).await;
    for event in &events {
        // pdate the configuration
        let uci_result = event.handle_event(game_manager_actor, stdout).await;
        // quit, stop and show best move or continue
        match uci_result {
            Ok(UciResult::Continue) => {}
            Ok(UciResult::Quit) => {
                is_quit = true;
            }
            Ok(UciResult::BestMove) => {
                if let Some(best_move) = game_manager_actor
                    .send(game::game_manager::GetBestMove)
                    .await
                    .unwrap()
                {
                    _ = best_move_action(stdout, best_move);
                }
            }
            Err(err) => {
                let error_as_str = format!("{:?}{}", err.event(), err.error());
                if show_errors {
                    _ = write_err(stdout, error_as_str.clone())
                }
                errors.push(error_as_str);
            }
        }
    }
    if errors.is_empty() {
        if is_quit {
            game_manager_actor
                .send(game_manager::UciCommand::CleanResources)
                .await
                .expect("Actix error")
                .unwrap(); // always Ok
        }
        Ok(is_quit)
    } else {
        Err(errors)
    }
}

fn write_err(stdout: &mut Stdout, err: String) -> Result<(), io::Error> {
    let res = writeln!(stdout, "{}", err);
    stdout.flush().unwrap();
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{
        fen::{self, EncodeUserInput},
        square,
    };
    use actix::Actor;
    use game::{game_manager, game_state, parameters, player};
    use parser::InputParser;

    async fn init<T: engine::EngineActor>(
        input: &str,
    ) -> (
        game_manager::GameManagerActor<engine::EngineDummy>,
        command::Command,
    ) {
        let game_manager_actor = game_manager::GameManager::<engine::EngineDummy>::start(
            game_manager::GameManager::new(),
        );
        let parser = InputParser::new(&input);
        let command = parser.parse_input().expect("Invalid command");
        (game_manager_actor, command)
    }
    async fn get_game_state<T: engine::EngineActor>(
        game_manager_actor: &game_manager::GameManagerActor<T>,
    ) -> Option<game_state::GameState> {
        let result = game_manager_actor
            .send(game_manager::GetGameState)
            .await
            .unwrap();
        result
    }

    #[actix::test]
    async fn test_uci_input_start_pos() {
        let input = "position startpos";
        let mut stdout = io::stdout();
        let (game_manager_actor, command) = init::<engine::EngineDummy>(input).await;
        let is_quit = execute_command(&game_manager_actor, command, &mut stdout, true)
            .await
            .unwrap();
        assert!(!is_quit);
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        assert_eq!(fen, fen::FEN_START_POSITION);
    }

    #[actix::test]
    async fn test_uci_input_start_pos_with_moves() {
        let input = "position startpos moves e2e4 e7e5 g1f3";
        let mut stdout = io::stdout();
        let (game_manager_actor, command) = init::<engine::EngineDummy>(input).await;
        let is_quit = execute_command(&game_manager_actor, command, &mut stdout, true)
            .await
            .unwrap();
        assert!(!is_quit);
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen_str = "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        assert_eq!(fen, fen_str);
    }

    #[actix::test]
    async fn test_uci_input_fen_pos() {
        let input = format!("position fen {}", fen::FEN_START_POSITION);
        let mut stdout = io::stdout();
        let (game_manager_actor, command) = init::<engine::EngineDummy>(&input).await;
        let is_quit = execute_command(&game_manager_actor, command, &mut stdout, true)
            .await
            .unwrap();
        assert!(!is_quit);
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        assert_eq!(fen, fen::FEN_START_POSITION);
    }
    #[actix::test]
    async fn test_uci_input_fen_pos_with_moves() {
        let input = format!(
            "position fen {} moves e2e4 e7e5 g1f3",
            fen::FEN_START_POSITION
        );
        let mut stdout = io::stdout();
        let (game_manager_actor, command) = init::<engine::EngineDummy>(&input).await;
        let is_quit = execute_command(&game_manager_actor, command, &mut stdout, true)
            .await
            .unwrap();
        assert!(!is_quit);
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen_str = "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        assert_eq!(fen, fen_str);
    }
    #[actix::test]
    async fn test_uci_input_fen_pos_with_moves_invalid() {
        let input = format!(
            "position fen {} moves e2e4 e7e5 g1f4",
            fen::FEN_START_POSITION
        );
        let mut stdout = io::stdout();
        let (game_manager_actor, command) = init::<engine::EngineDummy>(&input).await;
        let r = execute_command(&game_manager_actor, command, &mut stdout, true).await;
        assert!(r.is_err())
    }
    #[actix::test]
    async fn test_uci_input_default_parameters() {
        let input = "position startpos";
        let mut stdout = io::stdout();
        let (game_manager_actor, command) = init::<engine::EngineDummy>(&input).await;
        let is_quit = execute_command(&game_manager_actor, command, &mut stdout, true)
            .await
            .unwrap();
        assert!(!is_quit);
        let parameters = game_manager_actor
            .send(game_manager::GetParameters)
            .await
            .expect("mailbox error")
            .unwrap();
        let expected = parameters::Parameters::default();
        assert_eq!(parameters, expected)
    }

    #[actix::test]
    async fn test_uci_input_modified_parameters() {
        let inputs = vec![
            "position startpos",
            "go depth 3 movetime 5000 wtime 3600000 btime 3600001",
            "quit",
        ];
        let uci_reader = UciReadVecStringWrapper::new(inputs.as_slice());
        let mut game_manager = game_manager::GameManager::<engine::EngineDummy>::new();
        let engine_player1 = engine::EngineDummy::default().start();
        let engine_player2 = engine::EngineDummy::default().start();
        let player1 = player::Player::Human {
            engine_opt: Some(engine_player1),
        };
        let player2 = player::Player::Computer {
            engine: engine_player2,
        };
        let players = player::Players::new(player1, player2);
        game_manager.set_players(players);
        let game_manager_actor = game_manager::GameManager::start(game_manager);
        // set clocks before executing UCI commands
        let white_clock_actor =
            game::chessclock::Clock::new("white", 3, 0, game_manager_actor.clone()).start();
        let black_clock_actor =
            game::chessclock::Clock::new("black", 3, 0, game_manager_actor.clone()).start();
        game_manager_actor.do_send(game_manager::SetClocks::new(
            Some(white_clock_actor),
            Some(black_clock_actor),
        ));
        // execute UCI commands
        uci_loop(uci_reader, &game_manager_actor).await.unwrap();
        let parameters = game_manager_actor
            .send(game_manager::GetParameters)
            .await
            .expect("Actix error")
            .unwrap();
        let expected = parameters::Parameters::new(
            Some(3),
            Some(5000),
            Some(3600000),
            Some(3600001),
            None,
            None,
            vec![],
        );
        assert_eq!(parameters, expected);
        // check wtime and btime
        let remaining_time_white = game_manager_actor
            .send(game_manager::GetClockRemainingTime::new(
                square::Color::White,
            ))
            .await
            .expect("actor error")
            .expect("Missing data");
        let remaining_time_black = game_manager_actor
            .send(game_manager::GetClockRemainingTime::new(
                square::Color::Black,
            ))
            .await
            .expect("actor error")
            .expect("Missing data");
        assert_eq!(remaining_time_white, 3600000);
        assert_eq!(remaining_time_black, 3600001);
    }

    #[actix::test]
    async fn test_uci_start_stop_think_engine() {
        let inputs = vec!["position startpos", "go"];
        let uci_reader = UciReadVecStringWrapper::new(inputs.as_slice());
        let mut game_manager = game_manager::GameManager::<engine::EngineDummy>::new();
        let engine_player1_actor = engine::EngineDummy::default().start();
        let engine_player2_actor = engine::EngineDummy::default().start();
        let player1 = player::Player::Human {
            engine_opt: Some(engine_player1_actor.clone()),
        };
        let player2 = player::Player::Computer {
            engine: engine_player2_actor,
        };
        let players = player::Players::new(player1, player2);
        game_manager.set_players(players);
        let game_manager_actor = game_manager::GameManager::start(game_manager);
        // execute UCI commands
        uci_loop(uci_reader, &game_manager_actor).await.unwrap();
        let engine_status = engine_player1_actor
            .send(engine::EngineGetStatus)
            .await
            .expect("Actix error")
            .unwrap();
        let engine_is_thinking = true;
        let engine_is_running = true;
        let expected = engine::EngineStatus::new(engine_is_thinking, engine_is_running);
        let _ = game_manager_actor
            .send(game_manager::UciCommand::CleanResources)
            .await
            .unwrap();
        assert_eq!(engine_status, expected)
    }
}
