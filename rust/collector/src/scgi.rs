//!
//! Listen to SCGI connections to read database data
//!

use tokio::net::UnixListener;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{self,AsyncBufReadExt,AsyncWriteExt, AsyncReadExt, BufReader};
use crate::sensor::Sensor;
use std::fs::remove_file;


//----------------------------------------------------------------------------------------------------------------------------------
pub struct Listener {
    indoor_sensor : Arc<Sensor>,
    outdoor_sensor : Arc<Sensor>,
    sock_name : String,
}

//----------------------------------------------------------------------------------------------------------------------------------
impl Listener {


   //------------------------------------------------------------------------------------------------------------------------------
    pub fn new(sock_name : &str, indoor_sensor : Arc<Sensor>, outdoor_sensor : Arc<Sensor>) -> Self {
        Self {
            indoor_sensor,
            outdoor_sensor,
            sock_name : sock_name.to_string(),
        }
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn create_sock(&self) -> UnixListener {

        let _ = remove_file(&self.sock_name);
        // Next up we create a UNIX listener which will listen for incoming
        let server = match UnixListener::bind(&self.sock_name) {
            Ok(server) => server,
            Err(error) => panic!("Failed to create {} - {}", self.sock_name, error)
        };
        println!("Listening on: {}", self.sock_name);
        server
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub async fn task(&mut self) -> io::Result<()> {

        let server = self.create_sock();

        loop {
           let (conn, _) = server.accept().await?;

           let mut reader = BufReader::new(conn);

            let mut debug = vec![0; 10];
            let _ = reader.read_exact(& mut debug);
            println!("data: {:?}", debug);

           let mut hdr_fields = HashMap::new();

           let mut hdr_length = vec![];
           let _ = reader.read_until(b':', &mut hdr_length);

            // Drop the colon
           hdr_length.pop();

            let hdr_length : u32 = std::str::from_utf8(&hdr_length).unwrap().parse().unwrap();

            let mut hdr = vec![0; hdr_length as usize];
            let _ = reader.read_exact(& mut hdr);

            let iter = hdr.split(|x| *x == b'\0');
            let mut name = String::new();
            let mut idx = 0;
            for part in iter {
                if idx == 0 {
                    name = std::str::from_utf8(&part).unwrap().to_string();
                    idx = 1;
                } else {
                    let value = std::str::from_utf8(&part).unwrap().to_string();
                    idx = 0;
                    println!("{} => {}", name, value);
                    hdr_fields.insert(name.clone(), value);
                }
            }
            let mut writer = reader.into_inner();
            writer.write_all(b"Status: 200 OK\r\n").await?;
            writer.write_all(b"Content-Type: text/plain\r\n").await?;
            writer.write_all(b"\r\n").await?;
            writer.write_all(b"Hello, world!\r\n").await?;
            println!("Done");
       }
    }
}

