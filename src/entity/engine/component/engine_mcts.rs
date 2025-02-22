use actix::Addr;
use rand::Rng;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::engine::component::{engine_logic as logic, mcts_tree};
use crate::entity::game::component::bitboard::zobrist;
use crate::entity::game::component::square::Switch;
use crate::entity::game::component::{game_state, square};
use crate::entity::stat::actor::stat_entity;
use crate::span_debug;
use crate::ui::notation::long_notation;
use crate::{entity::game::component::bitboard, monitoring::debug};

use super::config::config;

const MAX_TREE_ITERATION: u64 = 1000;

fn span_debug() -> tracing::Span {
    span_debug!("engine_mcts")
}

#[derive(Default)]
struct MctsStat {
    n_simulation: u64,
    n_moves_per_game: u64,
    n_moves_gen: u64,
}
impl fmt::Display for MctsStat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let avg_moves_played = self.n_moves_per_game as f64 / (self.n_simulation as f64);
        let avg_moves_generated: f64 = self.n_moves_gen as f64 / (self.n_simulation as f64);
        write!(f, "n_simulation: {} - avg moves played per game: {}, avg number of moves generated per game: {}", self.n_simulation, avg_moves_played, avg_moves_generated)
    }
}
impl MctsStat {
    pub fn inc(&mut self, n_simulation: u64, n_moves_per_game: u64, n_moves_gen: u64) {
        self.n_simulation += n_simulation;
        self.n_moves_gen += n_moves_gen;
        self.n_moves_per_game += n_moves_per_game;
    }
}
pub struct EngineMcts {
    id_number: String,
    debug_actor_opt: Option<debug::DebugActor>,
    zobrist_table: zobrist::Zobrist,
    conf: config::MctsConfig,
}
impl EngineMcts {
    pub fn new(
        debug_actor_opt: Option<debug::DebugActor>,
        zobrist_table: zobrist::Zobrist,
        conf: &config::MctsConfig,
    ) -> Self {
        Self {
            id_number: "".to_string(),
            debug_actor_opt,
            zobrist_table,
            conf: conf.clone(),
        }
    }
    pub fn set_id_number(&mut self, id_number: &str) {
        self.id_number = id_number.to_string();
    }

    pub fn mcts(
        &self,
        game: &game_state::GameState,
        is_stop: &Arc<AtomicBool>,
    ) -> bitboard::BitBoardMove {
        let span = span_debug();
        let _enter = span.enter();

        let mut graph = mcts_tree::Graph::new();
        let moves = game.gen_moves();
        let root = mcts_tree::Node::build_root(game.clone(), &moves);
        let root_id = mcts_tree::add_node_to_graph(&mut graph, root.clone());
        let mut mcts_stat = MctsStat::default();
        for i in 0..MAX_TREE_ITERATION {
            if is_stop.load(Ordering::Relaxed) {
                break;
            }
            if i % 100 == 0 {
                tracing::debug!("tree iteration number: {}", i);
            }
            self.mcts_run(&mut graph, root_id, &mut mcts_stat);
        }
        if let Some(idx) = mcts_tree::Node::argmax(&graph, graph[root_id].children(), self.conf.c) {
            let best_move_id = graph[root_id].children().get(idx).unwrap();
            let best_move = if let Some(edge_index) = graph.find_edge(root_id, *best_move_id) {
                let edge = graph.edge_weight(edge_index).unwrap();
                edge.0
            } else {
                panic!("Graph error: edge not found");
            };
            mcts_tree::display_tree(&graph, root_id, 0, 0);
            let total: u64 = graph[root_id]
                .children()
                .iter()
                .map(|n_idx| graph[*n_idx].visits())
                .sum();
            tracing::debug!("total visits in children level 1: {}", total);
            tracing::debug!("total visits root: {}", graph[root_id].visits());
            tracing::debug!("{}", mcts_stat);
            best_move
        } else {
            panic!("No move found")
        }
    }
    fn mcts_run(
        &self,
        graph: &mut mcts_tree::Graph,
        node_id: mcts_tree::NodeIdx,
        mcts_stat: &mut MctsStat,
    ) {
        let span = span_debug();
        let _enter = span.enter();

        if graph[node_id].is_terminal() {
            let (n_white_wins, n_black_wins) = Self::evaluate_end_game(graph[node_id].game());
            self.mcts_back_propagation(graph, node_id, n_white_wins, n_black_wins);
        } else {
            let node = &graph[node_id];
            if node.untried_moves().is_empty() {
                tracing::debug!("selection");
                if node.children().is_empty() {
                    // generate moves
                    let moves = graph[node_id].game().gen_moves();
                    let invalid_move: Vec<_> = moves
                        .iter()
                        .filter(|m| m.capture() == Some(square::TypePiece::King))
                        .collect();
                    if !invalid_move.is_empty() {
                        tracing::debug!(
                            "{}",
                            graph[node_id].game().bit_position().to().chessboard()
                        );
                    }
                    graph[node_id].set_untried_moves(moves);
                } else {
                    // selection: all moves have been expanded: select the best ucb1 score
                    match mcts_tree::Node::argmax(graph, node.children(), self.conf.c) {
                        None => tracing::debug!("not found"),
                        Some(idx) => {
                            tracing::debug!("found");
                            let selected_node_idx = node.children().get(idx).unwrap();
                            self.mcts_run(graph, *selected_node_idx, mcts_stat)
                        }
                    }
                }
            // expand untried moves first
            } else {
                // expansion: add an untried move as a child
                let expanded_node_idx = self.exploration(graph, node_id);
                tracing::debug!("simulation");
                let (n_white_wins, n_black_wins) =
                    self.mcts_simulation(graph, expanded_node_idx, mcts_stat);
                self.mcts_back_propagation(graph, expanded_node_idx, n_white_wins, n_black_wins);
            }
        }
    }
    fn exploration(
        &self,
        graph: &mut mcts_tree::Graph,
        node_id: mcts_tree::NodeIdx,
    ) -> mcts_tree::NodeIdx {
        let mut rng = rand::thread_rng();
        let node = &graph[node_id];
        tracing::debug!("exploration / {}", node.untried_moves().len());
        let random_index = rng.gen_range(0..node.untried_moves().len()); // Random index
        mcts_tree::Node::exploration(graph, node_id, random_index, &self.zobrist_table)
    }
    fn mcts_simulation(
        &self,
        graph: &mcts_tree::Graph,
        expanded_node_idx: mcts_tree::NodeIdx,
        mcts_stat: &mut MctsStat,
    ) -> (u64, u64) {
        let mut n_white_wins: u64 = 0;
        let mut n_black_wins: u64 = 0;
        for _i in 0..self.conf.iterations_per_move {
            let (n_white_win, n_black_win) =
                self.mcts_one_simulation(graph, expanded_node_idx, mcts_stat);
            n_white_wins += n_white_win;
            n_black_wins += n_black_win;
        }
        (n_white_wins, n_black_wins)
    }
    // return None if Draw game, else return the winner
    fn mcts_one_simulation(
        &self,
        graph: &mcts_tree::Graph,
        node_id: mcts_tree::NodeIdx,
        mcts_stat: &mut MctsStat,
    ) -> (u64, u64) {
        let mut rng = rand::thread_rng();
        let mut game = graph[node_id].game().clone();
        let mut n_moves_gen: u64 = 0;
        while game.end_game() == game_state::EndGame::None {
            let moves = game.gen_moves();
            n_moves_gen += moves.len() as u64;
            let random_index = rng.gen_range(0..moves.len());
            let m = moves[random_index];
            let long_algebraic_move = long_notation::LongAlgebricNotationMove::build_from_b_move(m);
            let _ = game.play_moves(&[long_algebraic_move], &self.zobrist_table, None, false);
            game.update_endgame_status();
        }
        mcts_stat.inc(
            1,
            game.bit_position().bit_position_status().n_moves() as u64,
            n_moves_gen,
        );
        Self::evaluate_end_game(&game)
    }
    pub fn evaluate_end_game(game: &game_state::GameState) -> (u64, u64) {
        let color_win_opt = match game.end_game() {
            game_state::EndGame::Mat(lost_color) => Some(lost_color.switch()),
            game_state::EndGame::TimeOutLost(lost_color) => Some(lost_color.switch()),
            _ => None,
        };
        match color_win_opt {
            Some(square::Color::White) => (1, 0),
            Some(square::Color::Black) => (0, 1),
            None => (0, 0),
        }
    }
    pub fn mcts_back_propagation(
        &self,
        graph: &mut mcts_tree::Graph,
        node_id: mcts_tree::NodeIdx,
        n_white_wins: u64,
        n_black_wins: u64,
    ) {
        let span = span_debug();
        let _enter = span.enter();

        tracing::debug!("back propagation\n");
        // FIXME: check if player_turm is the opposite
        let player_turn = graph[node_id]
            .game()
            .bit_position()
            .bit_position_status()
            .player_turn();
        let n_wins = if player_turn == square::Color::White {
            n_white_wins
        } else {
            n_black_wins
        };
        graph[node_id].inc_stat(n_wins, self.conf.iterations_per_move);
        let mut node_iter = node_id;
        while let Some(node_id) = graph[node_iter].parent() {
            tracing::debug!(
                "inc {:?} {}/{} -> node updated = {}/{}",
                node_id,
                n_wins,
                self.conf.iterations_per_move,
                graph[node_id].n_wins(),
                graph[node_id].visits()
            );
            graph[node_id].inc_stat(n_wins, self.conf.iterations_per_move);
            node_iter = node_id;
        }
    }
}
unsafe impl Send for EngineMcts {}

const MCTS_ENGINE_ID_NAME: &str = "MCTS engine";
const MCTS_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

impl logic::Engine for EngineMcts {
    fn id(&self) -> logic::EngineId {
        let name = format!(
            "{} iterations {} - {}",
            MCTS_ENGINE_ID_NAME.to_owned(),
            self.conf.iterations_per_move,
            self.id_number
        )
        .trim()
        .to_string();
        let author = MCTS_ENGINE_ID_AUTHOR.to_owned();
        logic::EngineId::new(name, author)
    }
    fn find_best_move(
        &self,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        game: game_state::GameState,
        is_stop: &Arc<AtomicBool>,
    ) {
        let moves = logic::gen_moves(game.bit_position());
        if !moves.is_empty() {
            let best_move = self.mcts(&game, is_stop);
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
