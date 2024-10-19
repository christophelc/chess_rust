use crate::monitoring::debug;
use actix::Message;

#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub enum Event {
    Btime(u64),
    BtimeInc(u64),
    DebugMode(Option<debug::DebugActor>),
    Depth(u32),
    Fen(String),
    MaxTimePerMoveInMs(u32),
    Moves(Vec<String>),
    Quit,
    SearchInfinite,
    SearchMoves(Vec<String>),
    StartEngineThinking,
    StartPos,
    StopEngine,
    Write(String),
    WriteDebug(String),
    Wtime(u64),
    WtimeInc(u64),
}
