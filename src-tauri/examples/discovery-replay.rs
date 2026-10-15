fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = include_str!("../fixtures/discovery-lifecycle.json");
    for snapshot in obtainer_lib::discovery::replay_fixture(json)? {
        println!("{}", serde_json::to_string(&snapshot)?);
    }
    Ok(())
}
