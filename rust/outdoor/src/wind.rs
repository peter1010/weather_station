use tokio::fs::File;
use tokio::io::{self, AsyncBufReadExt};
use crate::stats;
use std::sync::Mutex;

use weather_err::Result;

//----------------------------------------------------------------------------------------------------------------------------------
pub struct Wind {
    pub speed : Mutex<stats::Accumulated>,
    pub dev_name : String
}

//----------------------------------------------------------------------------------------------------------------------------------
impl Wind {

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn new(dev_name : &str) -> Wind {
        Wind {
            dev_name : dev_name.to_string(),
            speed : Mutex::new(stats::Accumulated::new())
        }
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn process(&self, speed : f32) -> Result<()> {
        let mut data = self.speed.lock()?;
        (*data).add(speed);
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn sample(&self) -> Result<stats::Summary> {
        let mut data =self.speed.lock()?;
        Ok((*data).sample())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub async fn task(&self) -> Result<()> {
        let f = File::open(&self.dev_name).await?;
        let mut reader = io::BufReader::new(f);

        loop {
            let mut buffer = String::new();
            reader.read_line(&mut buffer).await?;

            match buffer.trim().parse::<f32>() {
                Ok(value) => self.process(value)?,
                Err(..) => ()
            };
        }
    }
}
