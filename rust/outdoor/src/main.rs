//!
//! Reading of Outdoor sensors
//!

use toml::Table;
use tokio::net::TcpListener;
use tokio::time::sleep;
use std::time::Duration;
use sqlite:: Connection;
use tokio::io::{self,AsyncBufReadExt,AsyncWriteExt, BufReader};
use std::sync::{Arc, Mutex};

use clock;

use crate::wind::Wind;

mod stats;
mod wind;

struct Listener {
    port : u16,
    db_connection : Arc<Mutex<Connection>>
}

impl Listener {

    pub fn new(port :u16, db_connection : Arc<Mutex<Connection>>) -> Listener {
        Listener {
            port,
            db_connection
        }
    }


    pub async fn task(&mut self) -> io::Result<()> {

        let sock_addr = format!("0.0.0.0:{}", self.port);

        // Next up we create a TCP listener which will listen for incoming
        // connections. This TCP listener is bound to the address we determined
        // above and must be associated with an event loop.
        let listener = TcpListener::bind(&sock_addr).await?;
        println!("Listening on: {}", sock_addr);

        loop {
            // Asynchronously wait for an inbound socket.
            let (socket, _) = listener.accept().await?;

            let mut stream = BufReader::new(socket);


            // In a loop, read data from the socket and write the data back.
            loop {
                let mut line = String::new();

                let n = stream
                    .read_line(&mut line)
                    .await
                    .expect("failed to read data from socket");

                if n == 0 {
                    break;
                }

                let unix_time = line.trim().parse::<i64>();
                if !unix_time.is_ok() {
                    continue;
                }
                let unix_time = unix_time.unwrap();

                println!("Rcv'd {:?}", unix_time);

                let mut response = String::new();

                let query = format!("select * from Outdoor where unix_time > {};", unix_time);
                {
                    let conn = self.db_connection.lock().unwrap();

                    let statement = (*conn).prepare(query).unwrap();

                    // unix_time INT, max REAL, ave REAL, min REAL


                    for row in statement
                        .into_iter()
                        .map(|row| row.unwrap())
                    {
                        response += &(format!("unix_time = {}", row.read::<i64, _>("unix_time")) + "\n");
                        response += &(format!("\tmax = {}", row.read::<f64, _>("max")) + "\n");
                        response += &(format!("\tave = {}", row.read::<f64, _>("ave")) + "\n");
                        response += &(format!("\tmin = {}", row.read::<f64, _>("min")) + "\n");
                    }
                }

                stream.write_all(response.as_bytes()).await
                    .expect("failed to write data to socket");
            }
        }
    }
}



async fn wait_tick(ticker : &clock::Clock) -> Result<(), ()> {
     let delay = ticker.secs_to_next_tick();
     sleep(Duration::from_secs(delay.into())).await;
     Ok(())
}

/// Application entry point
fn main() -> Result<(), ()> {
    let path = std::path::Path::new("weather.toml");
    let config_str = match std::fs::read_to_string(path) {
        Ok(f) => f,
        Err(e) => panic!("Failed to read config file {}", e)
    };

    let config: Table = config_str.parse().unwrap();
//    dbg!(&config);

    let db_file = config["outdoor"]["database"].as_str().unwrap();
    println!("Opening database {}", db_file);
    let db_connection = Arc::new(Mutex::new(sqlite::open(db_file).unwrap()));

    let db_table = config["outdoor"]["db_table"].as_str().unwrap();
    println!("Creating/using db table {}", db_table);

    let query = format!("CREATE TABLE IF NOT EXISTS {} (unix_time INT NOT NULL, max REAL, ave REAL, min REAL, PRIMARY KEY(unix_time));", db_table);
    {
        let conn = db_connection.lock().unwrap();
        (*conn).execute(query).unwrap();
    }

    let dev_name = config["outdoor"]["wind_dev"].as_str().unwrap();
    println!("Reading from {} for wind speeds", dev_name);

    let wind = Arc::new(Wind::new(dev_name));

    let period = config["common"]["sample_period_in_mins"].as_integer().unwrap() as i32;
    let ticker = clock::Clock::new(period * 60);

    let rt = tokio::runtime::Runtime::new().unwrap();

    let task_data = wind.clone();

    rt.spawn(async move { task_data.task().await });

    let port = config["common"]["port"].as_integer().unwrap() as u16;
    let mut listener = Listener::new(port, db_connection.clone());

    rt.spawn(async move { listener.task().await });


    loop {
        rt.block_on(wait_tick(&ticker)).unwrap();
        println!("Tick");
        let measurement = wind.sample(&ticker);
        let query = measurement.sql_insert_cmd("outdoor");
        {
            let conn = db_connection.lock().unwrap();
            (*conn).execute(query).unwrap();
        }
    }
}
