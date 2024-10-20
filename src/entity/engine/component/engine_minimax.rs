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
            //Self::display_tree(&graph, root_node_id, 0);
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
            //for m in game.moves().into_iter().filter(|m| current_depth > 0 || long_notation::LongAlgebricNotationMove::build_from_b_move(**m).cast() == "e7f8B") {
            // TODO: optimize that
            let long_algebric_move = long_notation::LongAlgebricNotationMove::build_from_b_move(*m);
            let updated_variant = format!("{} {}", variant, long_algebric_move.cast());
            let mut move_score = NodeNameAndScore::new(long_algebric_move.cast());
            // update graph with new child with score equals to zero
            let node_id: petgraph::graph::NodeIndex = graph.add_node(move_score.clone());
            if current_depth == 0 {
                println!("{}", updated_variant);
            }
            graph.add_edge(*node_parent_id, node_id, ());
            // TODO: implement 'go back' feature instead of cloning game_state
            let mut game_clone = game.clone();
            //println!("{}",game_clone.bit_position().to().chessboard());
            let _ = game_clone
                .play_moves(&[long_algebric_move], &self.zobrist_table, None)
                .unwrap();
            let score = match game_clone.end_game() {
                game_state::EndGame::None => {
                    if current_depth < self.max_depth {
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
                    }
                }
                // the last mave wins => it is a very good move
                game_state::EndGame::Mat(_color) => Score(i32::MAX),
                game_state::EndGame::TimeOutLost(color)
                    if color
                        == game_clone
                            .bit_position()
                            .bit_position_status()
                            .player_turn() =>
                {
                    Score(i32::MAX)
                }
                // the last move loses
                game_state::EndGame::TimeOutLost(_color) => Score(i32::MIN),
                _ => Score(0),
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
                    "Engine pf id {:?} reply is: '{:?}'",
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
    use std::sync::Arc;

    use actix::Actor;

    use crate::entity::engine::actor::engine_dispatcher as dispatcher;
    use crate::{
        entity::{
            engine::component::engine_minimax,
            game::{
                actor::game_manager,
                component::{bitboard, player, square::TypePiece},
            },
            uci::actor::uci_entity,
        },
        monitoring::debug,
        ui::notation::long_notation,
    };

    use super::evaluate_one_side;

    #[test]
    fn test_evaluation_one_side() {
        let mut bitboards = bitboard::BitBoards::default();
        bitboards.xor_piece(TypePiece::Rook, bitboard::BitBoard::new(1));
        bitboards.xor_piece(TypePiece::Pawn, bitboard::BitBoard::new(2));
        let score = evaluate_one_side(&bitboards);
        assert_eq!(score, 6);
    }

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

    // FIXME: remove sleep
    #[ignore]
    #[actix::test]
    async fn test_game_end() {
        const MINIMAX_DEPTH: u8 = 2;

        //let debug_actor_opt: Option<debug::DebugActor> = None;
        let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 b8a6 f1a6 b7a6 d2d4 d7d5 e4e5 c7c6 g1f3 a8b8 e1g1 c8g4 d1d3 b8b4 c2c3 b4a4 b2b3 a4a5 c1d2 g4f3 g2f3 a5b5 c3c4 b5b7 c4d5 d8d5 d3c3 b7b5 d2e3 d5f3 c3c6 f3c6 b1a3 b5b4 a1c1 c6e6 a3c4 b4b5 f1d1 b5b4 d4d5 e6g4 g1f1 b4b7 d5d6 g4h3 f1g1 h3g4 g1f1 g4h3 f1e1 h3h2 d6e7 g8f6", "go"];
        let uci_reader = uci_entity::UciReadVecStringWrapper::new(&inputs);
        let mut game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
        //let mut engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let mut engine_player1 = engine_minimax::EngineMinimax::new(
            debug_actor_opt.clone(),
            game_manager.zobrist_table(),
            MINIMAX_DEPTH,
        );
        engine_player1.set_id_number("white");
        let engine_player1_dispatcher =
            dispatcher::EngineDispatcher::new(Arc::new(engine_player1), debug_actor_opt.clone());
        //let mut engine_player2 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let mut engine_player2 = engine_minimax::EngineMinimax::new(
            debug_actor_opt.clone(),
            game_manager.zobrist_table(),
            MINIMAX_DEPTH,
        );
        engine_player2.set_id_number("black");
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
        let game_manager_actor = game_manager.start();
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
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
        let moves = game.moves();
        let moves: Vec<String> = (*moves
            .into_iter()
            .map(|m| long_notation::LongAlgebricNotationMove::build_from_b_move(*m).cast())
            .collect::<Vec<String>>())
        .to_vec();
        assert!(!moves.contains(&"h3h2".to_string()));
    }
}
