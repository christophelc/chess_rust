use crate::entity::engine::component::feature;

#[derive(Debug, Clone)]
pub struct DummyConf {}

#[derive(Debug, Clone)]
pub struct MinimaxConf {
    pub max_depth: u8,
}

#[derive(Debug, Clone)]
pub struct AlphabetaConf {
    pub max_depth: u8,
    pub alpha_beta_features: AlphabetaFeatureConf,
}
#[derive(Debug, Clone)]
pub struct AlphabetaFeatureConf {
    pub f_null_move_pruning: bool,
    pub f_transposition_table: bool,
    pub f_preorder: bool,
    pub f_lmr: bool,
    pub f_killer_move: bool,
    pub f_capture_horizon: bool,
    pub f_check_horizon: bool,
    pub f_cannot_win_force_null: bool,
}
impl AlphabetaFeatureConf {
    pub fn default() -> Self {
        Self {
            f_null_move_pruning: feature::FEATURE_NULL_MOVE_PRUNING,
            f_transposition_table: feature::FEATURE_TRANSPOSITION_TABLE,
            f_preorder: feature::FEATURE_PREORDER,
            f_lmr: feature::FEATURE_LMR,
            f_killer_move: feature::FEATURE_KILLER_MOVE,
            f_capture_horizon: feature::FEATURE_CAPTURE_HORIZON,
            f_check_horizon: feature::FEATURE_CHECK_HORIZON,
            f_cannot_win_force_null: feature::FEATURE_CANNOT_WIN_FORCE_NULL,
        }
    }
}
impl AlphabetaConf {
    pub fn new(max_depth: u8, features: AlphabetaFeatureConf) -> Self {
        Self {
            max_depth,
            alpha_beta_features: features,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IDDFSConfig {
    pub max_depth: u8,
    pub iddfs_feature_conf: IddfsFeatureConf,
    pub alphabeta_feature_conf: AlphabetaFeatureConf,
}
impl IDDFSConfig {
    pub fn new(max_depth: u8, iddfs_feature_conf: IddfsFeatureConf, alphabeta_feature_conf: AlphabetaFeatureConf) -> Self {
        Self {
            max_depth,
            iddfs_feature_conf,
            alphabeta_feature_conf,
        }
    }
}
#[derive(Debug, Clone)]
pub struct IddfsFeatureConf {
    pub f_mat_solver: bool,
    pub f_aspiration_window: bool,
}
impl IddfsFeatureConf {
    pub fn default() -> Self {
        Self {
            f_mat_solver: feature::FEATURE_MAT_SOLVER,
            f_aspiration_window: feature::FEATURE_ASPIRATION_WINDOW,            
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatConfig {
    pub max_depth: u8,
}
impl MatConfig {
    pub fn new(max_depth: u8) -> Self {
        Self {
            max_depth,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MctsConfig {
    pub iterations_per_move: u64,
    pub c: f64,
}
impl MctsConfig {
    pub fn new(iterations_per_move: u64) -> Self {
        Self {
            iterations_per_move,
            c: 1.0,
        }
    }
}