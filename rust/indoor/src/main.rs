use toml::Table;
use bme688::Bme688;
use sqlite;


fn main() {

    let path = std::path::Path::new("weather.toml");
    let config_str = match std::fs::read_to_string(path) {
        Ok(f) => f,
        Err(e) => panic!("Failed to read config file {}", e)
    };

    let config: Table = config_str.parse().unwrap();
    dbg!(&config);

    let dev_name = config["indoor"]["dev"].as_str().unwrap();

    let mut sensor =  Bme688::new(dev_name).unwrap();

    let db_name = config["indoor"]["database"].as_str().unwrap();
    let db_table = config["indoor"]["db_table"].as_str().unwrap();

    let db_connection = sqlite::open(db_name).unwrap();

    let query = format!("CREATE TABLE IF NOT EXISTS {} (unix_time INT NOT NULL, temperature REAL, humidity REAL, pressure REAL, PRIMARY KEY(unix_time));", db_table);

    db_connection.execute(query).unwrap();

    sensor.cache_params();

    sensor.set_humdity_oversampling(16);
    sensor.set_pressure_oversampling(16);
    sensor.set_temperature_oversampling(16);

    sensor.force();

    let temp = sensor.read_temp(0);
    let temp = sensor.read_press(0);
    let temp = sensor.read_humd(0);
}
