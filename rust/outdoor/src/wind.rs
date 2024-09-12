use tokio::fs::File;
use tokio::io::{self, BufReader, AsyncBufReadExt};
use clock;

use crate::stats;


pub struct Wind {
    pub speed : stats::Accumulated,
    pub dev_name : String
}


impl Wind {
    pub fn init(&mut self, dev_name : &str) {
        self.dev_name = dev_name.to_string();
        self.reset();
    }

    fn reset(&mut self) {
        self.speed.reset();
    }


    fn process(&mut self, speed : f32) {
        self.speed.add(speed);
    }

    pub fn sample(&mut self, ticker : &clock::Clock) -> stats::Summary {
        self.speed.sample(&ticker)
    }

    pub async fn task(&mut self) -> io::Result<()> {
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
