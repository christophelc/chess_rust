use chess_actix::benchmark;
use chess_actix::entity::engine::component::config::config;
#[allow(unused_imports)]
use chess_actix::entity::engine::component::engine_alphabeta;
use chess_actix::entity::engine::component::engine_iddfs;
#[allow(unused_imports)]
use chess_actix::entity::engine::component::engine_mat;
#[allow(unused_imports)]
use chess_actix::entity::engine::component::engine_mcts;
#[allow(unused_imports)]
use chess_actix::entity::engine::component::engine_minimax;
use chess_actix::entity::game::component::bitboard::zobrist;
use chrono::{Local, TimeZone, Utc};

use clap::Parser;
#[allow(unused_imports)]
use entity::engine::component::engine_dummy as dummy;

use chess_actix::entity::stat::actor::stat_entity;
use chess_actix::{entity, monitoring, ui};

use actix::Actor;
use entity::game::actor::game_manager;
use entity::game::component::square;
use std::env;
use std::io;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use tokio::io::AsyncBufReadExt as _;

use entity::engine::actor::engine_dispatcher as dispatcher;
use entity::game::component::bitboard::{
    piece_move::{self, GenMoves},
    BitPosition,
};
use entity::game::component::player;
use entity::uci::actor::uci_entity::{self, UciRead};
use fen::EncodeUserInput;
use monitoring::debug;
use ui::notation::{fen, san};

use tokio::sync::mpsc;
use tracing_appender::rolling;
use tracing_subscriber::{self, layer::SubscriberExt};

const DEPTH: u8 = 4;
const LOG_FILE_ONLY: bool = false;

#[allow(dead_code)]
fn fen() {
    println!("chessboard generated from initial position encoded with FEN");
    let position: fen::Position = fen::Position::build_initial_position();
    println!("{}", position.chessboard());
    let fen_str = fen::Fen::encode(&position).expect("Error when decoding position to FEN format.");
    println!("Encode initial position to FEN position:");
    println!("{}", fen_str);
}

#[allow(dead_code)]
async fn test(game_manager_actor: &game_manager::GameManagerActor) {
    let zobrist_table = zobrist::Zobrist::new();
    println!("Inital position with move e4");
    let inputs = vec!["position startpos moves e2e4 "];
    let uci_reader = Box::new(uci_entity::UciReadVecStringWrapper::new(&inputs));
    let uci_entity = uci_entity::UciEntity::new(uci_reader, game_manager_actor.clone(), None, None);
    let uci_entity_actor = uci_entity.start();
    uci_entity_actor.do_send(uci_entity::handler_read::ReadUserInput);
    let game_state = game_manager_actor
        .send(game_manager::handler_game::GetGameState)
        .await
        .expect("actix error")
        .expect("Error when retrieving game_state");
    let position = game_state.bit_position().to();
    println!("{}", position.chessboard());
    println!();

    println!("Generate moves for white king considering pawn is in e4");
    let bit_position = BitPosition::from(position);
    let moves = bit_position.bit_boards_white_and_black().gen_moves_for_all(
        &square::Color::White,
        piece_move::CheckStatus::None,
        None,
        bit_position.bit_position_status(),
    );
    let moves_as_str: Vec<String> = moves
        .iter()
        .map(|m| {
            san::san_to_str(
                m,
                &moves,
                &san::Lang::LangFr,
                &game_state,
                &zobrist_table,
                false,
            )
            .info()
            .clone()
        })
        .collect();
    println!("{:?}", moves_as_str);
}

#[allow(dead_code)]
fn uci_loop(
    game_manager_actor: &game_manager::GameManagerActor,
    stdin: &mut Arc<Mutex<io::Stdin>>,
) {
    let uci_reader = Box::new(uci_entity::UciReadWrapper::new(stdin.clone()));
    let uci_entity = uci_entity::UciEntity::new(uci_reader, game_manager_actor.clone(), None, None);
    let uci_entity_actor = uci_entity.start();
    uci_entity_actor.do_send(uci_entity::handler_read::ReadUserInput);
    // // we ignore errors (according to uci specifications)
    // let r = uci::uci_loop(uci_reader, game_actor).await;
    // println!("{:?}", r.err());
}

async fn tui_loop(
    game_manager_actor: &game_manager::GameManagerActor,
    stdin: &mut Arc<Mutex<io::Stdin>>,
) {
    // init the game
    let inputs = vec!["position startpos"];
    let uci_reader = Box::new(uci_entity::UciReadVecStringWrapper::new(&inputs));
    // we don't ignore error in tui mode
    let debug_actor = debug::DebugEntity::default().start();
    let uci_entity = uci_entity::UciEntity::new(
        uci_reader,
        game_manager_actor.clone(),
        Some(debug_actor),
        None,
    );
    let uci_entity_actor = uci_entity.start();
    // read all inputs and execute UCI commands
    for _idx in 0..inputs.len() {
        let _ = uci_entity_actor
            .send(uci_entity::handler_read::ReadUserInput)
            .await;
    }
    //let r = uci::uci_loop(uci_reader, game_actor).await;
    //println!("{:?}", r.err());
    let mut stdin_reader = uci_entity::UciReadWrapper::new(stdin.clone());
    // loop
    loop {
        let game_state = game_manager_actor
            .send(game_manager::handler_game::GetGameState)
            .await
            .expect("actix error")
            .expect("Error when retrieving game_state");
        println!("\n{}", game_state.bit_position().to().chessboard());
        let input_opt = stdin_reader.uci_read();
        match input_opt.as_deref() {
            None => {}
            Some("quit") => break,
            // e2e4 for example
            Some(input) if input.len() == 4 => {
                let moves = vec![input.to_string()];
                match uci_entity::handler_event::moves_validation(&moves) {
                    Err(err) => println!("Error: {}", err),
                    Ok(long_algebric_moves) => {
                        let result = game_manager_actor
                            .send(game_manager::handler_game::PlayMoves::new(
                                long_algebric_moves,
                            ))
                            .await
                            .unwrap();
                        if let Some(err) = result.err() {
                            println!("Move error: {}", err);
                        }
                    }
                }
            }
            _ => println!("Please enter a move to a format like e2e4"),
        }
    }
}

static LOG_GUARDS: OnceLock<(
    tracing_appender::non_blocking::WorkerGuard,
    Option<tracing_appender::non_blocking::WorkerGuard>,
)> = OnceLock::new();
fn init_trace() {
    // Default to debug if PLAIN_LOGS undefined
    let plain_output =
        std::env::var("PLAIN_LOGS").unwrap_or_else(|_| "false".to_string()) == "true";

    // Configure file-based logging
    let file_appender = rolling::daily("./logs", "chess_rust.log");
    let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);

    // Set up environment filter for log levels
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"));

    // Create file logging layer
    let file_layer = tracing_subscriber::fmt::Layer::new()
        .with_writer(file_writer)
        .with_target(true);

    if !LOG_FILE_ONLY {
        // Configure stdout logging
        let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
        let stdout_layer = tracing_subscriber::fmt::Layer::new()
            .with_writer(stdout_writer)
            .with_target(true)
            .with_ansi(plain_output);

        // Combine file and stdout layers
        let subscriber = tracing_subscriber::Registry::default()
            .with(env_filter)
            .with(file_layer)
            .with(stdout_layer);

        // Keep guards alive
        LOG_GUARDS.get_or_init(|| (file_guard, Some(stdout_guard)));
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global subscriber");
    } else {
        // File-only logging
        let subscriber = tracing_subscriber::Registry::default()
            .with(env_filter)
            .with(file_layer);

        // Keep file guard alive
        LOG_GUARDS.get_or_init(|| (file_guard, None));
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global subscriber");
    }
}

fn trace_build_info() {
    let build_date = env!("BUILD_DATE", "BUILD_DATE not set during compilation");
    let git_commit = env!(
        "GIT_COMMIT_HASH",
        "GIT_COMMIT_HASH not set during compilation"
    );
    let timestamp = build_date
        .parse::<i64>()
        .expect("BUILD_DATE should be a valid timestamp");
    let utc_datetime = Utc
        .timestamp_opt(timestamp, 0)
        .single()
        .expect("Invalid timestamp");
    let local_datetime = utc_datetime.with_timezone(&Local);
    let formatted_date = local_datetime.format("%Y-%m-%d %H:%M:%S %Z").to_string();
    tracing::debug!("Build date: {}", formatted_date);
    tracing::debug!("Last commit hash: {}", git_commit);
}

struct BuildParams {
    game_manager_actor: game_manager::GameManagerActor,
    debug_actor_opt: Option<debug::DebugActor>,
    stat_actor_opt: Option<actix::Addr<stat_entity::StatEntity>>,
    stdin: Arc<Mutex<io::Stdin>>,
}
fn init_game_params() -> BuildParams {
    let conf = config::IDDFSConfig::new(
        2*DEPTH -1,
        config::IddfsFeatureConf::default(),
        config::AlphabetaFeatureConf::default(),
    );
    let debug_actor_opt: Option<debug::DebugActor> = None;
    let stat_actor_opt = Some(stat_entity::StatEntity::new(None).start());
    //let debug_actor_opt: Option<debug::DebugActor> = Some(debug::DebugEntity::new(true).start());
    let stdin = Arc::new(Mutex::new(io::stdin()));
    let mut game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
    //let mut engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
    //let mut engine_player1 = engine_mat::EngineMat::new(
    //let mut engine_player1 = engine_alphabeta::EngineAlphaBeta::new(
    let mut engine_player1 = engine_iddfs::EngineIddfs::new(
        debug_actor_opt.clone(),
        game_manager.zobrist_table(),
        &conf
    );
    engine_player1.set_id_number("white");
    let engine_player1_dispatcher =
        dispatcher::EngineDispatcher::new(Arc::new(engine_player1), debug_actor_opt.clone(), None);
    //let mut engine_player2 = dummy::EngineDummy::new(debug_actor_opt.clone());
    //let mut engine_player2 = engine_mat::EngineMat::new(
    //let mut engine_player2 = engine_alphabeta::EngineAlphaBeta::new(
    let mut engine_player2 = engine_iddfs::EngineIddfs::new(
        debug_actor_opt.clone(),
        game_manager.zobrist_table(),
        &conf,
    );
    engine_player2.set_id_number("black");
    let engine_player2_dispatcher =
        dispatcher::EngineDispatcher::new(Arc::new(engine_player2), debug_actor_opt.clone(), None);
    let player1 = player::Player::Human {
        engine_opt: Some(engine_player1_dispatcher.start()),
    };
    let player2 = player::Player::Computer {
        engine: engine_player2_dispatcher.start(),
    };
    let players = player::Players::new(player1, player2);
    game_manager.set_players(players);
    let game_manager_actor = game_manager.start();
    BuildParams {
        game_manager_actor,
        debug_actor_opt,
        stat_actor_opt,
        stdin,
    }
}

async fn uci_mode(
    game_manager_actor: &game_manager::GameManagerActor,
    debug_actor_opt: Option<debug::DebugActor>,
    stat_actor_opt: Option<actix::Addr<stat_entity::StatEntity>>,
    stdin: &mut Arc<Mutex<io::Stdin>>,
) {
    let uci_reader = Box::new(uci_entity::UciReadWrapper::new(stdin.clone()));
    let uci_entity = uci_entity::UciEntity::new(
        uci_reader,
        game_manager_actor.clone(),
        debug_actor_opt.clone(),
        stat_actor_opt.clone(),
    );
    let uci_entity_actor = uci_entity.start();
    println!("Entering in uci mode");
    //fen();
    //test(&game_actor).await;
    println!("Enter an uci command:");

    let (tx, mut rx) = mpsc::unbounded_channel();

    // Spawn a task to handle async stdin reading
    tokio::spawn(async move {
        let stdin = tokio::io::stdin(); // Async stdin
        let mut reader = tokio::io::BufReader::new(stdin).lines(); // Buffered line reader

        while let Ok(Some(line)) = reader.next_line().await {
            if tx.send(line).is_err() {
                break; // Exit if the receiver is dropped
            }
        }
    });

    // Main loop
    loop {
        tokio::select! {
            // Handle user input
            Some(input) = rx.recv() => {
                //println!("Received input: {}", input);
                let _r = uci_entity_actor
                    .send(uci_entity::handler_read::ParseUserInput(input))
                    .await
                    .expect("Actix error");
            }
            // Handle timeout
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                println!("");
            }
        }
    }
}

/// Application CLI pour gérer différents modes
#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Commandes spécifiques
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Mode humain (TUI)
    Human,
    /// Benchmark de performance
    Benchmark,
}

#[actix::main]
async fn main() {
    init_trace();
    trace_build_info();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("Erreur de parsing des arguments: {e}");
            std::process::exit(1);
        }
    };
    match cli.command {
        Some(Command::Human) => {
            println!("Entering in tui mode");
            let BuildParams {
                game_manager_actor,
                debug_actor_opt: _,
                stat_actor_opt: _,
                mut stdin,
            } = init_game_params();
            tui_loop(&game_manager_actor, &mut stdin).await;
        }
        Some(Command::Benchmark) => {
            benchmark::launcher::benchmark("epd").unwrap();
        }
        None => {
            let BuildParams {
                game_manager_actor,
                debug_actor_opt,
                stat_actor_opt,
                mut stdin,
            } = init_game_params();
            uci_mode(
                &game_manager_actor,
                debug_actor_opt,
                stat_actor_opt,
                &mut stdin,
            )
            .await;
        }
    }
}
