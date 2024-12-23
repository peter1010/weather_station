use tokio::runtime::Runtime;
use tokio::time::sleep;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use sqlite;
use config;

use bme688;
use clock;
use listener::Listener;
use weather_err::Result;

type Connection = Arc<Mutex<sqlite::Connection>>;

//----------------------------------------------------------------------------------------------------------------------------------
async fn wait_tick(ticker : &clock::Clock) -> Result<()> {
     let delay_seconds = ticker.secs_to_next_tick();
     sleep(Duration::from_secs(delay_seconds.into())).await;
     Ok(())
}


//----------------------------------------------------------------------------------------------------------------------------------
fn launch_listener(config : &config::Config, rt : &Runtime, db_connection : Connection)
{
    let mut listener = Listener::new(config.get_port(), db_connection);

    rt.spawn(async move { listener.task().await });
}


//----------------------------------------------------------------------------------------------------------------------------------
fn create_sensor(config : &config::Config) -> Result<bme688::Bme688> {

    let mut sensor = bme688::Bme688::new(config.get_dev_name("indoor"));

    sensor.cache_params()?;

    sensor.set_humdity_oversampling(16);
    sensor.set_pressure_oversampling(16);
    sensor.set_temperature_oversampling(16);

    Ok(sensor)
}


//----------------------------------------------------------------------------------------------------------------------------------
async fn read_sensor(sensor : &mut bme688::Bme688) -> bme688::Summary {
    // Start sample..
    sensor.one_shot().unwrap();
    loop {
        sleep(Duration::from_secs(1)).await;
        if sensor.is_ready().unwrap() {
            break;
        }
    }
    sensor.sample().unwrap()
}



//----------------------------------------------------------------------------------------------------------------------------------
fn create_ticker(config : &config::Config) -> clock::Clock {
    clock::Clock::new(config.get_sample_period() * 60)
}


//----------------------------------------------------------------------------------------------------------------------------------
fn create_db_connection(config : &config::Config)-> (Connection, String) {

    let (db_file, db_table) = config.get_database("indoor");

    println!("Opening database {}", db_file);
    let db_connection = Arc::new(Mutex::new(sqlite::open(db_file).unwrap()));

    println!("Creating/using db table {}", db_table);

    let query = format!("CREATE TABLE IF NOT EXISTS {} (unix_time INT NOT NULL,
            temperature REAL, humidity REAL, pressure REAL, PRIMARY KEY(unix_time));", db_table);

    {
        let conn = db_connection.lock().unwrap();
        (*conn).execute(query).unwrap();
    }
    (db_connection, String::from(db_table))
}


//----------------------------------------------------------------------------------------------------------------------------------
fn main() {

    let config = config::Config::new();

    let mut sensor = create_sensor(&config).unwrap();

    let (db_connection, db_table) = create_db_connection(&config);

    let ticker = create_ticker(&config);

    let rt = Runtime::new().unwrap();

    launch_listener(&config, &rt, db_connection.clone());

    loop {
        rt.block_on(wait_tick(&ticker)).unwrap();
        println!("Tick");
        let unix_time = ticker.get_nearest_tick();

        let measurement = rt.block_on(read_sensor(&mut sensor));
        println!("{}", measurement);

        let temp = measurement.get_temperature();
        let humd = measurement.get_humidity();
        let press = measurement.get_pressure();

        let query = format!("INSERT INTO {} VALUES ({},{},{},{});", db_table, unix_time, temp, humd, press);

        {
            let conn = db_connection.lock().unwrap();
            (*conn).execute(query).unwrap();
        }
    }
}
