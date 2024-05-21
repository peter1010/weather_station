use toml::Table;

fn main() {
    let path = std::path::Path::new("outdoor.toml");
    let config_str = match std::fs::read_to_string(path) {
        Ok(f) => f,
        Err(e) => panic!("Failed to read config file {}", e)
    };

    let config: Table = config_str.parse().unwrap();


    println!("Hello, world!");
    dbg!(&config);
}
