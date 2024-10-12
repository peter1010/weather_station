///
/// Reading of Outdoor sensors
///

use toml::Table;
use tokio::time::sleep;
use tokio::runtime::Runtime;
use std::time::Duration;
use std::sync::{Arc, Mutex};

use clock;
use listener::Listener;

use crate::wind::Wind;

mod stats;
mod wind;

type Connection = Arc<Mutex<sqlite::Connection>>;

//----------------------------------------------------------------------------------------------------------------------------------
/// Aync wait for a tick event
async fn wait_tick(ticker : &clock::Clock) -> Result<(), ()> {
     let delay_secs = ticker.secs_to_next_tick();
     sleep(Duration::from_secs(delay_secs.into())).await;
     Ok(())
}


//----------------------------------------------------------------------------------------------------------------------------------
/// Create a ticker
fn create_ticker(config : &Table) -> clock::Clock {
    let period_mins = config["common"]["sample_period_in_mins"].as_integer().unwrap() as i32;
    clock::Clock::new(period_mins * 60)
}


//----------------------------------------------------------------------------------------------------------------------------------
/// Create a database connection
fn create_db_connection(config : &Table)-> (Connection, String) {

    let db_file = config["outdoor"]["database"].as_str().unwrap();
    println!("Opening database {}", db_file);
    let db_connection = Arc::new(Mutex::new(sqlite::open(db_file).unwrap()));

    let db_table = config["outdoor"]["db_table"].as_str().unwrap();
    println!("Creating/using db table {}", db_table);

    let query = format!("CREATE TABLE IF NOT EXISTS {} (unix_time INT NOT NULL, max_speed REAL, ave_speed REAL, min_speed REAL,
            PRIMARY KEY(unix_time));", db_table);
    {
        let conn = db_connection.lock().unwrap();
        (*conn).execute(query).unwrap();
    }
    (db_connection, String::from(db_table))
}


//----------------------------------------------------------------------------------------------------------------------------------
fn create_sensor(config : &Table) -> Arc<Wind> {

    let dev_name = config["outdoor"]["wind_dev"].as_str().unwrap();
    println!("Reading from {} for wind speeds", dev_name);

    Arc::new(Wind::new(dev_name))
}


//----------------------------------------------------------------------------------------------------------------------------------
fn launch_listener(config : &Table, rt : &Runtime, db_connection : Connection)
{
    let port = config["common"]["port"].as_integer().unwrap() as u16;
    let mut listener = Listener::new(port, db_connection);

    rt.spawn(async move { listener.task().await });
}


//----------------------------------------------------------------------------------------------------------------------------------
/// Application entry point
fn main() -> Result<(), ()> {
    let path = std::path::Path::new("weather.toml");
    let config_str = match std::fs::read_to_string(path) {
        Ok(f) => f,
        Err(e) => panic!("Failed to read config file {}", e)
    };

    let config: Table = config_str.parse().unwrap();
//    dbg!(&config);

    let (db_connection, db_table) = create_db_connection(&config);

    let wind = create_sensor(&config);

    let ticker = create_ticker(&config);

    let rt = Runtime::new().unwrap();

    let task_data = wind.clone();

    rt.spawn(async move { task_data.task().await });

    launch_listener(&config, &rt, db_connection.clone());

    loop {
        rt.block_on(wait_tick(&ticker)).unwrap();
        println!("Tick");
        let measurement = wind.sample(&ticker);
        let query = measurement.unwrap().sql_insert_cmd("outdoor");
        {
            let conn = db_connection.lock().unwrap();
            (*conn).execute(query).unwrap();
        }
    }
}
