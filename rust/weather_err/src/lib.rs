use std::fmt;
use std::sync::PoisonError;
use tokio::io;
use i2cdev::linux::LinuxI2CError;
use sqlite;
use std::num::{ParseIntError, ParseFloatError};

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
impl<T> From<PoisonError<T>> for WeatherError {
    fn from(error: PoisonError<T>) -> Self {
        Self {
            error : format!("Mutex Error {}", error)
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
impl From<toml::de::Error> for WeatherError {
    fn from(error: toml::de::Error) -> Self {
        Self {
            error : format!("TOML Error {}", error)
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


