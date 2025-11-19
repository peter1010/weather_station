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
            Ok(config_str) => config_str,
            Err(error) => panic!("Failed to read {:#?} {}", path, error)
        };

        let config = match config_str.parse() {
            Ok(cfg) => cfg,
            Err(error) => panic!("Config file error\n{}", error)
        };

//      dbg!(&config);

        Self { config }
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_host(&self, name :&str) -> Option<&str> {
        let host = self.config[name]["host"].as_str();
        if host.is_none() {
            println!("No host specified for {} in config file", name)
        }
        host
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_port(&self) -> u16 {
        match self.config["common"]["port"].as_integer() {
            Some(port) => port as u16,
            None => panic!("No port specified in config file common section")
        }
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_database(&self, name :&str) -> (&str, &str) {
        let db_file = match self.config[name]["database"].as_str() {
            Some(db_file) => db_file,
            None => panic!("No database specified for {} in config file", name)
        };
        let db_table = match self.config[name]["db_table"].as_str() {
            Some(db_table) => db_table,
            None => panic!("No db_table specified for {} in config file", name)
        };
        (db_file, db_table)
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_sock_name(&self) -> Option<&str> {
        let sock_name = self.config["scgi"]["sock_name"].as_str();
        if sock_name.is_none() {
            println!("No SCGI sock name specified");
        }
        sock_name
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_sample_period(&self) -> u32 {
        match self.config["common"]["sample_period_in_mins"].as_integer() {
            Some(period) => period as u32,
            None => panic!("No Sample period specified in config file")
        }
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_dev_name(&self, name : &str) -> &str {
        match self.config[name]["temp_dev"].as_str() {
            Some(dev) => dev,
            None => panic!("No dev specified for {} in config file", name)
        }
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_wind_dev_name(&self) -> &str {
        match self.config["outdoor"]["wind_dev"].as_str() {
            Some(dev) => dev,
            None => panic!("No wind dev specified for outdoor in config file")
        }
    }
}

