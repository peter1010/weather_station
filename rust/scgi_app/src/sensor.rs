
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use tokio::net::TcpStream;
use tokio::io::{self, BufReader, AsyncWriteExt, AsyncBufReadExt};

use crate::config;

type Connection = Arc<Mutex<sqlite::Connection>>;

//----------------------------------------------------------------------------------------------------------------------------------
pub struct Sensor {
    address : SocketAddr,
    columns : Vec<String>,
    db_conn : Connection,
    db_table : String,
    last_time : i64
}

//----------------------------------------------------------------------------------------------------------------------------------
impl Sensor {

    //------------------------------------------------------------------------------------------------------------------------------
    pub async fn new(config : &config::Config, name : &str) -> io::Result<Self> {
        let addr = Self::get_address(&config, &name);
        let columns = Self::get_column_names(&addr).await?;
        let (db_conn, db_table) = Self::create_db_connection(&config, &columns, &name);
        let last_time = Self::get_last_time(&config, &db_conn, &db_table);
        Ok(Self {
            address : addr,
            columns,
            db_conn,
            db_table,
            last_time
        })
    }

    //----------------------------------------------------------------------------------------------------------------------------------
    fn get_address(config : &config::Config, name : &str) -> SocketAddr {
        let host = config.get_host(&name);
        let port = config.get_port();

        let mut addrs_iter = format!("{}:{}", host, port).to_socket_addrs().unwrap();
        println!("{:?}", addrs_iter);
        addrs_iter.next().unwrap()
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub async fn get_column_names(addr : &SocketAddr) -> io::Result<Vec<String>> {

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


    //----------------------------------------------------------------------------------------------------------------------------------
    fn create_db_connection(config : &config::Config, columns : &Vec<String>, name : &str)-> (Connection, String) {

        let (db_file, db_table) = config.get_database(name);
        println!("Opening database {}", db_file);

        let db_connection = Arc::new(Mutex::new(sqlite::open(db_file).unwrap()));

        println!("Creating/using db table {}", db_table);

        let mut query = String::from(format!("CREATE TABLE IF NOT EXISTS {} (unix_time INT NOT NULL", db_table));
        for col in columns {
            query.push_str(format!(", {} REAL", col).as_str());
        }
        query.push_str(", PRIMARY KEY(unix_time));");
        {
            let conn = db_connection.lock().unwrap();
            (*conn).execute(query).unwrap();
        }
        (db_connection, String::from(db_table))
    }


    //----------------------------------------------------------------------------------------------------------------------------------
    fn get_last_time(config : &config::Config, db_conn : &Connection, db_table : &str) -> i64 {
        let query = format!("SELECT MAX(unix_time) from {};", db_table);
        {
            let conn = db_conn.lock().unwrap();

            let statement = (*conn).prepare(query).unwrap();

            for row in statement
                .into_iter()
                .map(|row| row.unwrap())
            {
                println!("{:?}", row);
                let id = match row.try_read::<i64, _>("MAX(unix_time") {
                     Ok(id) => id,
                     Err(..) => 0
                };
                return id;
//                response += &(format!("unix_time = {}", row.read::<i64, _>("unix_time")) + "\n");
//                for col in self.column_names.as_ref().unwrap() {
//                    response += &(format!("\t{} = {}", col, row.read::<f64, _>(col.as_str())) + "\n");
//                }
            }
        }
        0
    }


    //----------------------------------------------------------------------------------------------------------------------------------
    fn insert(&self, unix_time : i64, values : &HashMap::<String, f32>) {

        if values.len() == 0 {
            return;
        }
        let mut query = format!("INSERT INTO {} VALUES ({}", self.db_table, unix_time);
        for col in &self.columns {
            query.push_str(format!(", {}", values[col]).as_str());
        }
        query.push_str(");");
        {
            let conn = self.db_conn.lock().unwrap();
            (*conn).execute(query).unwrap();
        }
    }


    //----------------------------------------------------------------------------------------------------------------------------------
    pub async fn collect(&self) -> io::Result<()>{
        let mut stream = BufReader::new(TcpStream::connect(self.address).await?);

        stream.write_all(format!("{}\n", self.last_time).as_bytes()).await?;

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
            let name = String::from(tokens.next().unwrap().trim());
            if name == "unix_time" {
                self.insert(time, &values);
                time = tokens.next().unwrap().trim().parse::<i64>().unwrap();
                values.clear();
            } else {
                let value = tokens.next().unwrap().trim().parse::<f32>().unwrap();

                println!("{} => {}", name, value);
                values.insert(name, value);
            }
            line.clear();
        }
        self.insert(time, &values);

        Ok(())
    }
}





