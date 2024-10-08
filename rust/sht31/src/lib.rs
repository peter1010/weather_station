extern crate i2cdev;

use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use i2cdev::core::*;

const SHT31_ADDR : u16 = 0x44;


pub struct Sht31 {
    dev : LinuxI2CDevice,
    hum_oversampling : u8,
    temp_oversampling : u8,

    // Temperature params
    par_ta : f64,
    par_tb : f64,
    par_tc : f64,

    // Humdity params
    par_h1 : i32,
    par_h2 : f64,
    par_h3 : f64,
    par_h4 : f64,
    par_h5 : f64,
    par_h6 : f64,
    par_h7 : f64,

    par_hvar3 : i32,
    par_hvar4 : f64,
    par_hvar5 : f64,

    // Last measure temperature
    temperature : f64,
}


#[derive(Debug)]
enum Sht31Error {
    ConversionError,
}


fn calc_oversampling(reqd : u8) -> Result<u8, Sht31Error> {
    let result;
    if reqd == 1 {
        result =  1;  // no oversampling
    } else if reqd == 2 {
        result =  2; // 2x oversampling
    } else if reqd == 4 {
        result =  3; // 4x oversampling
    } else if reqd == 8 {
        result =  4; // 8x oversampling
    } else if reqd == 16 {
        result =  5; // 8x oversampling
    } else {
        return Err(Sht31Error::ConversionError);
    }
    Ok(result)
}

fn two_to_pow(exp : i8) -> f64 {
    return 2.0_f64.powi(exp as i32)
}

impl Sht31 {

    pub fn new() -> Result<Self,LinuxI2CError> {
        let dev = LinuxI2CDevice::new("/dev/i2c-bme688", SHT31_ADDR)?;

        let this = Self {
            dev,
            hum_oversampling : 0,
            temp_oversampling : 0,
            par_ta : 0.0, par_tb : 0.0, par_tc : 0.0,
            par_h1 : 0, par_h2 : 0.0, par_h3 : 0.0, par_h4: 0.0, par_h5: 0.0, par_h6: 0.0, par_h7: 0.0,
            par_hvar3 : 0, par_hvar4 : 0.0, par_hvar5 : 0.0,
            temperature : 0.0
        };
        Ok(this)
    }

    fn read_u8(&mut self, addr : u8) -> u8 {
        self.dev.smbus_read_byte_data(addr).unwrap()
    }

    fn write_u8(&mut self, addr :u8, value : u8) {
        self.dev.smbus_write_byte_data(addr, value).unwrap();
    }

    fn read_u16_le(&mut self, addr : u8) -> u16 {
        // little-endian
        ((self.read_u8(addr+1) as u16) << 8) | (self.read_u8(addr) as u16)
    }

    fn read_u16_be(&mut self, addr : u8) -> u16 {
        // big-endian
        ((self.read_u8(addr) as u16) << 8) | (self.read_u8(addr + 1) as u16)
    }

    fn read_i8(&mut self, addr : u8) -> i8 {
        self.dev.smbus_read_byte_data(addr).unwrap() as i8
    }

    fn read_i16_le(&mut self, addr : u8) -> i16 {
        self.read_u16_le(addr) as i16
    }


    ///
    /// From the Datasheet...
    /// var1 = ((temp_adc / 16384) - (par_t1 / 1024)) * par_t2;
    /// var2 = (((temp_adc / 131072) - (par_t1 / 8192)) * ((temp_adc / 131072) - (par_t1 / 8192))) * (par_t3 * 16);
    /// t_fine = var1 + var2;
    /// temp = t_fine / 5120.0
    ///
    fn cache_temperature_params(&mut self) {
        let par_t1 = self.read_u16_le(0xE9);
        let par_t2 = self.read_i16_le(0x8A);
        let par_t3 = self.read_i8(0x8C);

        // 16 = 2^4
        // 131072 = 2^17
        // 16384 = 2^14
        // 8192 = 2^13
        // 1024 = 2^10

        // t_fine = A * (temp_adc)^2 + B * temp_adc + C, find A,B and C
        // Collecting temp_adc^2, temp_adc and the constant terms from equations above, 
        // Note: prime versions are 2^30 bigger i.e. A' = A * 2^30
        // A = (par_t3 * 16) / (131072 * 131072)
        //   = (par_t3 * 2^4) / (2^17 * 2^17)
        //   = par_t3 / 2^30
        // A' = par_t3
        // B = (par_t2/16384) - 2 * (par_t3 * 16) * (par_t1 / (8192 * 131072))
        //   = (par_t2 / 2^14) - (2^1 * par_t3 * 2^4 * par_t1 / (2^13 * 2^17))
        //   = (par_t2 / 2^14) - ((par_t1 * par_t3) / 2^25)
        // B' = (par_t2 * 2^16) - ((par_t1 * par_t3) * 2^5)
        // C = - (par_t1 * par_t2) / 1024 + (par_t1 * par_t1 * par_t3 * 16) / (8192 * 8192)
        //   = - (par_t1 * par_t2) / 2^10 + (par_t1 * par_t1 * par_t3 * 2^4) / (2^13 * 2^13)
        //   = - (par_t1 * par_t2) / 2^10 + (par_t1^2 * par_t3) / 2^22
        // C' = - (par_t1 * par_t2) * 2^20 + (par_t1^2 * par_t3) * 2^8
        let par_t13 = (par_t1 as i32) * (par_t3 as i32);
        let par_t12 = (par_t1 as i32) * (par_t2 as i32);

        let a = par_t3;
        let b = (((par_t2 as i32) << 11) - par_t13) << 5;
        let c = (-((par_t12 as i64) << 12) + (par_t1 as i64) * (par_t13 as i64)) << 8;

        // 2^-30 / 5120 => to convert t_fine to temperature
        let denom = two_to_pow(-40) * 0.2;

        self.par_ta = (a as f64) * denom;
        self.par_tb = (b as f64) * denom;
        self.par_tc = (c as f64) * denom;
    }


    fn cache_humditiy_params(&mut self) {

        let tmp = self.read_u8(0xE2) as u16;
        let par_h1 = ((self.read_u8(0xE3) as u16) << 4) | (tmp & 0x0F);
        let par_h2 = ((self.read_u8(0xE1) as u16) << 4) | (tmp >> 4);
        let par_h3 = self.read_i8(0xE4);
        let par_h4 = self.read_i8(0xE5);
        let par_h5 = self.read_i8(0xE6);
        let par_h6 = self.read_i8(0xE7);
        let par_h7 = self.read_i8(0xE8);

        // From the Datasheet...
        // var1 = humd_adc - ((par_h1 * 16) + ((par_h3 / 2) * temp));
        // var2 = var1 * ((par_h2 / 262144) * (1 + ((par_h4 / 16384) * temp) + ((par_h5 / 1048576) * temp * temp)));
        // humd_comp = var2 + ((par_h6 / 16384) + ((par_h7 / 2097152) * temp)) * var2 * var2);

        //   16384 = 2^14
        //  262144 = 2^18
        // 1048576 = 2^20
        // 2097152 = 2^21

        self.par_h1 = (par_h1 as i32) << 4;
        self.par_h2 = (par_h2 as f64) * two_to_pow(-18);
        self.par_h3 = (par_h3 as f64) * two_to_pow(-1);
        self.par_h4 = ((par_h4 as f64) * self.par_h2) * two_to_pow(-14);
        self.par_h5 = ((par_h5 as f64) * self.par_h2) * two_to_pow(-20);
        self.par_h6 = (par_h6 as f64) * two_to_pow(-14);
        self.par_h7 = (par_h7 as f64) * two_to_pow(-21);

        // Becomes...
        // var1 = humd_adc - (par_h1 + par_h3 * temp);
        // var2 = var1 * (par_h2 + par_h4 * temp + par_h5 * temp * temp);
        // humd_comp = var2 + (par_h6 + (par_h7 * temp)) * var2 * var2);
    }


    pub fn cache_params(&mut self) {
        self.cache_temperature_params();
        self.cache_humditiy_params();
    }


    pub fn set_humdity_oversampling(&mut self, oversampling : u8) {
        self.hum_oversampling = calc_oversampling(oversampling).unwrap();
    }


    pub fn set_temperature_oversampling(&mut self, oversampling : u8) {
        self.temp_oversampling = calc_oversampling(oversampling).unwrap();
    }


    fn read_temp_adc(&mut self, field :u8) -> i32 {
        let base : u8 = 0x22 + 0x11 * field;
        // Big-endian!
        let msb = self.read_u8(base + 0x00) as u32;
        let lsb = self.read_u8(base + 0x01) as u32;
        let xlsb = self.read_u8(base + 0x02) as u32;

        let adc = ((msb << 12) | (lsb << 4) | (xlsb >> 4)) as i32;
//        println!("temp_adc {:?}", adc);
        adc
    }


    fn read_humd_adc(&mut self, field :u8) -> u16 {
        let base : u8 = 0x25 + 0x11 * field;
        // Big-endian!
        let adc = self.read_u16_be(base);
        println!("humd_adc {:?}", adc);
        adc
    }


    pub fn read_temp(&mut self, field :u8) -> f64 {

        let adc = self.read_temp_adc(field) as f64;

        let temp = self.par_ta * adc * adc + self.par_tb * adc + self.par_tc;

        if self.temperature != temp {
            self.temperature = temp;
            self.cache_humd_temp_vars();
        }

        println!("Temperature is {:.2} C", temp);
        temp
    }



    fn cache_humd_temp_vars(&mut self) {
        let temp = self.temperature;

        // var1 = humd_adc - (par_h1 + par_h3 * temp);
        // var2 = var1 * (par_h2 + par_h4 * temp + par_h5 * temp * temp);
        // humd_comp = var2 + (par_h6 + (par_h7 * temp)) * var2 * var2);

        let var3 = self.par_h1 + ((self.par_h3 * temp) as i32);
        let var4 = self.par_h2 + self.par_h4 * temp + self.par_h5 * temp * temp;
        let var5 = self.par_h6 + self.par_h7 * temp;

        // Becomes
        // var1 = humd_adc - var3;
        // var2 = var1 * var4;
        // humd_comp = var2 + var5 * var2 * var2);
        self.par_hvar3 = var3;
        self.par_hvar4 = var4;
        self.par_hvar5 = var5;
    }


    pub fn read_humd(&mut self, field: u8) -> f64 {

        let adc = self.read_humd_adc(field) as i32;

        // Becomes
        // var1 = humd_adc - var3;
        // var2 = var1 * var4;
        // humd_comp = var2 + var5 * var2 * var2);
        let var1 = (adc - self.par_hvar3) as f64;
        let var2 = var1 * self.par_hvar4;
        let humdity = var2 + self.par_hvar5 * var2 * var2;

        println!("Humdity {:.2}%", humdity);
        humdity
    }


    pub fn force(&mut self) {
        // Write Humdity oversampling

        self.write_u8(0x72, self.hum_oversampling);

        // Write temperature oversampling
        let tmp = (self.temp_oversampling << 5);
        self.write_u8(0x74, tmp);

        self.write_u8(0x74, tmp | 1);

        loop {
            let mode = self.read_u8(0x74) & 0x03;
            if mode == 0 {
                break;
            }
        }
    }
}

