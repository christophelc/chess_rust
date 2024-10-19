use std::fmt;

use actix::Addr;

use super::engine_logic as logic;
use crate::entity::game::component::bitboard::zobrist;
use crate::entity::game::component::game_state;
use crate::entity::game::component::square::Switch;
use crate::ui::notation::long_notation;
use crate::{entity::game::component::bitboard, monitoring::debug};

use crate::entity::engine::actor::engine_dispatcher as dispatcher;

#[derive(Clone)]
struct Score(i32);
impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone)]
struct NodeNameAndScore {
    move_str: String,
    score: Score,
}
impl fmt::Display for NodeNameAndScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.move_str, self.score)
    }
}
impl NodeNameAndScore {
    fn new(move_str: String) -> Self {
        Self {
            move_str,
            score: Score(0),
        }
    }
    fn set_score(&mut self, score: Score) {
        self.score = score;
    }
}

#[derive(Debug)]
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
    fn display_tree(
        graph: &petgraph::Graph<NodeNameAndScore, ()>,
        node: petgraph::graph::NodeIndex,
        indent: usize,
    ) {
        // Vérifier si le nœud est une feuille (pas de voisins sortants)
        if graph
            .neighbors_directed(node, petgraph::Direction::Outgoing)
            .next()
            .is_none()
        {
            // Si c'est une feuille, utiliser `print!`
            print!("{:indent$}{}", "", graph[node], indent = indent);
        } else {
            // Si ce n'est pas une feuille, utiliser `println!`
            println!("{:indent$}{}", "", graph[node], indent = indent);

            // Parcourir les nœuds enfants (successeurs)
            for neighbor in graph.neighbors_directed(node, petgraph::Direction::Outgoing) {
                Self::display_tree(graph, neighbor, indent + 4); // Augmenter l'indentation pour les enfants
            }
        }
    }
    fn minimax(&self, game: &game_state::GameState) -> bitboard::BitBoardMove {
        let mut graph = petgraph::Graph::<NodeNameAndScore, ()>::new();
        let root_content = NodeNameAndScore::new("root".to_string());
        let root_node_id = graph.add_node(root_content);
        let (best_move, _) = self.minimax_rec("", &game, 0, &root_node_id, &mut graph);
        // FIXME: send graph to actor
        if self.debug_actor_opt.is_some() {
            Self::display_tree(&graph, root_node_id, 0);
        }
        best_move
    }
    fn minimax_rec(
        &self,
        variant: &str,
        game: &game_state::GameState,
        current_depth: u8,
        node_parent_id: &petgraph::graph::NodeIndex,
        graph: &mut petgraph::Graph<NodeNameAndScore, ()>,
    ) -> (bitboard::BitBoardMove, Score) {
        let mut max_score = i32::MIN;
        let mut best_move = game.moves()[0];
        for m in game.moves() {
            // TODO: optimize that
            let long_algebric_move = long_notation::LongAlgebricNotationMove::build_from_b_move(*m);
            if current_depth == 0 {
                println!("{}", long_algebric_move.cast());
            }
            let mut move_score = NodeNameAndScore::new(long_algebric_move.cast());
            // update graph with new child with score equals to zero
            let node_id: petgraph::graph::NodeIndex = graph.add_node(move_score.clone());
            graph.add_edge(*node_parent_id, node_id, ());
            // TODO: implement 'go back' feature instead of cloning game_state
            let mut game_clone = game.clone();
            let updated_variant = format!("{} {}", variant, long_algebric_move.cast());
            let _ = game_clone
                .play_moves(&[long_algebric_move], &self.zobrist_table, None)
                .unwrap();
            let score = if game_clone.end_game() != game_state::EndGame::None {
                // TODO: evaluate end game
                Score(0)
            } else if current_depth < self.max_depth {
                let (_, score) = self.minimax_rec(
                    &updated_variant,
                    &game_clone,
                    current_depth + 1,
                    &node_id,
                    graph,
                );
                Score(-score.0)
            } else {
                let score = evaluate(game_clone.bit_position());
                // update current leaf node_id with score
                if let Some(node) = graph.node_weight_mut(node_id) {
                    move_score.set_score(score.clone());
                    *node = move_score;
                }
                score
            };
            if score.0 > max_score {
                best_move = *m;
                max_score = score.0;
                // update aggregated score in the parent node
                if let Some(node_parent) = graph.node_weight_mut(*node_parent_id) {
                    let mut content = NodeNameAndScore::new(node_parent.move_str.clone());
                    content.set_score(Score(max_score));
                    *node_parent = content;
                }
            }
        }
        (best_move, Score(max_score))
    }
}
unsafe impl Send for EngineMinimax {}

const MINIMAX_ENGINE_ID_NAME: &str = "Minimax engine";
const MINIMAX_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

impl logic::Engine for EngineMinimax {
    fn id(&self) -> logic::EngineId {
        let name = format!("{} {}", MINIMAX_ENGINE_ID_NAME.to_owned(), self.id_number)
            .trim()
            .to_string();
        let author = MINIMAX_ENGINE_ID_AUTHOR.to_owned();
        logic::EngineId::new(name, author)
    }
    fn find_best_move(
        &self,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        game: game_state::GameState,
    ) {
        // First generate moves
        let moves = logic::gen_moves(&game.bit_position());
        if !moves.is_empty() {
            let best_move = self.minimax(&game);
            self_actor.do_send(dispatcher::handler_engine::EngineStopThinking);
            let reply = dispatcher::handler_engine::EngineBestMoveFound(best_move);
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!(
                    "EngineDummy of id {:?} reply is: '{:?}'",
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

fn evaluate_one_side(bitboards: &bitboard::BitBoards) -> u32 {
    let n_rooks = bitboards.rooks().bitboard().iter().count() as u32;
    let n_knights = bitboards.knights().bitboard().iter().count() as u32;
    let n_bishops = bitboards.bishops().bitboard().iter().count() as u32;
    let n_queens = bitboards.queens().bitboard().iter().count() as u32;
    let n_pawns = bitboards.pawns().bitboard().iter().count() as u32;
    let score = n_rooks * 5 + n_knights * 3 + n_bishops * 3 + n_queens * 10 + n_pawns;
    //println!("r:{} n:{} b:{} q:{} p:{} -> {}", n_rooks, n_knights, n_bishops, n_queens, n_pawns, score);
    score
}

fn evaluate(bit_position: &bitboard::BitPosition) -> Score {
    let color = bit_position.bit_position_status().player_turn().switch();
    let score_current =
        evaluate_one_side(bit_position.bit_boards_white_and_black().bit_board(&color));
    let score_opponent = evaluate_one_side(
        bit_position
            .bit_boards_white_and_black()
            .bit_board(&color.switch()),
    );
    // println!("{}", bit_position.to().chessboard());
    // println!("{:?} / {:?}", score_current, score_opponent);
    let score = score_current as i32 - score_opponent as i32;
    Score(score)
}

#[cfg(test)]
mod tests {
    use crate::entity::game::component::{bitboard, square::TypePiece};

    use super::evaluate_one_side;

    #[test]
    fn test_evaluation_one_side() {
        let mut bitboards = bitboard::BitBoards::default();
        bitboards.xor_piece(TypePiece::Rook, bitboard::BitBoard::new(1));
        bitboards.xor_piece(TypePiece::Pawn, bitboard::BitBoard::new(2));
        let score = evaluate_one_side(&bitboards);
        assert_eq!(score, 6);
    }
}
