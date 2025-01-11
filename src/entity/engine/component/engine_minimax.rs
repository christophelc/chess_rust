use actix::Addr;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use super::engine_logic::{self as logic, Engine};
use super::evaluation::{self, score, stat_eval};
use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::game::component::bitboard::zobrist;
use crate::entity::game::component::game_state;
use crate::entity::stat::actor::stat_entity;
use crate::ui::notation::long_notation;
use crate::{entity::game::component::bitboard, monitoring::debug};

use crate::span_debug;

#[allow(dead_code)]
fn span_debug() -> tracing::Span {
    span_debug!("engine_minimax")
}

#[derive(Debug, Clone)]
pub struct EngineMinimax {
    id_number: String,
    debug_actor_opt: Option<debug::DebugActor>,
    zobrist_table: zobrist::Zobrist,
    max_depth: u8,
}
impl EngineMinimax {
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
    fn minimax(
        &self,
        game: &game_state::GameState,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
    ) -> bitboard::BitBoardMove {
        let num_cpus = num_cpus::get();

        // for each level 1 move, we add a node parent
        let chunks = self.prepare_tree_level_1(game, num_cpus);
        let results: Vec<_> = chunks
            .clone()
            .into_par_iter()
            .map(|chunk| {
                let mut game_clone = game.clone();
                // FIxME: should be global
                let mut stat_eval = stat_eval::StatEval::default();
                let bitboard_move_score = self.minimax_rec(
                    "",
                    &mut game_clone,
                    0,
                    &chunk,
                    self_actor.clone(),
                    stat_actor_opt.clone(),
                    &mut stat_eval,
                );
                let (best_move, score) = (
                    *bitboard_move_score.bitboard_move(),
                    bitboard_move_score.score().clone(),
                );
                // FIXME: send graph to actor
                if self.debug_actor_opt.is_some() {
                    //Self::display_tree(&graph, root_node_id, 0);
                }
                //Self::write_tree(&graph_clone, node_id);
                (best_move, score)
            })
            .collect();
        let (best_move, _score) = Self::merge_best_move_per_branch(&results).unwrap();
        //Self::write_tree(&graph, root_node_id);
        best_move
    }

    fn merge_best_move_per_branch(
        results: &[(bitboard::BitBoardMove, score::Score)],
    ) -> Option<(bitboard::BitBoardMove, score::Score)> {
        if results.is_empty() {
            return None;
        }
        let (mut best_move, mut best_score) = results[0].clone();
        for (b_move, score) in results.iter() {
            if score.value() > best_score.value() {
                best_move = *b_move;
                best_score = score.clone();
            }
        }
        Some((best_move, best_score))
    }

    fn minimax_rec(
        &self,
        variant: &str,
        game: &mut game_state::GameState,
        current_depth: u8,
        moves: &[bitboard::BitBoardMove],
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        stat_eval: &mut stat_eval::StatEval,
    ) -> score::BitboardMoveScore {
        let mut max_score_opt: Option<score::Score> = None;
        let mut best_move_opt: Option<bitboard::BitBoardMove> = None;
        for m in moves {
            let score = self.process_move(
                game,
                *m,
                variant,
                self_actor.clone(),
                stat_actor_opt.clone(),
                stat_eval,
                current_depth,
            );
            if max_score_opt.is_none() || score.is_greater_than(max_score_opt.as_ref().unwrap()) {
                // Send best move
                best_move_opt = Some(*m);
                max_score_opt = Some(score);
                send_best_move(self_actor.clone(), best_move_opt.unwrap());
            }
        }
        score::BitboardMoveScore::new(
            best_move_opt.unwrap(),
            max_score_opt.unwrap(),
            "".to_string(),
        )
    }

    fn prepare_tree_level_1(
        &self,
        game: &game_state::GameState,
        num_cpus: usize,
    ) -> Vec<Vec<bitboard::BitBoardMove>> {
        let moves = game.gen_moves();
        let mut chunk_size = moves.len() / num_cpus;
        if chunk_size <= 4 {
            chunk_size = 4;
        }
        let chunks: Vec<Vec<bitboard::BitBoardMove>> = moves
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();
        chunks
    }
    fn process_move(
        &self,
        game: &mut game_state::GameState,
        m: bitboard::BitBoardMove,
        variant: &str,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        stat_eval: &mut stat_eval::StatEval,
        current_depth: u8,
    ) -> score::Score {
        let long_algebraic_move = long_notation::LongAlgebricNotationMove::build_from_b_move(m);
        let updated_variant = format!("{} {}", variant, long_algebraic_move.cast());
        if current_depth == 0 {
            println!("{}", updated_variant);
        }
        game.play_moves(&[long_algebraic_move], &self.zobrist_table, None, false)
            .unwrap();
        game.update_endgame_status();

        let score = if game.end_game() == game_state::EndGame::None {
            if current_depth < self.max_depth {
                let moves = game.gen_moves();
                let bitboard_move_score = self.minimax_rec(
                    &updated_variant,
                    game,
                    current_depth + 1,
                    &moves,
                    self_actor.clone(),
                    stat_actor_opt.clone(),
                    stat_eval,
                );
                let score = bitboard_move_score.score();
                score::Score::new(-score.value(), current_depth, self.max_depth)
            } else {
                score::Score::new(
                    evaluation::evaluate_position(game, stat_eval, &stat_actor_opt, self.id()),
                    current_depth,
                    self.max_depth,
                )
            }
        } else {
            evaluation::handle_end_game_scenario(game, current_depth, self.max_depth)
        };

        game.play_back();
        score
    }
}
unsafe impl Send for EngineMinimax {}

const MINIMAX_ENGINE_ID_NAME: &str = "Minimax engine";
const MINIMAX_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

impl logic::Engine for EngineMinimax {
    fn id(&self) -> logic::EngineId {
        let name = format!(
            "{} max_depth {} - {}",
            MINIMAX_ENGINE_ID_NAME.to_owned(),
            self.max_depth,
            self.id_number
        )
        .trim()
        .to_string();
        let author = MINIMAX_ENGINE_ID_AUTHOR.to_owned();
        logic::EngineId::new(name, author)
    }
    fn find_best_move(
        &self,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        game: game_state::GameState,
    ) {
        // First generate moves
        let moves = logic::gen_moves(game.bit_position());
        if !moves.is_empty() {
            let best_move = self.minimax(&game, self_actor.clone(), stat_actor_opt.clone());
            self_actor.do_send(dispatcher::handler_engine::EngineStopThinking::new(
                stat_actor_opt,
            ));
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
            // FIXME: Do nothing. The engine should be put asleep
            panic!("To be implemented. When EndGame detected in game_manager, stop the engines")
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
