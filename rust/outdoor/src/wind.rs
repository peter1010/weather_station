use crate::stats;
use std::sync::{Arc, Mutex};
use std::thread;
use std::fs::File;
use std::io::{BufRead, BufReader};
use weather_err::Result;

//----------------------------------------------------------------------------------------------------------------------------------
pub struct Wind {
    pub speed : Arc<Mutex<stats::Accumulated>>,
    pub dev_name : String
}

//----------------------------------------------------------------------------------------------------------------------------------
impl Wind {

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn new(dev_name : &str) -> Self {
        Self {
            dev_name : dev_name.to_string(),
            speed : Arc::new(Mutex::new(stats::Accumulated::new()))
        }
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn start(&self) {
        let dev_name = self.dev_name.clone();
        let speed = self.speed.clone();

        thread::spawn(move || { 
            let _ = Self::task(dev_name, speed);
        });
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn sample(&self) -> stats::Summary {
        let mut data = self.speed.lock().expect("Unexpected failure to lock mutex");
        (*data).sample()
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn task(dev_name: String, speed: Arc<Mutex<stats::Accumulated>>) -> Result<()> {
        let f = File::open(&dev_name)?;
        let mut reader = BufReader::new(f);

        loop {
            let mut buffer = String::new();
            reader.read_line(&mut buffer)?;

            match buffer.trim().parse::<f32>() {
                Ok(value) => {
                    let mut data = speed.lock().expect("Unexpected failure to lock mutex");
                   (*data).add(value);
                },
                Err(..) => ()
            };
        }
    }
}
