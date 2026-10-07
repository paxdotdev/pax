//! Test transport for executing the real Web DTO/shared Rust policy from Node.
#[path = "../src/surface_scheduling.rs"]
mod surface_scheduling;

use std::io::Read;

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let snapshots: Vec<serde_json::Value> = serde_json::from_str(&input).unwrap();
    let mut activity = Default::default();
    let mut latency = Default::default();
    let mut prepared = None;
    let decisions: Vec<_> = snapshots
        .into_iter()
        .map(|snapshot| {
            if snapshot.get("viewport").is_some() {
                let next = surface_scheduling::prepare(
                    serde_json::from_value(snapshot).unwrap(),
                    &mut activity,
                    &mut latency,
                )
                .unwrap();
                let value = serde_json::to_value(next.summaries()).unwrap();
                prepared = Some(next);
                value
            } else {
                serde_json::to_value(
                    surface_scheduling::schedule(
                        serde_json::from_value(snapshot).unwrap(),
                        &mut activity,
                        prepared.as_ref(),
                    )
                    .unwrap(),
                )
                .unwrap()
            }
        })
        .collect();
    println!("{}", serde_json::to_string(&decisions).unwrap());
}
