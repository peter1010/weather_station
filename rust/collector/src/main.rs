use std::sync::Arc;
use std::time::Duration;
use std::thread;

use crate::sensor::Sensor;

use clock;
use config;

mod sensor;


//----------------------------------------------------------------------------------------------------------------------------------
fn wait_tick(ticker : &clock::Clock) -> Result<(), ()> {
     let delay = ticker.secs_to_next_tick() + 60;
     thread::sleep(Duration::from_secs(delay.into()));
     Ok(())
}


//----------------------------------------------------------------------------------------------------------------------------------
fn main() {
    let config = config::Config::new();

    let indoor_sensor = Sensor::new(&config, "indoor").unwrap();
    let outdoor_sensor = Sensor::new(&config, "outdoor").unwrap();

    indoor_sensor.collect().unwrap();
    outdoor_sensor.collect().unwrap();

    let ticker = clock::Clock::new(config.get_sample_period() * 60);

    loop {
        wait_tick(&ticker).unwrap();
        println!("Tick");
        indoor_sensor.collect().unwrap();
        outdoor_sensor.collect().unwrap();
    }
}
