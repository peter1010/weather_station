use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::time::sleep;
use std::time::Duration;

use crate::sensor::Sensor;

use clock;
use config;

mod sensor;


//----------------------------------------------------------------------------------------------------------------------------------
async fn wait_tick(ticker : &clock::Clock) -> Result<(), ()> {
     let delay = ticker.secs_to_next_tick() + 60;
     sleep(Duration::from_secs(delay.into())).await;
     Ok(())
}


//----------------------------------------------------------------------------------------------------------------------------------
fn main() {
    let config = config::Config::new();

    let rt = Runtime::new().unwrap();

    let indoor_sensor = Arc::new(rt.block_on(Sensor::new(&config, "indoor")).unwrap());
    let outdoor_sensor = Arc::new(rt.block_on(Sensor::new(&config, "outdoor")).unwrap());

    rt.block_on(indoor_sensor.collect()).unwrap();
    rt.block_on(outdoor_sensor.collect()).unwrap();

    let ticker = clock::Clock::new(config.get_sample_period() * 60);

    loop {
        rt.block_on(wait_tick(&ticker)).unwrap();
        println!("Tick");
        rt.block_on(indoor_sensor.collect()).unwrap();
        rt.block_on(outdoor_sensor.collect()).unwrap();
    }
}
