use actix::{Handler, Message};

use crate::entity::engine::component::engine_logic as logic;
use crate::{entity::stat::component::stat_data, monitoring::debug};

use super::StatEntity;

const STAT_FILE_PATH: &str = "stat.txt";

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct StatInit(pub logic::EngineId);
impl Handler<StatInit> for StatEntity {
    type Result = ();

    fn handle(&mut self, msg: StatInit, _ctx: &mut Self::Context) -> Self::Result {
        let engine_id = msg.0;
        self.stats.insert(engine_id, stat_data::StatData::new(0));
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct StatInc {
    engine_id: logic::EngineId,
    inc_n_position_evaluted: u64,
}

impl Handler<StatInc> for StatEntity {
    type Result = ();

    fn handle(&mut self, msg: StatInc, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!("Stat actor receive {:?}", msg)));
        }
        let (engine_id, inc) = (msg.engine_id, msg.inc_n_position_evaluted);
        let data_opt = self.stats.get_mut(&engine_id);
        match data_opt {
            Some(data) => data.inc(inc),
            None => {
                self.stats.insert(engine_id, stat_data::StatData::new(0));
            }
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct StatUpdate {
    engine_id: logic::EngineId,
    n_position_evaluted: u64,
}
impl StatUpdate {
    pub fn new(engine_id: logic::EngineId, n_position_evaluted: u64) -> Self {
        Self {
            engine_id,
            n_position_evaluted,
        }
    }
}

impl Handler<StatUpdate> for StatEntity {
    type Result = ();

    fn handle(&mut self, msg: StatUpdate, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!("Stat actor receive {:?}", msg)));
        }
        let (engine_id, n) = (msg.engine_id, msg.n_position_evaluted);
        let data_opt = self.stats.get_mut(&engine_id);
        match data_opt {
            Some(data) => data.set(n),
            None => {
                self.stats.insert(engine_id, stat_data::StatData::new(n));
            }
        };
    }
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct StatClose(logic::EngineId);
impl StatClose {
    pub fn new(engine_id: logic::EngineId) -> Self {
        Self(engine_id)
    }
}

impl Handler<StatClose> for StatEntity {
    type Result = ();

    fn handle(&mut self, msg: StatClose, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!("Stat actor receive {:?}", msg)));
        }
        let engine_id = msg.0;
        let data_opt = self.stats.get_mut(&engine_id);
        if let Some(data) = data_opt {
            data.close();
            write_stat(&engine_id, data);
        };
    }
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Option<stat_data::StatData>")]
pub struct GetStat(logic::EngineId);

impl Handler<GetStat> for StatEntity {
    type Result = Option<stat_data::StatData>;
    fn handle(&mut self, msg: GetStat, _ctx: &mut Self::Context) -> Self::Result {
        self.stats.get(&msg.0).cloned()
    }
}

use std::env;
use std::fs::OpenOptions;
use std::io::Write;

fn write_stat(engine_id: &logic::EngineId, stat: &stat_data::StatData) {
    if let Some(end_time) = stat.end_time() {
        let exe_path = env::current_exe().expect("Failed to find executable path");
        let folder_exe_path = exe_path
            .parent()
            .expect("Failed to get folder executable path");
        let n_position_evaluated = stat.n_positions_evaluated();
        let start_time = stat.start_time();
        let duration = end_time - start_time;
        // write stat to a file
        let path = format!("{}/{}", folder_exe_path.display(), STAT_FILE_PATH);
        let mut file = OpenOptions::new()
            .append(true) // Set append mode
            .create(true) // Create file if it doesn't exist
            .open(path)
            .expect("Failed to open or create file");
        let n_per_second = n_position_evaluated as f32 / duration.num_milliseconds() as f32;
        let data = format!(
            "{} id:'{}' n_positions_evluated/ms: {:.2} - total: {}\n",
            start_time,
            engine_id.name(),
            n_per_second,
            n_position_evaluated
        );
        // Attempt to write the string data to the file
        match file.write_all(data.as_bytes()) {
            Ok(_) => {}
            Err(e) => panic!("Failed to write data to file: {}", e),
        }
    }
}
