use actix::Addr;

use super::engine_logic::{self as logic, Engine};
use super::{score, stat_eval};
use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::game::component::bitboard::piece_move::GenMoves;
use crate::entity::game::component::bitboard::zobrist;
use crate::entity::game::component::game_state;
use crate::entity::stat::actor::stat_entity;
use crate::entity::stat::component::stat_data;
use crate::ui::notation::long_notation;
use crate::{entity::game::component::bitboard, monitoring::debug};

#[derive(Debug)]
pub struct EngineMat {
    id_number: String,
    debug_actor_opt: Option<debug::DebugActor>,
    zobrist_table: zobrist::Zobrist,
    max_depth: u8,
}
impl EngineMat {
    pub fn new(
        debug_actor_opt: Option<debug::DebugActor>,
        zobrist_table: zobrist::Zobrist,
        max_depth: u8,
    ) -> Self {
        assert!(max_depth >= 1);
        Self {
            id_number: "".to_string(),
            debug_actor_opt,
            zobrist_table,
            max_depth: max_depth * 2 - 1,
        }
    }
    pub fn set_id_number(&mut self, id_number: &str) {
        self.id_number = id_number.to_string();
    }

    pub fn mat_solver_init(
        &self,
        game: &game_state::GameState,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        max_depth: u8,
        stat_eval: &mut stat_eval::StatEval,
    ) -> Option<score::BitboardMoveScoreMat> {
        let mut game_clone = game.clone();
        let shortest_mat_opt = self.mat_solver(
            "",
            &mut game_clone,
            0,
            true,
            self_actor.clone(),
            stat_actor_opt.clone(),
            stat_eval,
            max_depth,
        );
        // if let Some(mat_move) = &shortest_mat_opt {
        //     println!("============");
        //     println!("{}", mat_move.variant());
        //     println!("============");
        // }
        shortest_mat_opt
    }

    fn mat_solver(
        &self,
        variant: &str,
        game: &mut game_state::GameState,
        current_depth: u8,
        is_attacker: bool,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        stat_eval: &mut stat_eval::StatEval,
        max_depth: u8,
    ) -> Option<score::BitboardMoveScoreMat> {
        let mut game_clone = game.clone();
        let moves = if is_attacker {
            self.filter_move_with_check(&mut game_clone, &game.gen_moves(), stat_eval)
        } else {
            game.gen_moves()
        };
        if moves.is_empty() {
            None
        } else {
            let mut shortest_mat_opt: Option<score::BitboardMoveScoreMat> = None;
            for m in moves {
                let move_mat_opt = self.process_move(
                    game,
                    m,
                    is_attacker,
                    variant,
                    self_actor.clone(),
                    stat_actor_opt.clone(),
                    stat_eval,
                    current_depth,
                    max_depth,
                );
                match (move_mat_opt, &shortest_mat_opt) {
                    (Some(move_mat), Some(shortest_mat))
                        // maximize
                        if is_attacker && shortest_mat.mat_in() > move_mat.mat_in() =>
                    {
                        let m_mat = score::BitboardMoveScoreMat::new(m, move_mat.mat_in(), &move_mat.variant());
                        shortest_mat_opt = Some(m_mat);
                    }
                    (Some(move_mat), Some(shortest_mat))
                        // minimize
                        if !is_attacker && shortest_mat.mat_in() < move_mat.mat_in() =>
                    {
                        let m_mat = score::BitboardMoveScoreMat::new(m, move_mat.mat_in(), &move_mat.variant());
                        shortest_mat_opt = Some(m_mat);
                    }
                    (Some(move_mat), None) => {
                        let m_mat = score::BitboardMoveScoreMat::new(m, move_mat.mat_in(), &move_mat.variant());
                        shortest_mat_opt = Some(m_mat);
                    }
                    (None, Some(_)) => {
                        if !is_attacker {
                            shortest_mat_opt = None;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            shortest_mat_opt
        }
    }

    fn filter_move_with_check(
        &self,
        game: &game_state::GameState,
        moves: &[bitboard::BitBoardMove],
        stat_eval: &mut stat_eval::StatEval,
    ) -> Vec<bitboard::BitBoardMove> {
        let mut v = vec![];
        let mut game_clone = game.clone();
        for m in moves {
            let long_algebraic_move =
                long_notation::LongAlgebricNotationMove::build_from_b_move(*m);
            game_clone
                .play_moves(&[long_algebraic_move], &self.zobrist_table, None, false)
                .unwrap();
            if game_clone.check_status().is_check() {
                v.push(m.clone());
            }
            game_clone.play_back();
        }
        stat_eval.inc_n_check(v.len() as u64);
        v
    }
    fn update_stat(
        &self,
        stat_eval: &mut stat_eval::StatEval,
        stat_actor_opt: Option<&stat_entity::StatActor>,
    ) {
        if stat_eval.inc_n_positions_evaluated() % stat_data::SEND_STAT_EVERY_N_POSITION_EVALUATED
            == 0
        {
            if let Some(stat_actor) = stat_actor_opt {
                let msg = stat_entity::handler_stat::StatUpdate::new(
                    self.id(),
                    stat_eval.n_positions_evaluated(),
                );
                stat_actor.do_send(msg);
            }
            stat_eval.reset_n_positions_evaluated();
        }
    }
    // return None if mat failed, or Some(current_depth) if success
    fn process_move(
        &self,
        game: &mut game_state::GameState,
        m: bitboard::BitBoardMove,
        is_attacker: bool,
        variant: &str,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        stat_eval: &mut stat_eval::StatEval,
        current_depth: u8,
        max_depth: u8,
    ) -> Option<score::BitboardMoveScoreMat> {
        let long_algebraic_move = long_notation::LongAlgebricNotationMove::build_from_b_move(m);
        let updated_variant = format!("{} {}", variant, long_algebraic_move.cast())
            .trim()
            .to_string();
        game.play_moves(&[long_algebraic_move], &self.zobrist_table, None, false)
            .unwrap();
        game.update_endgame_status();

        let result = match &game.end_game() {
            game_state::EndGame::Mat(_lost_color) if !is_attacker => {
                self.update_stat(stat_eval, stat_actor_opt.as_ref());
                None
            }
            game_state::EndGame::Mat(_) => {
                self.update_stat(stat_eval, stat_actor_opt.as_ref());
                Some(score::BitboardMoveScoreMat::new(
                    m,
                    current_depth + 1,
                    &updated_variant,
                ))
            }
            game_state::EndGame::None => {
                if current_depth < self.max_depth {
                    self.mat_solver(
                        &updated_variant,
                        game,
                        current_depth + 1,
                        !is_attacker,
                        self_actor.clone(),
                        stat_actor_opt.clone(),
                        stat_eval,
                        max_depth,
                    )
                } else {
                    self.update_stat(stat_eval, stat_actor_opt.as_ref());
                    None
                }
            }
            _ => {
                self.update_stat(stat_eval, stat_actor_opt.as_ref());
                None
            }
        };
        game.play_back();
        result
    }
}
unsafe impl Send for EngineMat {}

const MAT_ENGINE_ID_NAME: &str = "Mat engine";
const MAT_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

impl logic::Engine for EngineMat {
    fn id(&self) -> logic::EngineId {
        let name = format!(
            "{} max_depth {} - {}",
            MAT_ENGINE_ID_NAME.to_owned(),
            self.max_depth,
            self.id_number
        )
        .trim()
        .to_string();
        let author = MAT_ENGINE_ID_AUTHOR.to_owned();
        logic::EngineId::new(name, author)
    }
    fn find_best_move(
        &self,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        game: game_state::GameState,
    ) {
        let mut stat_eval = stat_eval::StatEval::default();
        let best_move_opt = self.mat_solver_init(
            &game,
            self_actor.clone(),
            stat_actor_opt.clone(),
            self.max_depth,
            &mut stat_eval,
        );
        let best_move_opt = best_move_opt.map(|m| m.bitboard_move().clone());
        self_actor.do_send(dispatcher::handler_engine::EngineStopThinking::new(
            stat_actor_opt,
        ));
        if let Some(best_move) = best_move_opt {
            let reply = dispatcher::handler_engine::EngineEndOfAnalysis(best_move);
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!(
                    "Engine of id {:?} reply is: '{:?}'",
                    self.id(),
                    reply
                )));
            }
            self_actor.do_send(reply);
        } else {
            // FIXME: send a message
            println!("No mat in {} half moves or less found", self.max_depth);
        }
    }
}

fn send_best_move(
    self_actor: Addr<dispatcher::EngineDispatcher>,
    best_move: bitboard::BitBoardMove,
) {
    let msg = dispatcher::handler_engine::EngineSendBestMove(best_move);
    self_actor.do_send(msg);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix::Actor;

    use crate::entity::engine::actor::engine_dispatcher as dispatcher;
    use crate::{
        entity::{
            engine::component::engine_mat,
            game::{actor::game_manager, component::player},
            uci::actor::uci_entity,
        },
        monitoring::debug,
        ui::notation::long_notation,
    };

    #[cfg(test)]
    use crate::entity::game::component::game_state;

    #[cfg(test)]
    async fn get_game_state(
        game_manager_actor: &game_manager::GameManagerActor,
    ) -> Option<game_state::GameState> {
        let result_or_error = game_manager_actor
            .send(game_manager::handler_game::GetGameState)
            .await;
        result_or_error.unwrap()
    }

    #[actix::test]
    async fn test_game_end() {
        const MAT_DEPTH: u8 = 5;

        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        // mat in 3
        let inputs = vec![
            "position fen 8/R5P1/5P2/3kBp2/3p1P2/1K1P1P2/8/8 w - - 1 3",
            "go",
        ];
        // mat in 5 (no initial check)
        //"position fen 8/R7/4kPP1/3ppp2/3B1P2/1K1P1P2/8/8 w - - 0 1",
        let uci_reader = Box::new(uci_entity::UciReadVecStringWrapper::new(&inputs));
        let mut game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
        //let mut engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let mut engine_player1 = engine_mat::EngineMat::new(
            debug_actor_opt.clone(),
            game_manager.zobrist_table(),
            MAT_DEPTH,
        );
        engine_player1.set_id_number("white");
        let engine_player1_dispatcher = dispatcher::EngineDispatcher::new(
            Arc::new(engine_player1),
            debug_actor_opt.clone(),
            None,
        );
        let mut engine_player2 = engine_mat::EngineMat::new(
            debug_actor_opt.clone(),
            game_manager.zobrist_table(),
            MAT_DEPTH,
        );
        engine_player2.set_id_number("black");
        let engine_player2_dispatcher = dispatcher::EngineDispatcher::new(
            Arc::new(engine_player2),
            debug_actor_opt.clone(),
            None,
        );
        let player1 = player::Player::Human {
            engine_opt: Some(engine_player1_dispatcher.start()),
        };
        let player2 = player::Player::Computer {
            engine: engine_player2_dispatcher.start(),
        };
        let players = player::Players::new(player1, player2);
        game_manager.set_players(players);
        let game_manager_actor = game_manager.start();
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
            None,
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let r = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
            println!("{:?}", r);
        }
        actix::clock::sleep(std::time::Duration::from_secs(100)).await;
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let game = game_opt.as_ref().unwrap();
        let moves = game.gen_moves();
        let moves: Vec<String> = (*moves
            .into_iter()
            .map(|m| long_notation::LongAlgebricNotationMove::build_from_b_move(m).cast())
            .collect::<Vec<String>>())
        .to_vec();
        println!("{:?}", moves);
        assert!(!moves.contains(&"h3h2".to_string()));
    }
}
