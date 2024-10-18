//!
//! Listen to connections to read database data
//!

use tokio::net::TcpListener;
use sqlite;
use tokio::io::{AsyncBufReadExt,AsyncWriteExt, BufReader};
use std::sync::{Arc, Mutex};
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
        let conn = self.db_connection.lock()?;
        for row in (*conn).prepare(query)?.into_iter() {
            self.table_name = Some(String::from(row?.read::<&str,_>("name")));
            return Ok(());
        }
        Err(WeatherError::from("No table"))
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn cfg_columns(&mut self) -> Result<()> {
        let query = format!("pragma table_info ('{}');", self.table_name.as_ref().unwrap());
        let conn = self.db_connection.lock()?;
        self.column_names = Some((*conn)
                .prepare(query)?
                .into_iter()
                .map(|row| String::from(row.unwrap().read::<&str,_>("name")))
                .filter(|x| x != "unix_time")
                .collect());
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    async fn measurement_resp(&self, unix_time : i64, stream : &mut (impl AsyncWriteExt + std::marker::Unpin)) -> Result<()> {
        println!("Rcv'd {:?}", unix_time);

        let mut response = String::new();

        let query = format!("select * from {} where unix_time > {};", self.table_name.as_ref().unwrap(), unix_time);
        {
            let conn = self.db_connection.lock()?;

            let statement = (*conn).prepare(query)?;

            for row in statement
                .into_iter()
                .map(|row| row.unwrap())
            {
                // println!("{:?}", row);
                response += &(format!("unix_time = {}", row.read::<i64, _>("unix_time")) + "\n");
                for col in self.column_names.as_ref().unwrap() {
                    response += &(format!("\t{} = {}", col, row.read::<f64, _>(col.as_str())) + "\n");
                }
            }
        }

        stream.write_all(response.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    async fn columns_resp(&self, stream : &mut (impl AsyncWriteExt + std::marker::Unpin)) -> Result<()>{
        println!("Rcv'd columns");

        let mut response = String::new();

        for column in self.column_names.as_ref().unwrap() {
            response += &(String::from(column) + "\n");
        }
        response += &("\n");
        stream.write_all(response.as_bytes()).await?;
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    async fn process_client(&mut self, mut stream : impl AsyncWriteExt + AsyncBufReadExt + std::marker::Unpin) -> Result<()>{
        loop {
            let mut line = String::new();

            let n = stream.read_line(&mut line).await?;

            if n == 0 {
                return Ok(());
            }
            line = String::from(line.trim());

            if line == "columns" {
                self.columns_resp(&mut stream).await?;
            } else {
                match line.parse::<i64>() {
                    Ok(unix_time) => self.measurement_resp(unix_time, &mut stream).await?,
                    Err(..) => stream.write_all(format!("Error unknown command {}\n", line).as_bytes()).await?
                }
            }
        }
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub async fn task(&mut self) -> Result<()> {
        self.cfg_table()?;
        self.cfg_columns()?;

        // println!("{:?}", self.column_names);
        let sock_addr = format!("0.0.0.0:{}", self.port);

        // Next up we create a TCP listener which will listen for incoming
        // connections. This TCP listener is bound to the address we determined
        // above and must be associated with an event loop.
        let listener = TcpListener::bind(&sock_addr).await?;
        println!("Listening on: {}", sock_addr);

        loop {
            // Asynchronously wait for an inbound socket.
            let (socket, _) = listener.accept().await?;

            let stream = BufReader::new(socket);

            self.process_client(stream).await?
        }
    }
}

