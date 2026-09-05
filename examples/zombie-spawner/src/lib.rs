use worldvm_sdk::prelude::*;

worldvm_sdk::export_entrypoint!(handle_event);

fn handle_event(event_name: &str, _payload: &[u8]) {
    if event_name == "round_start" {
        // Spawn 3 hostile zombie NPCs
        let _ = world::spawn("zombie", -10.0, 0.0, 5.0);
        let _ = world::spawn("zombie", 0.0, 0.0, 15.0);
        let _ = world::spawn("zombie", 12.0, 0.0, -8.0);

        // Notify players
        let _ = ui::notify("player_1", "WARNING: 3 Zombies spawned in the Arena!", 5.0);
    }
}
