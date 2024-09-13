use toml::Table;
use tokio::net::TcpListener;
use tokio::time::sleep;
use std::time::Duration;
use sqlite:: Connection;
use clock;
use tokio::io::{self,AsyncReadExt,AsyncWriteExt};

use crate::wind::Wind;

mod stats;
mod wind;

struct Listener {
    port : u16,
    db_connection : Option<Connection>
}

impl Listener {

    pub fn attach_db(&mut self, db_file : &str) {
        self.db_connection = Some(sqlite::open(db_file).unwrap());
    }

    pub async fn task(&mut self) -> io::Result<()> {

        let sock_addr = "0.0.0.0:8080";

        // Next up we create a TCP listener which will listen for incoming
        // connections. This TCP listener is bound to the address we determined
        // above and must be associated with an event loop.
        let listener = TcpListener::bind(&sock_addr).await?;
        println!("Listening on: {}", sock_addr);

        loop {
            // Asynchronously wait for an inbound socket.
            let (mut socket, _) = listener.accept().await?;

            let mut buf = vec![0; 1024];

            // In a loop, read data from the socket and write the data back.
            loop {
                let n = socket
                    .read(&mut buf)
                    .await
                    .expect("failed to read data from socket");

                if n == 0 {
                    break;
                }

                let query = "select * from Outdoor where unix_time > 1726258500;";

                println!("Rcv'd {:?}", buf);
                socket
                    .write_all(&buf[0..n])
                    .await
                    .expect("failed to write data to socket");
            }
        }
        Ok(())
    }
}


static mut G_LISTENER : Listener = Listener {
    port : 8080,
    db_connection : None
};


static mut G_WIND : Wind = Wind {
    dev_name : String::new(),
    speed : stats::Accumulated::new()
};


async fn wait_tick(ticker : &clock::Clock) -> Result<(), ()> {
     let delay = ticker.secs_to_next_tick();
     sleep(Duration::from_secs(delay.into())).await;
     Ok(())
}


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
    let db_connection = sqlite::open(db_file).unwrap();

    let db_table = config["outdoor"]["db_table"].as_str().unwrap();
    println!("Creating/using db table {}", db_table);

    let query = format!("CREATE TABLE IF NOT EXISTS {} (unix_time INT NOT NULL, max REAL, ave REAL, min REAL, PRIMARY KEY(unix_time));", db_table);
    db_connection.execute(query).unwrap();

    let dev_name = config["outdoor"]["wind_dev"].as_str().unwrap();
    println!("Reading from {} for wind speeds", dev_name);

    unsafe {
        G_WIND.init(dev_name);
    }

    let period = config["common"]["sample_period_in_mins"].as_integer().unwrap() as i32;
    let ticker = clock::Clock::new(period * 60);

    let rt = tokio::runtime::Runtime::new().unwrap();

    let _ = unsafe {
        rt.spawn(G_WIND.task())
    };

    unsafe {
        G_LISTENER.attach_db(db_file);
    };

    let _ = unsafe {
        rt.spawn(G_LISTENER.task())
    };

    loop {
        rt.block_on(wait_tick(&ticker)).unwrap();
        println!("Tick");
        let measurement = unsafe {
            G_WIND.sample(&ticker)
        };
        let query = measurement.sql_insert_cmd("outdoor");
        db_connection.execute(query).unwrap();
    }
}
