use std::fs::OpenOptions;
use std::io::Write;
use std::{env, fmt};

use crate::entity::game::component::bitboard::zobrist;
use crate::entity::game::component::{bitboard, game_state};
use crate::ui::notation::long_notation;

const TREE_FILE_PATH: &str = "tree.txt";

pub type NodeIdx = petgraph::graph::NodeIndex;
pub type Graph = petgraph::graph::Graph<Node, EdgeMove>;
pub struct EdgeMove(pub bitboard::BitBoardMove);

use crate::span_debug;

fn span_debug() -> tracing::Span {
    span_debug!("mcts_tree")
}

#[derive(Debug, Clone)]
pub struct Node {
    index: Option<NodeIdx>,
    parent: Option<NodeIdx>,
    children: Vec<NodeIdx>,
    untried_moves: Vec<bitboard::BitBoardMove>,
    game: game_state::GameState,
    n_visits: u64,
    n_wins: u64,
}
impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.n_wins, self.n_visits)
    }
}
impl Node {
    pub fn index(&self) -> Option<NodeIdx> {
        self.index
    }
    pub fn parent(&self) -> Option<NodeIdx> {
        self.parent
    }
    pub fn game(&self) -> &game_state::GameState {
        &self.game
    }
    pub fn children(&self) -> &Vec<NodeIdx> {
        &self.children
    }
    pub fn untried_moves(&self) -> &Vec<bitboard::BitBoardMove> {
        &self.untried_moves
    }
    pub fn n_wins(&self) -> u64 {
        self.n_wins
    }
    pub fn visits(&self) -> u64 {
        self.n_visits
    }

    pub fn add_child(parent_idx: NodeIdx, game: game_state::GameState) -> Self {
        Self {
            index: None,
            parent: Some(parent_idx),
            children: vec![],
            untried_moves: vec![],
            game,
            n_visits: 0,
            n_wins: 0,
        }
    }
    pub fn build_root(game: game_state::GameState, moves: &[bitboard::BitBoardMove]) -> Self {
        Self {
            index: None,
            parent: None,
            children: vec![],
            untried_moves: moves.to_vec(),
            game,
            n_visits: 0,
            n_wins: 0,
        }
    }
    pub fn get_node_mut(graph: &mut Graph, node_idx: NodeIdx) -> &mut Node {
        if let Some(node) = graph.node_weight_mut(node_idx) {
            node
        } else {
            panic!("No node {:?}", node_idx);
        }
    }
    pub fn argmax(graph: &Graph, values: &[NodeIdx], c: f64) -> Option<usize> {
        values
            .iter()
            .enumerate()
            .fold(None, |max_index: Option<(usize, &Node)>, (i, node_idx)| {
                let node = &graph[*node_idx];
                match max_index {
                    Some((max_i, max_value)) if node.ucb1(graph, c) <= max_value.ucb1(graph, c) => {
                        Some((max_i, max_value))
                    }
                    _ => Some((i, node)),
                }
            })
            .map(|(i, _)| i)
    }
    pub fn ucb1(&self, graph: &Graph, c: f64) -> f64 {
        assert!(!self.is_root());
        if self.n_visits == 0 {
            // ensure a node is explored at least once
            f64::INFINITY
        } else {
            let exploitation_rate = (self.n_wins as f64) / (self.n_visits as f64);
            let mut exploration_rate = 0f64;
            if let Some(node_parent) = graph.node_weight(self.parent.unwrap()) {
                exploration_rate =
                    c * ((node_parent.visits() as f64).ln() / (self.n_visits as f64));
            }
            exploitation_rate + exploration_rate
        }
    }
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }
    pub fn is_terminal(&self) -> bool {
        self.children.is_empty()
            && self.untried_moves.is_empty()
            && self.game.end_game() != game_state::EndGame::None
    }
    pub fn set_untried_moves(&mut self, untried_moves: Vec<bitboard::BitBoardMove>) {
        self.untried_moves = untried_moves;
    }
    pub fn inc_stat(&mut self, n_new_wins: u64, n_new_visits: u64) {
        self.n_wins += n_new_wins;
        self.n_visits += n_new_visits;
    }
    // add a new child based on the untried_moves at index idx
    pub fn exploration(
        graph: &mut Graph,
        node_idx: NodeIdx,
        idx: usize,
        zobrist_table: &zobrist::Zobrist,
    ) -> NodeIdx {
        let span = span_debug();
        let _enter = span.enter();

        assert!(idx < graph[node_idx].untried_moves.len());
        let selected_move = graph[node_idx].untried_moves.swap_remove(idx);
        let long_algebraic_move =
            long_notation::LongAlgebricNotationMove::build_from_b_move(selected_move);
        let mut game_clone = graph[node_idx].game.clone();
        game_clone
            .play_moves(&[long_algebraic_move], zobrist_table, None, false)
            .unwrap();
        game_clone.update_endgame_status();
        // create child node
        let new_node = Node::add_child(graph[node_idx].index.unwrap(), game_clone);
        let child_id = add_node_to_graph(graph, new_node);
        let edge_move = EdgeMove(selected_move);
        graph.add_edge(graph[node_idx].index.unwrap(), child_id, edge_move);
        // Update the parent's children list
        graph[node_idx].children.push(child_id);
        tracing::debug!(
            "node {:?} updated -> children: {:?}",
            node_idx,
            graph[node_idx].children()
        );
        child_id
    }
}

#[allow(dead_code)]
fn write_tree(graph: &Graph, node: NodeIdx) {
    let exe_path = env::current_exe().expect("Failed to find executable path");
    let folder_exe_path = exe_path
        .parent()
        .expect("Failed to get folder executable path");
    let path = format!("{}/{}", folder_exe_path.display(), TREE_FILE_PATH);
    let mut file = OpenOptions::new()
        .write(true) // Set append mode
        .create(true) // Create file if it doesn't exist
        .truncate(true)
        .open(path)
        .expect("Failed to open or create file");
    let indent = 0;
    display_tree_to_file(graph, node, indent, 0, &mut file)
}

#[allow(dead_code)]
fn display_tree_to_file(
    graph: &Graph,
    node: NodeIdx,
    indent: usize,
    level: i8,
    file: &mut std::fs::File,
) {
    // Leaf ?
    if graph
        .neighbors_directed(node, petgraph::Direction::Outgoing)
        .next()
        .is_none()
    {
        let output = format!("{:indent$}{} {}\n", "", level, graph[node], indent = indent);
        let _ = file.write_all(output.as_bytes());
    } else {
        let output = format!("{:indent$}{} {}\n", "", level, graph[node], indent = indent);
        let _ = file.write_all(output.as_bytes());
        if level < 5 {
            // Iterate over children
            for neighbor in graph.neighbors_directed(node, petgraph::Direction::Outgoing) {
                display_tree(graph, neighbor, indent + 3, level + 1);
                // Augmenter l'indentation pour les enfants
            }
        }
    }
}

pub fn display_tree(graph: &Graph, node: NodeIdx, indent: usize, level: i8) {
    let span = span_debug();
    let _enter = span.enter();

    let mut m_as_str: String = "".to_string();
    if let Some(parent) = graph[node].parent() {
        if let Some(edge_index) = graph.find_edge(parent, node) {
            let edge = graph.edge_weight(edge_index).unwrap();
            let s = &long_notation::LongAlgebricNotationMove::build_from_b_move(edge.0);
            m_as_str = s.cast().to_string();
        }
    }

    // Vérifier si le nœud est une feuille (pas de voisins sortants)
    if graph
        .neighbors_directed(node, petgraph::Direction::Outgoing)
        .next()
        .is_none()
    {
        // only show explored nodes
        if graph[node].visits() > 0 {
            let output = format!("{:indent$}{} {}\n", "", level, graph[node], indent = indent);
            println!("{}", output);
        }
    } else {
        // only show explored nodes
        if graph[node].visits() > 0 {
            let output = format!(
                "{:indent$}{} {} {}\n",
                "",
                level,
                m_as_str,
                graph[node],
                indent = indent
            );
            tracing::debug!("{}", output);
        }
        if level < 5 {
            // Parcourir les nœuds enfants (successeurs)
            for neighbor in graph.neighbors_directed(node, petgraph::Direction::Outgoing) {
                display_tree(graph, neighbor, indent + 3, level + 1);
            }
        }
    }
}

pub fn add_node_to_graph(graph: &mut Graph, node: Node) -> NodeIdx {
    let node_idx = graph.add_node(node);
    // Update the `index` field after the node is added
    graph[node_idx].index = Some(node_idx);
    node_idx
}
