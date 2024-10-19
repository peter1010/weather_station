use std::os::unix::net::UnixListener;
use std::io::{BufRead, Read, Write};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::sleep;
use std::time::Duration;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::io::{BufReader, AsyncWriteExt, AsyncBufReadExt, AsyncBufRead};
use crate::scgi::Listener;
use std::fs::remove_file;

use clock;

mod scgi;

use crate::sensor::Sensor;
mod sensor;
use config;
mod drop_privs;

type Connection = Arc<Mutex<sqlite::Connection>>;

//const SOCK_USER : &str = "http";
const SOCK_USER : &str = "lighttpd";
//const SOCK_GROUP : &str = "http";
const SOCK_GROUP : &str = "lighttpd";


//fn create_socket() -> UnixListener {
//
//    let server = UnixListener::bind(SOCK_NAME).unwrap();
//    drop_privs(SOCK_USER, SOCK_GROUP);
//    return server
//}

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

    let mut indoor_sensor = rt.block_on(Sensor::new(&config, "indoor")).unwrap();

    let mut outdoor_sensor = rt.block_on(Sensor::new(&config, "outdoor")).unwrap();

    rt.block_on(indoor_sensor.collect()).unwrap();
    rt.block_on(outdoor_sensor.collect()).unwrap();

    let sock_name = config.get_sock_name().unwrap();
    let _ = remove_file(sock_name);

//    let mut listener = Listener::new(sock_name, &indoor_sensor, &outdoor_sensor);

//    rt.spawn(async move { listener.task().await });

    let ticker = clock::Clock::new(config.get_sample_period() * 60);

    loop {
        rt.block_on(wait_tick(&ticker)).unwrap();
        println!("Tick");
        rt.block_on(indoor_sensor.collect()).unwrap();
        rt.block_on(outdoor_sensor.collect()).unwrap();
    }
}
