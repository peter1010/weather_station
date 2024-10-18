use std::path::Path;
use toml::Table;
use weather_err::Result;

//----------------------------------------------------------------------------------------------------------------------------------
pub struct Config {
    config : Table
}


//----------------------------------------------------------------------------------------------------------------------------------
impl Config {

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn new() -> Result<Self> {
        let path = Path::new("weather.toml");
        let config_str = match std::fs::read_to_string(path) {
            Ok(f) => f,
            Err(e) => panic!("Failed to read config file {}", e)
        };

        let config = config_str.parse()?;
//      dbg!(&config);

        Ok(Self {
            config
        })
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_host(&self, name :&str) -> Option<&str> {
        Some(self.config[name]["host"].as_str()?)
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_port(&self) -> Option<u16> {
        Some(self.config["common"]["port"].as_integer()? as u16)
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_database(&self, name :&str) -> Option<(&str, &str)> {
        let db_file = self.config[name]["database"].as_str()?;
        let db_table = self.config[name]["db_table"].as_str()?;
        Some((db_file, db_table))
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_sock_name(&self) -> Option<&str> {
        Some(self.config["scgi"]["sock_name"].as_str()?)
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_sample_period(&self) -> Option<i32> {
        Some(self.config["common"]["sample_period_in_mins"].as_integer()? as i32)
    }
}


