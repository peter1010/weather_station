
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use tokio::net::TcpStream;
use tokio::io::{self, BufReader, AsyncWriteExt, AsyncBufReadExt};
use weather_err::Result;

use crate::config;

type Connection = Arc<Mutex<sqlite::Connection>>;

//----------------------------------------------------------------------------------------------------------------------------------
pub struct Sensor {
    address : SocketAddr,
    columns : Vec<String>,
    db_connection : Connection,
    db_table : String,
    last_collected_time : i64
}

//----------------------------------------------------------------------------------------------------------------------------------
impl Sensor {

    //------------------------------------------------------------------------------------------------------------------------------
    pub async fn new(config : &config::Config, name : &str) -> Result<Self> {
        let address = Self::get_address(&config, &name)?;
        let columns = Self::get_column_names(&address).await?;
        let (db_connection, db_table) = Self::create_db_connection(&config, &columns, &name)?;
        let last_collected_time = Self::get_last_time(&db_connection, &db_table)?;
        Ok(Self {
            address,
            columns,
            db_connection,
            db_table,
            last_collected_time
        })
    }

    //------------------------------------------------------------------------------------------------------------------------------
    fn get_address(config : &config::Config, name : &str) -> Result<SocketAddr> {
        let host = config.get_host(&name)?;
        let port = config.get_port()?;

        let mut addrs_iter = format!("{}:{}", host, port).to_socket_addrs()?;
        println!("{:?}", addrs_iter);
        Ok(addrs_iter.next().ok_or("No IP Address found")?)
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub async fn get_column_names(addr : &SocketAddr) -> Result<Vec<String>> {

        let mut stream = BufReader::new(TcpStream::connect(addr).await?);

        stream.write_all(b"columns\n").await?;

        let mut columns = Vec::<String>::new();

        loop {
            let mut line = String::new();
            let n = stream.read_line(&mut line).await?;
            if n == 0 {
                break;
            }
            line = String::from(line.trim());
            if line == "" {
                break;
            }
            columns.push(line);
        }
        println!("{:?}", columns);
        return Ok(columns);
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn create_db_connection(config : &config::Config, columns : &Vec<String>, name : &str)-> Result<(Connection, String)> {

        let (db_file, db_table) = config.get_database(name)?;
        println!("Opening database {}", db_file);

        let db_connection = Arc::new(Mutex::new(sqlite::open(db_file)?));

        println!("Creating/using db table {}", db_table);

        let mut query = String::from(format!("CREATE TABLE IF NOT EXISTS {} (unix_time INT NOT NULL", db_table));
        for col in columns {
            query.push_str(format!(", {} REAL", col).as_str());
        }
        query.push_str(", PRIMARY KEY(unix_time));");
        {
            let conn = db_connection.lock()?;
            (*conn).execute(query)?;
        }
        Ok((db_connection, String::from(db_table)))
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn get_last_time(db_connection : &Connection, db_table : &str) -> Result<i64> {
        let query = format!("SELECT MAX(unix_time) from {};", db_table);
        // println!("{}", query);

        let conn = db_connection.lock()?;

        let statement = (*conn).prepare(query)?;

        // Should only be one row!
        let row = statement.into_iter().next().unwrap()?;
        // println!("{:?}", row);
        let last_collected_time = match row.try_read::<i64, _>("MAX(unix_time)") {
            Ok(value) => value,
            Err(..) => 0
        };
        //println!("last_collected_time = {}", last_collected_time);
        Ok(last_collected_time)
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn insert(&mut self, unix_time : i64, values : &HashMap::<String, f32>) {

        if values.len() == 0 {
            return;
        }
        let mut query = format!("INSERT INTO {} VALUES ({}", self.db_table, unix_time);
        for col in &self.columns {
            query.push_str(format!(", {}", values[col]).as_str());
        }
        query.push_str(");");
        {
            let conn = self.db_connection.lock().unwrap();
            (*conn).execute(query).unwrap();
        }
        self.last_collected_time = unix_time;
    }


    //----------------------------------------------------------------------------------------------------------------------------------
    pub async fn collect(&mut self) -> Result<()>{
        let mut stream = BufReader::new(TcpStream::connect(self.address).await?);

        stream.write_all(format!("{}\n", self.last_collected_time + 1).as_bytes()).await?;

        let mut values = HashMap::new();
        let mut line = String::new();
        let mut time : i64 = 0;

        loop {
            let n = stream.read_line(&mut line).await?;
            if n == 0 {
                break;
            }
            line = String::from(line.trim());
            if line == "" {
                break;
            }
            let mut tokens = line.split("=");
            let name = String::from(tokens.next().ok_or("No name")?.trim());
            if name == "unix_time" {
                self.insert(time, &values);
                time = tokens.next().ok_or("No value")?.trim().parse::<i64>()?;
                values.clear();
            } else {
                let value = tokens.next().ok_or("No value")?.trim().parse::<f32>()?;

                println!("{} => {}", name, value);
                values.insert(name, value);
            }
            line.clear();
        }
        self.insert(time, &values);

        Ok(())
    }
}


