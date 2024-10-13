extern crate i2cdev;

use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use i2cdev::core::*;
use std::fmt;

const SHT31_ADDR : u16 = 0x44;

//----------------------------------------------------------------------------------------------------------------------------------
pub struct Sht31Error {
    error : String
}

pub type Result<T> = std::result::Result<T, Sht31Error>;

//----------------------------------------------------------------------------------------------------------------------------------
impl From<LinuxI2CError> for Sht31Error {
    fn from(error: LinuxI2CError) -> Sht31Error {
        Sht31Error {
            error : format!("I2C Error {}", error)
        }
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl From<&str> for Sht31Error {
    fn from(error : &str) -> Sht31Error {
        Sht31Error {
            error : String::from(error)
        }
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl fmt::Debug for Sht31Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
pub struct Sht31 {
    dev : LinuxI2CDevice,
}


//----------------------------------------------------------------------------------------------------------------------------------
impl Sht31 {


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn new() -> Result<Self> {
        Ok(Self {
            dev : LinuxI2CDevice::new("/dev/i2c-sht31", SHT31_ADDR)?
        })
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn crc(data : &[u8]) -> u8 {
        let mut crc : u8 = 0xff;

        for val in data {
            crc ^= val;
            for _ in 0..8 {
                if (crc & 0x80) != 0 {
                    crc = (crc << 1) ^ 0x31;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn process_resp(&self, resp : &[u8]) -> Result<(f32, f32)> {
        let calc_crc = Self::crc(&resp[0..2]);
        if calc_crc != resp[2] {
            return Err(Sht31Error::from("Invalid Checksum"));
        }
        let calc_crc = Self::crc(&resp[3..5]);
        if calc_crc != resp[5] {
            return Err(Sht31Error::from("Invalid Checksum"));
        }
        let adc = ((resp[0] as u16) << 8) | (resp[1] as u16);
        let temp = -45.0 + 175.0 * (adc as f32) / 65535.0;
        let adc = ((resp[3] as u16) << 8) | (resp[4] as u16);
        let humd = 100.0 * (adc as f32) / 65535.0;
        Ok((temp, humd))
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn one_shot(&mut self) -> Result<(f32, f32)> {
        // Write Humdity oversampling

        self.dev.write(&[0x24, 0x00])?;
        loop {
            self.dev.write(&[0xE0, 0x00])?;
            let mut resp : [u8; 6] = [0; 6];
            match self.dev.read(&mut resp) {
                Ok(_) => return self.process_resp(&resp),
                Err(..) => ()
            }
        }
    }
}

//----------------------------------------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_temperature() {
        let mut sensor = Sht31::new().unwrap();

        let (temp, humd) = sensor.one_shot().unwrap();
        println!("{} {}", temp, humd);
    }
}

