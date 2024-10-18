///
/// Reading of Outdoor sensors
///

use toml::Table;
use tokio::time::sleep;
use tokio::runtime::Runtime;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use chrono::DateTime;

use clock;
use listener::Listener;

use crate::wind::Wind;
use sht31::{self, Sht31};

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
    clock::Clock::new(period_mins * 60).unwrap()
}


//----------------------------------------------------------------------------------------------------------------------------------
/// Create a database connection
fn create_db_connection(config : &Table)-> (Connection, String) {

    let db_file = config["outdoor"]["database"].as_str().unwrap();
    println!("Opening database {}", db_file);
    let db_connection = Arc::new(Mutex::new(sqlite::open(db_file).unwrap()));

    let db_table = config["outdoor"]["db_table"].as_str().unwrap();
    println!("Creating/using db table {}", db_table);

    let query = format!("CREATE TABLE IF NOT EXISTS {} (unix_time INT NOT NULL,
            max_speed REAL, ave_speed REAL, min_speed REAL,
            temperature REAL, humidity REAL, precipitation REAL, solar REAL,
            PRIMARY KEY(unix_time));", db_table);
    {
        let conn = db_connection.lock().unwrap();
        (*conn).execute(query).unwrap();
    }
    (db_connection, String::from(db_table))
}

//----------------------------------------------------------------------------------------------------------------------------------
fn send_to_database(db_connection : &Connection, db_table : &str, unix_time : i64, wind : stats::Summary, temp : sht31::Summary) {
    let dt = DateTime::from_timestamp(unix_time, 0).expect("invalid timestamp");
    println!("{} {} {}", dt, wind, temp);

    let query = format!("INSERT INTO {} VALUES ({},{},{},{},{},{},0.0,0.0);",
            db_table, unix_time, wind.get_max(), wind.get_average(), wind.get_min(),
            temp.get_temperature(), temp.get_humidity());
    println!("{}", query);

    {
        let conn = db_connection.lock().unwrap();
        (*conn).execute(query).unwrap();
    }
}

//----------------------------------------------------------------------------------------------------------------------------------
fn create_wind_sensor(config : &Table) -> Arc<Wind> {

    let dev_name = config["outdoor"]["wind_dev"].as_str().unwrap();
    println!("Reading from {} for wind speeds", dev_name);

    Arc::new(Wind::new(dev_name))
}


//----------------------------------------------------------------------------------------------------------------------------------
fn create_temp_sensor(config : &Table) -> Sht31 {

    let dev_name = config["outdoor"]["temp_dev"].as_str().unwrap();
    println!("Reading from {} for temp/humidity speeds", dev_name);

    Sht31::new(dev_name).unwrap()
}


//----------------------------------------------------------------------------------------------------------------------------------
async fn read_temp(sensor : &mut Sht31) -> sht31::Summary {
    // Start sample..
    sensor.one_shot().unwrap();
    sleep(Duration::from_secs(1)).await;
    sensor.sample().unwrap()
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

    let wind = create_wind_sensor(&config);
    let mut temp = create_temp_sensor(&config);

    let ticker = create_ticker(&config);

    let rt = Runtime::new().unwrap();

    let task_data = wind.clone();

    rt.spawn(async move { task_data.task().await });

    launch_listener(&config, &rt, db_connection.clone());

    loop {
        rt.block_on(wait_tick(&ticker)).unwrap();
        println!("Tick");
        let unix_time = ticker.get_nearest_tick();

        let wind_measurement = wind.sample().unwrap();

        // Start sample..
        let temp_measurement = rt.block_on(read_temp(&mut temp));

        send_to_database(&db_connection, &db_table, unix_time, wind_measurement, temp_measurement);
    }
}
