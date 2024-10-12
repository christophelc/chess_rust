pub mod actor;
pub mod component;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::actor::engine_dispatcher as dispatcher;

    use crate::entity::game::actor::{game_manager, game_manager::build_game_manager_actor};

    #[actix::test]
    async fn test_engine_dummy() {
        let inputs = vec!["position startpos", "go"];
        let game_manager_actor = build_game_manager_actor(inputs.clone()).await;
        let msg = game_manager::handler_engine::GetCurrentEngine::default();
        let result = game_manager_actor.send(msg).await;
        let mut vec_engine_id: Vec<String> = vec![];
        if let Ok(Some(engine_actor)) = result {
            let engine_id_opt = engine_actor.send(dispatcher::handler_engine::EngineGetId::default()).await;
            if let Ok(Some(engine_id)) = engine_id_opt {
                vec_engine_id.push(engine_id.name().to_string());
                vec_engine_id.push(engine_id.author().to_string());
            }
        }
        assert_eq!(vec_engine_id, vec!["Random engine", "Christophe le cam"])
    }

    #[actix::test]
    async fn test_engine_dummy_is_random() {
        let mut best_moves = Vec::new();
        let inputs = vec!["position startpos", "go", "wait100ms"];

        for _ in 0..10 {
            let game_manager_actor = build_game_manager_actor(inputs.clone()).await;
            let ts_best_move = game_manager_actor
                .send(game_manager::handler_game::GetBestMove)
                .await
                .expect("actix mailbox error") // Ensure no Actix mailbox error
                .expect("No best move found"); // Ensure a best move is found

            let best_move_str = ts_best_move.best_move().cast(); // Convert the best move to the desired format (if necessary)
            best_moves.push(best_move_str); // Add the best move to the Vec
            game_manager_actor
                .send(game_manager::handler_uci_command::UciCommand::CleanResources)
                .await
                .expect("actix mailbox error")
                .unwrap();
        }
        let unique_moves: HashSet<_> = best_moves.iter().cloned().collect();
        // ensure that we generate random moves
        assert!(unique_moves.len() > 1)
    }
}
