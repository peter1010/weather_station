use toml::Table;
use tokio::fs::File;
use tokio::io::{self, BufReader, AsyncBufReadExt};
use tokio::time::sleep;
use chrono;
use chrono::{DateTime, Utc};
use chrono::Timelike;
use std::time::Duration;

struct Wind {
    max_speed : f32,
    min_speed : f32,
    sum : f64,
    num_of : u16,
    dev_name : String
}

impl Wind {
    fn init(&mut self, dev_name : &str) {
        self.dev_name = dev_name.to_string();
        self.reset();
    }

    fn reset(&mut self) {
        self.num_of = 0;
    }

    fn process(&mut self, speed : f32) {
        if self.num_of > 0 {
            if speed > self.max_speed {
                self.max_speed = speed;
            } else {
                if speed < self.min_speed {
                    self.min_speed = speed;
                }
            }
            self.num_of += 1;
            self.sum += speed as f64;
        } else {
            self.max_speed = speed;
            self.min_speed = speed;
            self.sum = speed as f64;
            self.num_of = 1;
        }
    }

    fn save(&mut self) {
        let now = chrono::Utc::now();
        println!("{} {} {} {}", now, self.max_speed, self.sum / (self.num_of as f64), self.min_speed);
        self.num_of = 0;
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
            max_speed : 0.0,
            min_speed : 0.0,
            sum : 0.0,
            num_of : 0,
            dev_name : String::new()
};


struct Clock {
    end_time : DateTime<Utc>
}


async fn clock() -> Result<(), ()> {
     let now = chrono::Utc::now();
     let min = now.minute();
     let sec = now.second();
     let mut delay = 60 - sec;
     if min < 15 {
         delay += (15 - min) * 60;
      } else if min < 30 {
         delay += (30 - min) * 60;
      } else if min < 45 {
         delay += (45 - min) * 60;
      } else {
         delay += (60 - min) * 60;
      }
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
            g_wind.save();
        }
    }

    Ok(())
}
