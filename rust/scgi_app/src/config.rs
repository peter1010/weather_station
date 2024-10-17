use std::path::Path;
use toml::Table;

//----------------------------------------------------------------------------------------------------------------------------------
pub struct Config {
    config : Table
}


//----------------------------------------------------------------------------------------------------------------------------------
impl Config {

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn new() -> Self {
        let path = Path::new("weather.toml");
        let config_str = match std::fs::read_to_string(path) {
            Ok(f) => f,
            Err(e) => panic!("Failed to read config file {}", e)
        };

        let config = config_str.parse().unwrap();
//      dbg!(&config);

        Self {
            config
        }
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_host(&self, name :&str) -> &str {
        self.config[name]["host"].as_str().unwrap()
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_port(&self) -> u16 {
        self.config["common"]["port"].as_integer().unwrap() as u16
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_database(&self, name :&str) -> (&str, &str) {
        let db_file = self.config[name]["database"].as_str().unwrap();
        let db_table = self.config[name]["db_table"].as_str().unwrap();
        (db_file, db_table)
    }
}


