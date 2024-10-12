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
    StartEngine,
    Moves(Vec<String>),
    Quit,
    SearchMoves(Vec<String>),
    SearchInfinite,
    StartPos,
    StopEngine,
    MaxTimePerMoveInMs(u32),
    Write(String),
    WriteDebug(String),
    Wtime(u64),
    WtimeInc(u64),
}
