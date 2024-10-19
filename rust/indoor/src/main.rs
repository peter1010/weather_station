use toml::Table;
use tokio::runtime::Runtime;
use tokio::time::sleep;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use sqlite;

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
fn launch_listener(config : &Table, rt : &Runtime, db_connection : Connection)
{
    let port = config["common"]["port"].as_integer().unwrap() as u16;
    let mut listener = Listener::new(port, db_connection);

    rt.spawn(async move { listener.task().await });
}


//----------------------------------------------------------------------------------------------------------------------------------
fn create_sensor(config : &Table) -> Result<bme688::Bme688> {

    let dev_name = config["indoor"]["dev"].as_str().unwrap();
    println!("Reading from {} for indoor sensor", dev_name);

    let mut sensor = bme688::Bme688::new(dev_name)?;

    sensor.cache_params()?;

    sensor.set_humdity_oversampling(16)?;
    sensor.set_pressure_oversampling(16)?;
    sensor.set_temperature_oversampling(16)?;

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
fn create_ticker(config : &Table) -> clock::Clock {
    let period = config["common"]["sample_period_in_mins"].as_integer().unwrap() as u32;
    clock::Clock::new(period * 60).unwrap()
}


//----------------------------------------------------------------------------------------------------------------------------------
fn create_db_connection(config : &Table)-> (Connection, String) {

    let db_file = config["indoor"]["database"].as_str().unwrap();
    println!("Opening database {}", db_file);
    let db_connection = Arc::new(Mutex::new(sqlite::open(db_file).unwrap()));

    let db_table = config["indoor"]["db_table"].as_str().unwrap();
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

    let path = std::path::Path::new("weather.toml");
    let config_str = match std::fs::read_to_string(path) {
        Ok(f) => f,
        Err(e) => panic!("Failed to read config file {}", e)
    };

    let config: Table = config_str.parse().unwrap();
    // dbg!(&config);

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
