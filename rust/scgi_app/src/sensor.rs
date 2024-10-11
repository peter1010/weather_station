
use std::net::SocketAddr;

use tokio::net::TcpStream;
use tokio::io::{self, BufReader, AsyncWriteExt, AsyncBufReadExt};

//----------------------------------------------------------------------------------------------------------------------------------
pub struct Sensor {
    address : SocketAddr
}

//----------------------------------------------------------------------------------------------------------------------------------
impl Sensor {

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn new(addr : SocketAddr) -> Sensor {
        Sensor {
            address : addr
        }
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub async fn get_column_names(&self) -> io::Result<Vec<String>> {

        let mut stream = BufReader::new(TcpStream::connect(self.address).await?);

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
        return Ok(columns);
    }
}


