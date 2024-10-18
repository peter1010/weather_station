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
    pub fn get_host(&self, name :&str) -> Result<&str> {
        Ok(self.config[name]["host"].as_str().ok_or("No Host specified")?)
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_port(&self) -> Result<u16> {
        Ok(self.config["common"]["port"].as_integer().ok_or("No Port specified")? as u16)
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_database(&self, name :&str) -> Result<(&str, &str)> {
        let db_file = self.config[name]["database"].as_str().ok_or("No DB  specified")?;
        let db_table = self.config[name]["db_table"].as_str().ok_or("No DB table specified")?;
        Ok((db_file, db_table))
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_sock_name(&self) -> Result<&str> {
        Ok(self.config["scgi"]["sock_name"].as_str().ok_or("No valid sock name specified")?)
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_sample_period(&self) -> Result<i32> {
        Ok(self.config["common"]["sample_period_in_mins"].as_integer().ok_or("No valid Sample period specified")? as i32)
    }
}


