use i2cdev::linux::LinuxI2CDevice;
use i2cdev::core::*;
use std::fmt;
use weather_err::{Result, WeatherError};

const SHT31_ADDR : u16 = 0x44;



//----------------------------------------------------------------------------------------------------------------------------------
pub struct Summary {
    temperature : f32,
    humidity : f32
}

//----------------------------------------------------------------------------------------------------------------------------------
impl Summary {
    pub fn new(temp: f32, humd : f32) -> Self {
        Self {
            temperature : temp,
            humidity : humd
        }
    }

    pub fn get_temperature(&self) -> f32 {
        self.temperature
    }

    pub fn get_humidity(&self) -> f32 {
        self.humidity
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl fmt::Display for Summary{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.1}C {:.1}%", self.temperature, self.humidity)
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
pub struct Sht31 {
    dev : LinuxI2CDevice,
}


//----------------------------------------------------------------------------------------------------------------------------------
impl Sht31 {


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn new(dev_name : &str) -> Result<Self> {
        Ok(Self {
            dev : LinuxI2CDevice::new(dev_name, SHT31_ADDR)?
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
    fn process_resp(&self, resp : &[u8]) -> Result<Summary> {
        let calc_crc = Self::crc(&resp[0..2]);
        if calc_crc != resp[2] {
            return Err(WeatherError::from("Invalid Checksum"));
        }
        let calc_crc = Self::crc(&resp[3..5]);
        if calc_crc != resp[5] {
            return Err(WeatherError::from("Invalid Checksum"));
        }
        let adc = ((resp[0] as u16) << 8) | (resp[1] as u16);
        let temp = -45.0 + 175.0 * (adc as f32) / 65535.0;
        let adc = ((resp[3] as u16) << 8) | (resp[4] as u16);
        let humd = 100.0 * (adc as f32) / 65535.0;
        Ok(Summary::new(temp, humd))
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn one_shot(&mut self) -> Result<()> {
        self.dev.write(&[0x24, 0x00])?;
        Ok(())
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn sample(&mut self) -> Result<Summary> {
        self.dev.write(&[0xE0, 0x00])?;
        let mut resp : [u8; 6] = [0; 6];
        self.dev.read(&mut resp)?;
        Ok(self.process_resp(&resp)?)
    }
}

//----------------------------------------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn read_temperature() {
        let mut sensor = Sht31::new("/dev/i2c-sht31").unwrap();

        sensor.one_shot().unwrap();
        thread::sleep(Duration::from_secs(1));
        let summary = sensor.sample().unwrap();

        println!("{}", summary);
    }
}

