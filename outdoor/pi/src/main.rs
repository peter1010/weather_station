use toml::Table;
use tokio::fs::File;
use tokio::io::{self, BufReader, AsyncBufReadExt};
use tokio::time::sleep;
use chrono;
use chrono::{DateTime, Utc};
use chrono::Timelike;
use std::time::Duration;
use sqlite;

mod stats;

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

    fn sample(&mut self) -> stats::Summary {
        let result = stats::Summary::new(&self.speed, SAMPLE_PERIOD_IN_SECS);
        result.print();
        result
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
        Ok(())
    }
}

static mut g_wind : Wind = Wind {
    dev_name : String::new(),
    speed : stats::Accumulated::new()
};


struct Clock {
    end_time : DateTime<Utc>
}


async fn clock() -> Result<(), ()> {
     let now = chrono::Utc::now();
     let min = now.minute();
     let sec = now.second();

     let secs = (now.second() + 60 * now.minute()) as i32;
     let delay = (SAMPLE_PERIOD_IN_SECS - secs % SAMPLE_PERIOD_IN_SECS) as u32;
     println!("Duration {}", delay);
     sleep(Duration::from_secs(delay.into())).await;
     Ok(())
}


fn main() -> Result<(), ()> {
    let path = std::path::Path::new("outdoor.toml");
    let config_str = match std::fs::read_to_string(path) {
        Ok(f) => f,
        Err(e) => panic!("Failed to read config file {}", e)
    };

    let config: Table = config_str.parse().unwrap();

    let dev_name = config["Wind"]["dev"].as_str().unwrap();

    unsafe {
        g_wind.init(dev_name);
    }

    println!("Hello, world!");
    dbg!(&config);

    let mut rt = tokio::runtime::Runtime::new().unwrap();


    unsafe {
        rt.spawn(g_wind.task());
    }
    loop {
        rt.block_on(clock());
        println!("Tick");
        unsafe {
            let _ = g_wind.sample();
        }
    }

    Ok(())
}
