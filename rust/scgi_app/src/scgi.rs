//!
//! Listen to SCGI connections to read database data
//!

use tokio::net::UnixListener;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{self,AsyncBufReadExt,AsyncWriteExt, AsyncReadExt, BufReader};
use sqlite:: Connection;

pub struct Listener {
    sock_name :  String,
    db_connection : Arc<Mutex<Connection>>,
}

impl Listener {

    pub fn new(sockname : &str, db_connection : Arc<Mutex<Connection>>) -> Listener {
        Listener {
            sock_name : String::from(sockname),
            db_connection
        }
    }

    pub async fn task(&mut self) -> io::Result<()> {

        // Next up we create a UNIX listener which will listen for incoming
        let server = UnixListener::bind(&self.sock_name).unwrap();
        println!("Listening on: {}", self.sock_name);


        loop {
           let (conn, _) = server.accept().await?;

           let mut reader = BufReader::new(conn);

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
            writer.write_all(b"Status: 200 OK\r\n");
            writer.write_all(b"Content-Type: text/plain\r\n");
            writer.write_all(b"\r\n");
            writer.write_all(b"Hello, world!\r\n");
            println!("Done");
       }
    }
}

