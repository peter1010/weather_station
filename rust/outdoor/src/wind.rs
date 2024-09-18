use tokio::fs::File;
use tokio::io::{self, BufReader, AsyncBufReadExt};
use clock;
use crate::stats;
use std::sync::Mutex;

pub struct Wind {
    pub speed : Mutex<stats::Accumulated>,
    pub dev_name : String
}


impl Wind {
    pub fn new(dev_name : &str) -> Wind {
        Wind {
            dev_name : dev_name.to_string(),
            speed : Mutex::new(stats::Accumulated::new())
        }
    }


    fn process(&self, speed : f32) {
        let mut data = self.speed.lock().unwrap();
        (*data).add(speed);
    }


    pub fn sample(&self, ticker : &clock::Clock) -> stats::Summary {
        let mut data = self.speed.lock().unwrap();
        (*data).sample(&ticker)
    }


    pub async fn task(&self) -> io::Result<()> {
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
