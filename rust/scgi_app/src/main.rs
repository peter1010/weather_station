use libc::{getpwnam, getgrnam, getuid};
use std::ffi::CString;
use std::fs::remove_file;
use std::os::unix::fs::chown;
use std::os::unix::net::UnixListener;
use std::io::{BufRead, Read, Write};
use std::sync::{Arc, Mutex};
use toml::Table;
use tokio::runtime::Runtime;
use tokio::time::sleep;
use std::time::Duration;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::net::TcpStream;
use tokio::io::{BufReader, AsyncWriteExt, AsyncBufReadExt, AsyncBufRead};
use crate::scgi::Listener;

use clock;

mod scgi;

use crate::sensor::Sensor;
mod sensor;

type Connection = Arc<Mutex<sqlite::Connection>>;

//const SOCK_USER : &str = "http";
const SOCK_USER : &str = "lighttpd";
//const SOCK_GROUP : &str = "http";
const SOCK_GROUP : &str = "lighttpd";

fn get_uid_and_gid(uid_name : &str, gid_name : &str) -> Option<(u32, u32)> {
    let cstr = CString::new(uid_name.as_bytes()).ok()?;

    let p = unsafe { libc::getpwnam(cstr.as_ptr()) };
    if p.is_null() {
        return None;
    }
    let uid = unsafe { (*p).pw_uid };

    let cstr = CString::new(gid_name.as_bytes()).ok()?;

    let p = unsafe {libc::getgrnam(cstr.as_ptr()) };
    if p.is_null() {
        return None;
    }
    let gid = unsafe { (*p).gr_gid };

    Some((uid, gid))
}

fn drop_privs(sock_name: &str, uid_name : &str, gid_name : &str) {

    let (uid, gid) = get_uid_and_gid(uid_name, gid_name).unwrap();

    chown(sock_name, Some(uid), Some(gid)).expect("Chown failed");

    let p_uid = unsafe { libc::getuid() };
    if p_uid == 0 {
        // Remove group privileges
        unsafe { libc::setgroups(0, std::ptr::null()) };

        // Try setting the new uid/gid
        unsafe { libc::setgid(gid) };
        unsafe { libc::setuid(uid) };

        // Ensure a very conservative umask
        unsafe { libc::umask(0o077) }; 
    }
}

//fn create_socket() -> UnixListener {
//
//    let server = UnixListener::bind(SOCK_NAME).unwrap();
//    drop_privs(SOCK_USER, SOCK_GROUP);
//    return server
//}

async fn wait_tick(ticker : &clock::Clock) -> Result<(), ()> {
     let delay = ticker.secs_to_next_tick() + 60;
     sleep(Duration::from_secs(delay.into())).await;
     Ok(())
}


//----------------------------------------------------------------------------------------------------------------------------------
fn get_address(config : &Table, section : &str) -> SocketAddr {
    let host = config[section]["host"].as_str().unwrap();
    let port = config["common"]["port"].as_integer().unwrap();
    let mut addrs_iter = format!("{}:{}", host, port).to_socket_addrs().unwrap();
    println!("{:?}", addrs_iter);
    addrs_iter.next().unwrap()
}


//----------------------------------------------------------------------------------------------------------------------------------
fn create_db_connection(config : &Table)-> (Connection, String) {

    let db_file = config["scgi"]["database"].as_str().unwrap();
    println!("Opening database {}", db_file);
    let db_connection = Arc::new(Mutex::new(sqlite::open(db_file).unwrap()));

    let db_table = config["scgi"]["db_table"].as_str().unwrap();
    println!("Creating/using db table {}", db_table);

    let query = format!("CREATE TABLE IF NOT EXISTS {} (unix_time INT NOT NULL, temperature REAL, humidity REAL, pressure REAL, PRIMARY KEY(unix_time));", db_table);

    {
        let conn = db_connection.lock().unwrap();
        (*conn).execute(query).unwrap();
    }
    (db_connection, String::from(db_table))
}



fn main() {
    let path = std::path::Path::new("weather.toml");
    let config_str = match std::fs::read_to_string(path) {
        Ok(f) => f,
        Err(e) => panic!("Failed to read config file {}", e)
    };

    let config: Table = config_str.parse().unwrap();
//    dbg!(&config);

    let indoor_sensor = Sensor::new(get_address(&config, "indoor"));

    let outdoor_sensor = Sensor::new(get_address(&config, "outdoor"));

    let rt = Runtime::new().unwrap();

    let columns = rt.block_on(indoor_sensor.get_column_names());
    println!("{:?}", columns);
    let columns = rt.block_on(outdoor_sensor.get_column_names());
    println!("{:?}", columns);



    let sock_name = config["scgi"]["sock_name"].as_str().unwrap();
    let _ = remove_file(sock_name);

    let (db_connection, db_table) = create_db_connection(&config);

    let mut listener = Listener::new(sock_name, db_connection.clone());


    rt.spawn(async move { listener.task().await });

    let period = config["common"]["sample_period_in_mins"].as_integer().unwrap() as i32;
    let ticker = clock::Clock::new(period * 60);

    loop {
        rt.block_on(wait_tick(&ticker)).unwrap();
        println!("Tick");

    }
}
