use worldvm_sdk::prelude::*;

worldvm_sdk::export_entrypoint!(handle_event);

fn handle_event(event_name: &str, payload: &[u8]) {
    if event_name == "player_join" {
        if let Ok(player) = deserialize_payload::<PlayerJoinPayload>(payload) {
            let msg = format!("Welcome to WorldVM Arena, {}!", player.player_name);
            let _ = ui::notify(&player.player_id, &msg, 3.5);
        }
    }
}
