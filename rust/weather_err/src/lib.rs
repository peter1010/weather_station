use std::fmt;
use tokio::io;
use i2cdev::linux::LinuxI2CError;
use sqlite;
use std::num::{ParseIntError, ParseFloatError};
use std::ffi::NulError;

//----------------------------------------------------------------------------------------------------------------------------------
pub struct WeatherError {
    error : String
}

pub type Result<T> = std::result::Result<T, WeatherError>;


//----------------------------------------------------------------------------------------------------------------------------------
impl From<io::Error> for WeatherError {
    fn from(error: io::Error) -> Self {
        Self {
            error : format!("IO Error {}", error)
        }
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl From<&str> for WeatherError {
    fn from(error : &str) -> Self {
        Self {
            error : String::from(error)
        }
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl fmt::Debug for WeatherError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl From<LinuxI2CError> for WeatherError {
    fn from(error: LinuxI2CError) -> Self {
        Self {
            error : format!("I2C Error {}", error)
        }
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl From<sqlite::Error> for WeatherError {
    fn from(error: sqlite::Error) -> Self {
        Self {
            error : format!("SQL Error {}", error)
        }
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl From<ParseIntError> for WeatherError {
    fn from(error: ParseIntError) -> Self {
        Self {
            error : format!("Parse to Int Error {}", error)
        }
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl From<ParseFloatError> for WeatherError {
    fn from(error: ParseFloatError) -> Self {
        Self {
            error : format!("Parse to Float Error {}", error)
        }
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl From<NulError> for WeatherError {
    fn from(error: NulError) -> Self {
        Self {
            error : format!("Null Error {}", error)
        }
    }
}


