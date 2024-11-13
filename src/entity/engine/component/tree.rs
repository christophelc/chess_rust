use petgraph::visit::EdgeRef;
use std::fs::OpenOptions;
use std::io::Write;
use std::{env, fmt};

use super::score;
use crate::entity::game::component::bitboard;
use crate::entity::stat::component::stat_data;
use crate::ui::notation::long_notation;

const TREE_FILE_PATH: &str = "tree.txt";

#[derive(Clone)]
enum NodeType {
    Root(RootNodeContent),
    Regular(RegularNodeContent),
}
impl NodeType {
    fn new_root() -> Self {
        NodeType::Root(RootNodeContent::default())
    }
}
impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeType::Root(root_node_content) => {
                write!(f, "root with score: {}", root_node_content)
            }
            NodeType::Regular(regular_node_content) => {
                write!(f, "root with score: {}", regular_node_content)
            }
        }
    }
}
#[derive(Clone, Default)]
struct RootNodeContent {
    score_opt: Option<score::Score>,
}
impl fmt::Display for RootNodeContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "root with score: {:?}", self.score_opt)
    }
}

#[derive(Clone)]
struct RegularNodeContent {
    b_move: bitboard::BitBoardMove,
    score_opt: Option<score::Score>,
}
impl fmt::Display for RegularNodeContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let move_long_notation =
            long_notation::LongAlgebricNotationMove::build_from_b_move(self.b_move);
        write!(f, "{}:{:?}", move_long_notation.cast(), self.score_opt)
    }
}
impl RegularNodeContent {
    fn new(b_move: bitboard::BitBoardMove) -> Self {
        Self {
            b_move,
            score_opt: None,
        }
    }
    fn set_score(&mut self, score: score::Score) {
        self.score_opt = Some(score);
    }
}
#[allow(dead_code)]
fn write_tree(graph: &petgraph::Graph<NodeType, ()>, node: petgraph::graph::NodeIndex) {
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
    display_tree(graph, node, indent, 0, &mut file)
}

#[allow(dead_code)]
fn display_tree(
    graph: &petgraph::Graph<NodeType, ()>,
    node: petgraph::graph::NodeIndex,
    indent: usize,
    level: i8,
    file: &mut std::fs::File,
) {
    // Vérifier si le nœud est une feuille (pas de voisins sortants)
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
            // Parcourir les nœuds enfants (successeurs)
            for neighbor in graph.neighbors_directed(node, petgraph::Direction::Outgoing) {
                display_tree(graph, neighbor, indent + 3, level + 1, file);
                // Augmenter l'indentation pour les enfants
            }
        }
    }
}
fn add_graph_node(
    graph: &mut petgraph::Graph<NodeType, ()>,
    parent_node_id: petgraph::graph::NodeIndex,
    node_content: RegularNodeContent,
) -> petgraph::graph::NodeIndex {
    let node_id = graph.add_node(NodeType::Regular(node_content));
    graph.add_edge(parent_node_id, node_id, ());
    node_id
}
