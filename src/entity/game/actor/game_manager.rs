pub mod handler_clock;
pub mod handler_engine;
pub mod handler_game;
pub mod handler_uci_command;

use actix::{Actor, Addr, Context};

use crate::entity::game::component::{
    bitboard::{self, zobrist},
    square,
};
use crate::ui::notation::fen;
use crate::ui::notation::long_notation::{self, LongAlgebricNotationMove};

use crate::entity::clock::actor::chessclock;
use crate::entity::engine::component::engine_logic as logic;
use crate::entity::game::component::{game_state::GameState, parameters, player};
use crate::monitoring::debug;

pub type GameManagerActor = Addr<GameManager>;

#[derive(Debug, Default, Clone)]
pub struct History {
    fen: String,
    moves: Vec<bitboard::BitBoardMove>,
}
impl History {
    pub fn init(&mut self) {
        self.set_fen(fen::FEN_START_POSITION);
    }
    pub fn set_fen(&mut self, fen: &str) {
        self.fen = fen.to_string();
        self.moves = vec![];
    }
    pub fn add_moves(&mut self, m: bitboard::BitBoardMove) {
        self.moves.push(m);
    }
}

#[derive(Debug, Clone)]
pub struct TimestampedBestMove {
    best_move: long_notation::LongAlgebricNotationMove,
    timestamp: chrono::DateTime<chrono::Utc>, // date of best_move initialization
    engine_id: logic::EngineId,               // which engine has found the best move
}
impl TimestampedBestMove {
    fn build(
        best_move: long_notation::LongAlgebricNotationMove,
        timestamp: chrono::DateTime<chrono::Utc>,
        engine_id: logic::EngineId,
    ) -> Self {
        Self {
            best_move,
            timestamp,
            engine_id,
        }
    }
    pub fn best_move(&self) -> long_notation::LongAlgebricNotationMove {
        self.best_move
    }
    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }
    pub fn origin(&self) -> logic::EngineId {
        self.engine_id.clone()
    }
    fn is_more_recent_best_move_than(&self, timestamped_best_move: &TimestampedBestMove) -> bool {
        self.timestamp > timestamped_best_move.timestamp
    }
}

#[derive(Default)]
pub struct GameManager {
    game_state_opt: Option<GameState>,
    debug_actor_opt: Option<debug::DebugActor>,
    ts_best_move_opt: Option<TimestampedBestMove>,
    history: History,
    parameters: parameters::Parameters,
    players: player::Players,
    white_clock_actor_opt: Option<chessclock::ClockActor>,
    black_clock_actor_opt: Option<chessclock::ClockActor>,
    zobrist_table: zobrist::Zobrist,
}

impl GameManager {
    pub fn new(debug_actor_opt: Option<debug::DebugActor>) -> Self {
        let mut game_manager = GameManager::default();
        game_manager.debug_actor_opt = debug_actor_opt;
        game_manager.zobrist_table = zobrist::Zobrist::new();
        game_manager
    }
    pub fn game_state(&self) -> Option<&GameState> {
        self.game_state_opt.as_ref()
    }
    pub fn history(&self) -> &History {
        &self.history
    }
}

impl Actor for GameManager {
    type Context = Context<Self>;
}

impl GameManager {
    fn play_moves(&mut self, valid_moves: Vec<LongAlgebricNotationMove>) -> Result<(), String> {
        let result: Option<Result<Vec<bitboard::BitBoardMove>, String>> = self
            .game_state_opt
            .as_mut()
            .map(|game_state: &mut GameState| {
                game_state.play_moves(
                    valid_moves,
                    &self.zobrist_table,
                    self.debug_actor_opt.clone(),
                )
            });
        match result {
            Some(Ok(b_moves)) => {
                let mut n_moves_white = 0u64;
                let mut n_moves_black = 0u64;
                for b_move in b_moves {
                    self.history.add_moves(b_move);
                    if b_move.color() == square::Color::White {
                        n_moves_white += 1;
                    } else {
                        n_moves_black += 1;
                    }
                }
                if let Some(white_clock_actor) = &self.white_clock_actor_opt {
                    async_clock_inc(
                        "white".to_string(),
                        n_moves_white,
                        white_clock_actor.clone(),
                    );
                }
                if let Some(black_clock_actor) = &self.black_clock_actor_opt {
                    async_clock_inc(
                        "black".to_string(),
                        n_moves_black,
                        black_clock_actor.clone(),
                    );
                }
                Ok(())
            }
            Some(Err(err)) => Err(err), // illegal move
            None => Err("moves ignored since no position has been defined".to_string()),
        }
    }
    pub fn set_players(&mut self, players: player::Players) {
        self.players = players;
    }
}

fn async_clock_inc(debug: String, n_moves: u64, clock_actor: Addr<chessclock::Clock>) {
    use tokio::task;

    // Offload the sending to a background task
    task::spawn(async move {
        let result = clock_actor
            .send(chessclock::handler_clock::IncRemainingTime(n_moves))
            .await;
        match result {
            Ok(response) => println!(
                "Time for {} incremented successfully: {:?}",
                debug, response
            ),
            Err(e) => println!("Error incrementing time: {:?}", e),
        }
    });
}

#[cfg(test)]
use crate::entity::engine::component::engine_dummy as dummy;

#[cfg(test)]
pub async fn build_game_manager_actor(inputs: Vec<&str>) -> GameManagerActor {
    use crate::entity::{engine::actor::engine_dispatcher as dispatcher, uci::actor::uci_entity};
    use std::sync::Arc;

    let debug_actor_opt: Option<debug::DebugActor> = None;
    //let debug_actor_opt: Option<debug::DebugActor> = Some(debug::DebugEntity::new(true).start());
    let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
    let mut game = GameManager::new(debug_actor_opt.clone());
    let engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
    let engine_player1_dispatcher =
        dispatcher::EngineDispatcher::new(Arc::new(engine_player1), debug_actor_opt.clone());
    let engine_player2 = dummy::EngineDummy::new(debug_actor_opt.clone());
    let engine_player2_dispatcher =
        dispatcher::EngineDispatcher::new(Arc::new(engine_player2), debug_actor_opt.clone());
    let player1 = player::Player::Human {
        engine_opt: Some(engine_player1_dispatcher.start()),
    };
    let player2 = player::Player::Computer {
        engine: engine_player2_dispatcher.start(),
    };
    let players = player::Players::new(player1, player2);
    game.set_players(players);
    let game_manager_actor = GameManager::start(game);
    // set the position from uci command
    let uci_entity = uci_entity::UciEntity::new(
        uci_reader,
        game_manager_actor.clone(),
        debug_actor_opt.clone(),
    );
    let uci_entity_actor = uci_entity.start();
    for _i in 0..inputs.len() {
        let _r = uci_entity_actor
            .send(uci_entity::handler_read::ReadUserInput)
            .await;
    }
    actix::clock::sleep(std::time::Duration::from_millis(100)).await;
    // define clocks
    let white_clock_actor =
        chessclock::Clock::new("white", 3, 0, game_manager_actor.clone()).start();
    let black_clock_actor =
        chessclock::Clock::new("black", 3, 0, game_manager_actor.clone()).start();
    game_manager_actor.do_send(handler_clock::SetClocks::new(
        Some(white_clock_actor),
        Some(black_clock_actor),
    ));
    // send clock to game
    let set_clock_msg = handler_clock::SetClockRemainingTime::new(&square::Color::White, 2);
    game_manager_actor.do_send(set_clock_msg);
    actix::clock::sleep(std::time::Duration::from_millis(100)).await;
    game_manager_actor
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use actix::Actor;

    use super::GameManager;
    use crate::entity::clock::actor::chessclock;
    use crate::entity::engine::actor::engine_dispatcher as dispatcher;
    use crate::entity::engine::component::engine_dummy as dummy;
    use crate::entity::game::actor::game_manager;
    use crate::entity::game::component::{game_state, player, square};
    use crate::entity::uci::actor::uci_entity;
    use crate::monitoring::debug;
    use crate::ui::notation::fen::{self, EncodeUserInput};

    // FIXME: redudant with uci.rs test
    async fn get_game_state(
        game_manager_actor: &game_manager::GameManagerActor,
    ) -> Option<game_state::GameState> {
        let result_or_error = game_manager_actor
            .send(game_manager::handler_game::GetGameState)
            .await;
        result_or_error.unwrap()
    }

    #[actix::test]
    async fn test_game_capture_en_passant_valid() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 d7d5 e4d5 e7e5 d5e6"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        let fen_expected = "rnbqkbnr/ppp2ppp/4P3/8/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 3";
        assert_eq!(fen, fen_expected);
    }
    #[actix::test]
    async fn test_game_pawn_move_invalid() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 e7e5 e4e5"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        let fen_expected = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        assert_eq!(fen, fen_expected);
    }

    #[actix::test]
    async fn test_game_mat() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position startpos moves e2e4 e7e5 f1c4 a7a6 d1f3 a6a5 f3f7"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _r = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let end_game = game_manager_actor
            .send(game_manager::handler_game::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Mat(square::Color::Black))
    }
    #[actix::test]
    async fn test_game_pat_white_first() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position fen k7/7R/1R6/8/8/8/8/7K w - - 0 1 moves h1g1"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _r = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let end_game = game_manager_actor
            .send(game_manager::handler_game::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Pat)
    }
    #[actix::test]
    async fn test_game_pat_black_first() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position fen k7/7R/1R6/8/8/8/8/7K b - - 0 1"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = game_manager::GameManager::start(game_manager::GameManager::new(
            debug_actor_opt.clone(),
        ));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _r = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let end_game = game_manager_actor
            .send(game_manager::handler_game::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Pat)
    }
    #[actix::test]
    async fn test_game_weird() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position startpos moves d2d4 d7d5 b1c3 a7a6 c1f4 a6a5 d1d2 a5a4 e1c1 a4a3 h2h3 a3b2 c1b1 a8a2 h3h4 a2a1 b1b2"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        let mut is_error = false;
        for _i in 0..inputs.len() {
            let r = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
            if r.is_err() {
                is_error = true;
            }
        }
        assert!(!is_error)
    }
    #[actix::test]
    async fn test_game_blocked_pawn_ckeck_invalid() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 e7e5 a2a3 d8h4 f2f3"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        let fen_expected = "rnb1kbnr/pppp1ppp/8/4p3/4P2q/P7/1PPP1PPP/RNBQKBNR w KQkq - 1 3";
        assert_eq!(fen, fen_expected);
    }
    #[actix::test]
    async fn test_game_block_ckeck() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some( debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 d7d5 e4d5 d8d5 a2a3 d5e5 d1f3"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = game_manager::GameManager::start(game_manager::GameManager::new(
            debug_actor_opt.clone(),
        ));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        let fen_expected = "rnb1kbnr/ppp1pppp/8/4q3/8/P7/1PPP1PPP/RNBQKBNR w KQkq - 1 4";
        assert_eq!(fen, fen_expected);
    }
    #[actix::test]
    async fn test_game_block_ckeck2() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position startpos moves e2e4 d7d5 e4d5 d8d5 a2a3 d5e5 d1e2"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        let mut is_error = false;
        for _i in 0..inputs.len() {
            let r = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
            if r.is_err() {
                is_error = true;
            }
        }
        assert!(!is_error)
    }
    #[actix::test]
    async fn test_game_escape() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 c7c5 f1c4 d7d6 d1h5 a7a6 h5f7 e8d7"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        let fen_expected = "rnbq1bnr/1p1kpQpp/p2p4/2p5/2B1P3/8/PPPP1PPP/RNB1K1NR w KQ - 1 5";
        assert_eq!(fen, fen_expected);
    }
    #[actix::test]
    async fn test_game_king_close_to_king() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let fen_initial = "r7/8/8/4k3/8/4K3/8/7R w - - 0 1";
        let position = format!("position fen {} moves e3e4", fen_initial);
        let inputs = vec![position.as_str()];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        assert_eq!(fen, fen_initial);
    }
    #[actix::test]
    async fn test_game_rule_insufficient_material() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position fen k7/8/8/8/8/8/8/7K b - - 0 1"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let end_game = game_manager_actor
            .send(game_manager::handler_game::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::InsufficientMaterial)
    }
    #[actix::test]
    async fn test_game_rule_50xmoves() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        // f5e5 -> forbidden => review attackers() ?
        let seq1x10 = "h1g1 a8b8 g1f1 b8c8 f1e1 c8d8 e1d1 d8e8 d1c1 e8f8 c1b1 f8g8 b1b2 g8g7 b2c2 g7f7 c2d2 f7e7 d2e2 e7d7";
        let seq2x10 = "e2f2 d7c7 f2g2 c7b7 g2h2 b7a7 h3h4 a6a5 h2h3 a7a6 h3g3 a6b6 g3f3 b6c6 f3e3 c6d6 e3d3 d6e6 d3c3 e6f6";
        let seq3x10 = "c3b3 f6g6 b3b4 g6g5 b4c4 g5f5 c4d4 f5e6 d4e4 e6d6 e4d4 d6c6 d4c4 c6b6 c4b4 b6a6 b4b3 a6a7 b3b2 a7b7";
        let seq4x10 = "b2c2 b7c7 c2d2 c7d7 d2e2 d7e7 e2f2 e7f7 f2g2 f7g7 g2f2 g7g8 f2e2 g8f8 e2d2 f8e8 d2c2 e8d8 c2b2 d8c8";
        let seq5x10 = "b2b1 c8b8 b1c1 b8a8 c1d1 a8a7 d1e1 a7b7 e1f1 b7c7 f1g1 c7d7 g1h1 d7e7 h1h2 e7e6 h2g2 e6e5 g2f2 e5f5";
        let movesx50 = format!(
            "{} {} {} {} {}",
            seq1x10, seq2x10, seq3x10, seq4x10, seq5x10
        );
        let fen = format!(
            "position fen k7/8/r7/8/8/7R/8/7K w - - 0 1 moves {}",
            movesx50
        );
        let inputs = vec![fen.as_str()];
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let end_game = game_manager_actor
            .send(game_manager::handler_game::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::NoPawnAndCapturex50)
    }

    #[actix::test]
    async fn test_game_3x_position() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let moves = "h1g1 a8b8 g1h1 b8a8 h1g1 a8b8 g1h1 b8a8";
        let fen = format!("position fen k7/8/r7/8/8/7R/8/7K w - - 0 1 moves {}", moves);
        let inputs = vec![fen.as_str()];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let end_game = game_manager_actor
            .send(game_manager::handler_game::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Repetition3x)
    }
    #[actix::test]
    async fn test_game_3x_position_with_pawn() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let moves = "h7h6 h1g1 a8b8 g1h1 b8a8 h1g1 a8b8 g1h1 b8a8";
        let fen = format!(
            "position fen k7/7p/r7/8/8/7R/8/7K b - - 0 1 moves {}",
            moves
        );
        let inputs = vec![fen.as_str()];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        let end_game = game_manager_actor
            .send(game_manager::handler_game::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Repetition3x)
    }

    #[actix::test]
    async fn test_game_timeout_gameover() {
        let inputs = vec!["position startpos"];
        let game_manager_actor = game_manager::build_game_manager_actor(inputs).await;
        game_manager_actor.do_send(game_manager::handler_clock::StartOrSwitchClocks);
        actix::clock::sleep(Duration::from_secs(3)).await;
        // Introduce a delay to ensure the TimeOut message is processed
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let end_game = game_manager_actor
            .send(game_manager::handler_game::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            end_game,
            game_state::EndGame::TimeOutLost(square::Color::White)
        )
    }
    #[actix::test]
    async fn test_game_inc_timer() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position startpos moves e2e4 e7e5 g1f3 g8f6 f1c4"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let mut game_manager = super::GameManager::new(debug_actor_opt.clone());
        let engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let engine_player1_dispatcher =
            dispatcher::EngineDispatcher::new(Arc::new(engine_player1), debug_actor_opt.clone());
        let engine_player2 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let engine_player2_dispatcher =
            dispatcher::EngineDispatcher::new(Arc::new(engine_player2), debug_actor_opt.clone());
        let player1 = player::Player::Human {
            engine_opt: Some(engine_player1_dispatcher.start()),
        };
        let player2 = player::Player::Computer {
            engine: engine_player2_dispatcher.start(),
        };
        let players = player::Players::new(player1, player2);
        game_manager.set_players(players);
        let game_manager_actor = game_manager::GameManager::start(game_manager);
        // set the position from uci command
        let white_clock_actor =
            chessclock::Clock::new("white", 3, 1, game_manager_actor.clone()).start();
        let black_clock_actor =
            chessclock::Clock::new("black", 4, 2, game_manager_actor.clone()).start();
        game_manager_actor.do_send(game_manager::handler_clock::SetClocks::new(
            Some(white_clock_actor),
            Some(black_clock_actor),
        ));
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
        }
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let clock_white = game_manager_actor
            .send(game_manager::handler_clock::GetClockRemainingTime::new(
                square::Color::White,
            ))
            .await
            .unwrap()
            .unwrap();
        let clock_black = game_manager_actor
            .send(game_manager::handler_clock::GetClockRemainingTime::new(
                square::Color::Black,
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(clock_white, 6);
        assert_eq!(clock_black, 8);
    }
    #[actix::test]
    async fn test_game_timeout_no_material_gameover() {
        let inputs = vec!["position fen k7/7p/8/8/8/8/8/7K w - - 0 1"];
        let game_manager_actor = game_manager::build_game_manager_actor(inputs).await;
        game_manager_actor.do_send(game_manager::handler_clock::StartOrSwitchClocks);
        actix::clock::sleep(Duration::from_secs(3)).await;
        // Introduce a delay to ensure the TimeOut message is processed
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let end_game = game_manager_actor
            .send(game_manager::handler_game::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            end_game,
            game_state::EndGame::TimeOutLost(square::Color::White)
        )
    }
    #[actix::test]
    async fn test_game_opponent_timeout_no_material_draw() {
        let inputs = vec!["position fen k7/7p/8/8/8/8/8/7K b - - 0 1"];
        let game_manager_actor = game_manager::build_game_manager_actor(inputs).await;
        game_manager_actor.do_send(game_manager::handler_clock::StartOrSwitchClocks);
        actix::clock::sleep(Duration::from_secs(4)).await;
        // Introduce a delay to ensure the TimeOut message is processed
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let end_game = game_manager_actor
            .send(game_manager::handler_game::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::TimeOutDraw)
    }
}
