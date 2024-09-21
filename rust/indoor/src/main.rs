use toml::Table;
use tokio::runtime::Runtime;
use tokio::time::sleep;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use sqlite;

use bme688::Bme688;
use clock;
use listener::Listener;

type Connection = Arc<Mutex<sqlite::Connection>>;

async fn wait_tick(ticker : &clock::Clock) -> Result<(), ()> {
     let delay = ticker.secs_to_next_tick();
     sleep(Duration::from_secs(delay.into())).await;
     Ok(())
}


fn launch_listener(config : &Table, rt : &Runtime, db_connection : Connection)
{
    let port = config["common"]["port"].as_integer().unwrap() as u16;
    let mut listener = Listener::new(port, db_connection);

    rt.spawn(async move { listener.task().await });
}


fn main() {

    let path = std::path::Path::new("weather.toml");
    let config_str = match std::fs::read_to_string(path) {
        Ok(f) => f,
        Err(e) => panic!("Failed to read config file {}", e)
    };

    let config: Table = config_str.parse().unwrap();
    // dbg!(&config);

    let dev_name = config["indoor"]["dev"].as_str().unwrap();
    println!("Reading from {} for indoor sensor", dev_name);

    let mut sensor =  Bme688::new(dev_name).unwrap();

    let db_file = config["indoor"]["database"].as_str().unwrap();
    println!("Opening database {}", db_file);
    let db_connection = Arc::new(Mutex::new(sqlite::open(db_file).unwrap()));

    let db_table = config["indoor"]["db_table"].as_str().unwrap();
    println!("Creating/using db table {}", db_table);

    let query = format!("CREATE TABLE IF NOT EXISTS {} (unix_time INT NOT NULL, temperature REAL, humidity REAL, pressure REAL, PRIMARY KEY(unix_time));", db_table);

    {
        let conn = db_connection.lock().unwrap();
        (*conn).execute(query).unwrap();
    }

    sensor.cache_params();

    sensor.set_humdity_oversampling(16);
    sensor.set_pressure_oversampling(16);
    sensor.set_temperature_oversampling(16);


    let period = config["common"]["sample_period_in_mins"].as_integer().unwrap() as i32;
    let ticker = clock::Clock::new(period * 60);

    let rt = Runtime::new().unwrap();

    launch_listener(&config, &rt, db_connection.clone());

    loop {
        rt.block_on(wait_tick(&ticker)).unwrap();
        println!("Tick");

        sensor.force();

        let temp = sensor.read_temp(0);
        let press = sensor.read_press(0);
        let humd = sensor.read_humd(0);
        let unix_time = ticker.get_nearest_tick();

        let query = format!("INSERT INTO {} VALUES ({},{},{},{});", db_table, unix_time, temp, humd, press);

        {
            let conn = db_connection.lock().unwrap();
            (*conn).execute(query).unwrap();
        }
    }
}
