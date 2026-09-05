use worldvm_sdk::prelude::*;

worldvm_sdk::export_entrypoint!(handle_event);

fn handle_event(event_name: &str, payload: &[u8]) {
    match event_name {
        "round_start" => {
            // Set lunar gravity (2.40 m/s2 instead of default 9.81)
            let _ = world::set_gravity(2.40);
        }
        "player_join" => {
            if let Ok(player) = deserialize_payload::<PlayerJoinPayload>(payload) {
                let _ = ui::notify(
                    &player.player_id,
                    "Low Gravity Mode Active: Float high!",
                    4.0,
                );
            }
        }
        _ => {}
    }
}
