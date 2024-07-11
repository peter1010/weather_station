use toml::Table;
use tokio::fs::File;
use tokio::io::{self, BufReader, AsyncBufReadExt};

struct Wind {
    max_speed : f32,
    min_speed : f32,
    sum : f64,
    num_of : u16,
    dev_name : String
}

impl Wind {
    pub fn new(dev_name : &str) -> Self {
        let tmp = Wind {
            max_speed : 0.0,
            min_speed : 0.0,
            sum : 0.0,
            num_of : 0,
            dev_name : dev_name.to_string()
        };
        tmp
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
        } else {
            self.max_speed = speed;
            self.min_speed = speed;
        }
        self.num_of += 1;
        self.sum += speed as f64;
        println!("{} {} {}", self.max_speed, self.sum / (self.num_of as f64), self.min_speed);
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


fn main() -> Result<(), ()> {
    let path = std::path::Path::new("outdoor.toml");
    let config_str = match std::fs::read_to_string(path) {
        Ok(f) => f,
        Err(e) => panic!("Failed to read config file {}", e)
    };

    let config: Table = config_str.parse().unwrap();

    let dev_name = config["Wind"]["dev"].as_str().unwrap();
    let mut wind = Wind::new(dev_name);

    println!("Hello, world!");
    dbg!(&config);

    let mut rt = tokio::runtime::Runtime::new().unwrap();

    let task = wind.task();
    rt.block_on(task);

    Ok(())
}
