pub mod command;
pub mod event;
pub mod notation;
use crate::game::{self, configuration, engine};
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
    let res = writeln!(stdout, "{}", best_move.cast());
    stdout.flush().unwrap();
    res
}

pub async fn uci_loop<T: UciRead, E: engine::EngineActor>(
    mut uci_reader: T,
    game_actor: &game::GameActor<E>,
) -> Result<(), Vec<String>> {
    let mut stdout = io::stdout();
    let mut errors: Vec<String> = vec![];

    loop {
        let input = uci_reader.uci_read();
        let parser = parser::InputParser::new(&input);
        let command = parser.parse_input().expect("Invalid command");
        let r = execute_command(game_actor, command, &mut stdout, true).await;
        if let Some(error) = r.clone().err() {
            errors.extend(error);
        } else if r.unwrap() {
            break;
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub trait UciRead {
    fn uci_read(&mut self) -> String;
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
    fn uci_read(&mut self) -> String {
        let mut input = String::new();
        self.stdin
            .lock()
            .read_line(&mut input)
            .expect("Failed to read line");
        input.trim().to_string()
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
    fn uci_read(&mut self) -> String {
        if self.idx < self.inputs.len() {
            let result = self.inputs[self.idx];
            self.idx += 1;
            result.to_string()
        } else {
            "quit".to_string()
        }
    }
}

pub async fn execute_command<T: engine::EngineActor>(
    game_actor: &game::GameActor<T>,
    command: command::Command,
    stdout: &mut Stdout,
    show_errors: bool,
) -> Result<bool, Vec<String>> {
    let mut is_quit = false;
    let mut errors: Vec<String> = vec![];
    let events = command.handle_command();
    for event in &events {
        // pdate the configuration
        let uci_result = event.handle_event(game_actor, stdout).await;
        // quit, stop and show best move or continue
        match uci_result {
            Ok(UciResult::Continue) => {}
            Ok(UciResult::Quit) => {
                is_quit = true;
            }
            Ok(UciResult::BestMove) => {
                if let Some(best_move) = game_actor.send(game::GetBestMove).await.unwrap() {
                    _ = best_move_action(stdout, best_move);
                }
            }
            Err(err) => {
                let error_as_str = format!("{:?}{}", err.event(), err.error());
                if show_errors {
                    _ = configuration::write_err(stdout, error_as_str.clone())
                }
                errors.push(error_as_str);
            }
        }
    }
    if errors.is_empty() {
        Ok(is_quit)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::fen::{self, EncodeUserInput};
    use actix::Actor;
    use game::{parameters, player, Game, GameActor};
    use parser::InputParser;

    async fn init<T: engine::EngineActor>(input: &str) -> (GameActor<T>, command::Command) {
        let game_actor = Game::start(Game::new());
        let parser = InputParser::new(&input);
        let command = parser.parse_input().expect("Invalid command");
        (game_actor, command)
    }
    async fn get_configuration<T: engine::EngineActor>(
        game_actor: &GameActor<T>,
    ) -> configuration::Configuration {
        let result = game_actor.send(game::GetConfiguration).await.unwrap();
        result.unwrap()
    }

    #[actix::test]
    async fn test_uci_input_start_pos() {
        let input = "position startpos";
        let mut stdout = io::stdout();
        let (game_actor, command) = init::<engine::EngineDummy>(input).await;
        let is_quit = execute_command(&game_actor, command, &mut stdout, true)
            .await
            .unwrap();
        assert!(!is_quit);
        let configuration = get_configuration(&game_actor).await;
        assert!(configuration.opt_position().is_some());
        let fen = fen::Fen::encode(&configuration.opt_position().unwrap())
            .expect("Failed to encode position");
        assert_eq!(fen, fen::FEN_START_POSITION);
    }

    #[actix::test]
    async fn test_uci_input_start_pos_with_moves() {
        let input = "position startpos moves e2e4 e7e5 g1f3";
        let mut stdout = io::stdout();
        let (game_actor, command) = init::<engine::EngineDummy>(input).await;
        let is_quit = execute_command(&game_actor, command, &mut stdout, true)
            .await
            .unwrap();
        assert!(!is_quit);
        let configuration = get_configuration(&game_actor).await;
        assert!(configuration.opt_position().is_some());
        let fen_str = "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let fen = fen::Fen::encode(&configuration.opt_position().unwrap())
            .expect("Failed to encode position");
        assert_eq!(fen, fen_str);
    }

    #[actix::test]
    async fn test_uci_input_fen_pos() {
        let input = format!("position fen {}", fen::FEN_START_POSITION);
        let mut stdout = io::stdout();
        let (game_actor, command) = init::<engine::EngineDummy>(&input).await;
        let is_quit = execute_command(&game_actor, command, &mut stdout, true)
            .await
            .unwrap();
        assert!(!is_quit);
        let configuration = get_configuration(&game_actor).await;
        assert!(configuration.opt_position().is_some());
        let fen = fen::Fen::encode(&configuration.opt_position().unwrap())
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
        let (game_actor, command) = init::<engine::EngineDummy>(&input).await;
        let is_quit = execute_command(&game_actor, command, &mut stdout, true)
            .await
            .unwrap();
        assert!(!is_quit);
        let configuration = get_configuration(&game_actor).await;
        assert!(configuration.opt_position().is_some());
        let fen_str = "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let fen = fen::Fen::encode(&configuration.opt_position().unwrap())
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
        let (game_actor, command) = init::<engine::EngineDummy>(&input).await;
        let r = execute_command(&game_actor, command, &mut stdout, true).await;
        assert!(r.is_err())
    }
    #[actix::test]
    async fn test_uci_input_default_parameters() {
        let input = "position startpos";
        let mut stdout = io::stdout();
        let (game_actor, command) = init::<engine::EngineDummy>(&input).await;
        let is_quit = execute_command(&game_actor, command, &mut stdout, true)
            .await
            .unwrap();
        assert!(!is_quit);
        let configuration = get_configuration(&game_actor).await;
        assert!(configuration.opt_position().is_some());
        let parameters = configuration.parameters();
        let expected = parameters::Parameters::default();
        assert_eq!(*parameters, expected)
    }
    #[actix::test]
    async fn test_uci_input_modified_parameters() {
        let inputs = vec![
            "position startpos",
            "go depth 3 movetime 5000 wtime 3600000 btime 3600001",
        ];
        let uci_reader = UciReadVecStringWrapper::new(inputs.as_slice());
        let mut game = Game::<engine::EngineDummy>::new();
        let engine_player1 = engine::EngineDummy::new();
        let engine_player2 = engine::EngineDummy::new();
        let player1 = player::Player::Human(Some(Box::new(engine_player1)));
        let player2 = player::Player::Computer(Box::new(engine_player2));
        let players = player::Players::new(player1, player2);
        game.set_players(players);
        let game_actor = Game::start(game);
        uci_loop(uci_reader, &game_actor).await.unwrap();
        let configuration = get_configuration(&game_actor).await;
        assert!(configuration.opt_position().is_some());
        let expected =
            parameters::Parameters::new(Some(3), Some(5000), Some(3600000), Some(3600001), vec![]);
        assert_eq!(*configuration.parameters(), expected)
    }
}
