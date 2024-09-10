use toml::Table;
use tokio::fs::File;
use tokio::io::{self, BufReader, AsyncBufReadExt};
use tokio::time::sleep;
use std::time::Duration;
use sqlite;

mod stats;
mod clock;

const SAMPLE_PERIOD_IN_MINS : i32 = 15;
const SAMPLE_PERIOD_IN_SECS : i32 = 60 * SAMPLE_PERIOD_IN_MINS;

struct Wind {
    speed : stats::Accumulated,
    dev_name : String
}



impl Wind {
    fn init(&mut self, dev_name : &str) {
        self.dev_name = dev_name.to_string();
        self.reset();
    }

    fn reset(&mut self) {
        self.speed.reset();
    }


    fn process(&mut self, speed : f32) {
        self.speed.add(speed);
    }

    fn sample(&mut self, ticker : &clock::Clock) -> stats::Summary {
        self.speed.sample(&ticker)
    }

    async fn task(&mut self) -> io::Result<()> {
        let f = File::open(&self.dev_name).await?;
        let mut reader = BufReader::new(f);

        loop {
            let mut buffer = String::new();
            reader.read_line(&mut buffer).await?;

            match buffer.trim().parse::<f32>() {
                Ok(value) => self.process(value),
                Err(..) => ()
            };
        }
    }
}

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

    let ticker = clock::Clock::new(SAMPLE_PERIOD_IN_SECS);

    let rt = tokio::runtime::Runtime::new().unwrap();

    let _ = unsafe {
        rt.spawn(G_WIND.task())
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
