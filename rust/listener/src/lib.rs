//!
//! Listen to connections to read database data
//!

use std::net::TcpListener;
use sqlite;
use std::io::{BufReader, BufRead, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use weather_err::{Result, WeatherError};


//----------------------------------------------------------------------------------------------------------------------------------
pub struct Listener {
    port : u16,
    db_connection : Arc<Mutex<sqlite::Connection>>,
    table_name : Option<String>,
    column_names : Option<Vec<String>>
}

//----------------------------------------------------------------------------------------------------------------------------------
impl Listener {

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn new(port :u16, db_connection : Arc<Mutex<sqlite::Connection>>) -> Listener {
        Listener {
            port,
            db_connection,
            table_name : None,
            column_names : None
        }
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn cfg_table(&mut self) -> Result<()> {
        if self.table_name.is_some() {
            return Ok(());
        }
        let query = "select name from sqlite_master where type = 'table';";
        let conn = self.db_connection.lock().expect("Unexpected failure to lock mutex");
        for row in (*conn).prepare(query)?.into_iter() {
            self.table_name = Some(String::from(row?.read::<&str,_>("name")));
            return Ok(());
        }
        Err(WeatherError::from("No table"))
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn cfg_columns(&mut self) -> Result<()> {
        let query = format!("pragma table_info ('{}');", self.table_name.as_ref().unwrap());
        let conn = self.db_connection.lock().expect("Unexpected failure to lock mutex");
        self.column_names = Some((*conn)
                .prepare(query)?
                .into_iter()
                .map(|row| String::from(row.unwrap().read::<&str,_>("name")))
                .filter(|x| x != "unix_time")
                .collect());
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn measurement_resp(db_connection : &Arc<Mutex<sqlite::Connection>>, column_names : &Vec<String>,
                table_name : &str,
                unix_time : i64, stream : &mut impl Write) -> Result<()> {
        println!("Rcv'd {:?}", unix_time);

        let mut response = String::new();

        let query = format!("select * from {} where unix_time > {};", table_name, unix_time);
        {
            let conn = db_connection.lock().expect("Unexpected failure to lock mutex");

            let statement = (*conn).prepare(query)?;

            for row in statement
                .into_iter()
                .map(|row| row.unwrap())
            {
                // println!("{:?}", row);
                response += &(format!("unix_time = {}", row.read::<i64, _>("unix_time")) + "\n");
                for col in column_names {
                    response += &(format!("\t{} = {}", col, row.read::<f64, _>(col.as_str())) + "\n");
                }
            }
        }

        stream.write_all(response.as_bytes())?;
        stream.write_all(b"\n")?;
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn columns_resp(column_names : &Vec<String>, stream : &mut impl Write) -> Result<()>{
        println!("Rcv'd columns");

        let mut response = String::new();

        for column in column_names {
            response += &(String::from(column) + "\n");
        }
        response += &("\n");
        println!("{}", response);
        stream.write_all(response.as_bytes())?;
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn process_client(column_names: &Vec<String>, table_name: &str, 
                db_connection: &Arc<Mutex<sqlite::Connection>>, mut stream_in : impl BufRead, mut stream_out : impl Write) -> Result<()>{
        loop {
            let mut line = String::new();

            let n = stream_in.read_line(&mut line)?;

            if n == 0 {
                return Ok(());
            }
            line = String::from(line.trim());

            if line == "columns" {
                Self::columns_resp(&column_names, &mut stream_out)?;
            } else {
                match line.parse::<i64>() {
                    Ok(unix_time) => Self::measurement_resp(db_connection, column_names, table_name,unix_time, &mut stream_out)?,
                    Err(..) => stream_out.write_all(format!("Error unknown command {}\n", line).as_bytes())?
                }
            }
        }
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn start(&mut self) {
        self.cfg_table().unwrap();
        self.cfg_columns().unwrap();

        let port = self.port;
        let column_names = self.column_names.clone().unwrap().clone();
        let db_connection = self.db_connection.clone();
        let table_name = self.table_name.clone().unwrap().clone();

        thread::spawn(move || { 
            let _ = Self::task(port, column_names, table_name, db_connection); 
        });
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn task(port : u16, column_names: Vec<String>, 
        table_name: String, db_connection : Arc<Mutex<sqlite::Connection>>) -> Result<()> {

        // println!("{:?}", self.column_names);
        let sock_addr = format!("0.0.0.0:{}", port);

        // Next up we create a TCP listener which will listen for incoming
        // connections. This TCP listener is bound to the address we determined
        // above and must be associated with an event loop.
        let listener = TcpListener::bind(&sock_addr)?;
        println!("Listening on: {}", sock_addr);

        loop {
            // Asynchronously wait for an inbound socket.
            let (socket, _) = listener.accept()?;

            let stream_in = BufReader::new(&socket);
            let stream_out = &socket;

            Self::process_client(&column_names, &table_name, &db_connection, stream_in, stream_out)?
        }
    }
}

