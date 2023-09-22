extern crate i2cdev;

use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use i2cdev::core::*;

const BME688_ADDR : u16 = 0x76;


struct Bme688 {
    dev : LinuxI2CDevice,
    hum_oversampling : u8,
    temp_oversampling : u8,
    pres_oversampling : u8,
    par_t1 : i32,
    par_t2 : i32,
    par_t3 : i32,
    par_p1 : f64,
    par_p2 : f64,
    par_p3 : f64,
    par_p4 : f64,
    par_p5 : f64,
    par_p6 : f64,
    par_p7 : f64,
    par_p8 : i16,
    par_p9 : i16,
    par_p10 : u8,
    par_h1 : u16,
    par_h2 : u16,
    par_h3 : i8,
    par_h4 : i8,
    par_h5 : i8,
    par_h6 : i8,
    par_h7 : i8,

    temperature : f64
}


#[derive(Debug)]
enum Bme688Error {
    ConversionError,
}


fn calc_oversampling(reqd : u8) -> Result<u8, Bme688Error> {
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
        return Err(Bme688Error::ConversionError);
    }
    Ok(result)
}


impl Bme688 {

    fn new() -> Result<Self,LinuxI2CError> {
        let mut dev = LinuxI2CDevice::new("/dev/i2c-4", BME688_ADDR)?;

        let par_t1 = (((dev.smbus_read_byte_data(0xEA).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0xE9).unwrap() as u16)) as i32;

        let par_t2 = ((((dev.smbus_read_byte_data(0x8B).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x8A).unwrap() as u16)) as i16) as i32;

        let par_t3 = (dev.smbus_read_byte_data(0x8C).unwrap() as i8) as i32;

        println!("par_t1 {:?}", par_t1);
        println!("par_t2 {:?}", par_t2);
        println!("par_t3 {:?}", par_t3);

        let par_p1 = (((dev.smbus_read_byte_data(0x8F).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x8E).unwrap() as u16)) as f64;

        let par_p2 = ((((dev.smbus_read_byte_data(0x91).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x90).unwrap() as u16)) as i16) as f64;

        let par_p3 = (dev.smbus_read_byte_data(0x92).unwrap() as i8) as f64;

        let par_p4 = ((((((dev.smbus_read_byte_data(0x95).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x94).unwrap() as u16)) as i16) as i32) * 65536) as f64;

        let par_p5 = ((((((dev.smbus_read_byte_data(0x97).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x96).unwrap() as u16)) as i16) as i32) * 2) as f64;

        let par_p6 = ((dev.smbus_read_byte_data(0x99).unwrap() as i8) as f64) / 131072.0;

        let par_p7 = (((dev.smbus_read_byte_data(0x98).unwrap() as i8) as i16) * 128) as f64;

        let par_p8 = (((dev.smbus_read_byte_data(0x9D).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x9C).unwrap() as u16)) as i16;

        let par_p9 = (((dev.smbus_read_byte_data(0x9F).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x9E).unwrap() as u16)) as i16;

        let par_p10 = dev.smbus_read_byte_data(0xA0).unwrap();

        println!("par_p1 {:?}", par_p1);
        println!("par_p2 {:?}", par_p2);
        println!("par_p3 {:?}", par_p3);
        println!("par_p4 {:?}", par_p4);
        println!("par_p5 {:?}", par_p5);
        println!("par_p6 {:?}", par_p6);
        println!("par_p7 {:?}", par_p7);
        println!("par_p8 {:?}", par_p8);
        println!("par_p9 {:?}", par_p9);
        println!("par_p10 {:?}", par_p10);

        let tmp = dev.smbus_read_byte_data(0xE2).unwrap() as u16;
        let par_h1 = ((dev.smbus_read_byte_data(0xE3).unwrap() as u16) << 4)
                | (tmp & 0x0F);
        let par_h2 = ((dev.smbus_read_byte_data(0xE1).unwrap() as u16) << 4)
                | (tmp >> 4);
        let par_h3 = dev.smbus_read_byte_data(0xE4).unwrap() as i8;
        let par_h4 = dev.smbus_read_byte_data(0xE5).unwrap() as i8;
        let par_h5 = dev.smbus_read_byte_data(0xE6).unwrap() as i8;
        let par_h6 = dev.smbus_read_byte_data(0xE7).unwrap() as i8;
        let par_h7 = dev.smbus_read_byte_data(0xE8).unwrap() as i8;

        println!("par_h1 {:?}", par_h1);
        println!("par_h2 {:?}", par_h2);
        println!("par_h3 {:?}", par_h3);
        println!("par_h4 {:?}", par_h4);
        println!("par_h5 {:?}", par_h5);
        println!("par_h6 {:?}", par_h6);
        println!("par_h7 {:?}", par_h7);


        let this = Self {
            dev,
            hum_oversampling : 0,
            temp_oversampling : 0,
            pres_oversampling : 0,
            par_t1, par_t2, par_t3,
            par_p1, par_p2, par_p3, par_p4, par_p5, par_p6, par_p7, par_p8, par_p9, par_p10,
            par_h1, par_h2, par_h3, par_h4, par_h5, par_h6, par_h7,
            temperature : 0.0
        };
        Ok(this)
    }


    fn set_humdity_oversampling(&mut self, oversampling : u8) {
        self.hum_oversampling = calc_oversampling(oversampling).unwrap();
    }


    fn set_temperature_oversampling(&mut self, oversampling : u8) {
        self.temp_oversampling = calc_oversampling(oversampling).unwrap();
    }


    fn set_pressure_oversampling(&mut self, oversampling : u8) {
        self.pres_oversampling = calc_oversampling(oversampling).unwrap();
    }


    fn read_temp_adc(&mut self, field :u8) -> i32 {
        let base : u8 = 0x22 + 0x11 * field;
        let msb = self.dev.smbus_read_byte_data(base + 0x00).unwrap() as u32;
        let lsb = self.dev.smbus_read_byte_data(base + 0x01).unwrap() as u32;
        let xlsb = self.dev.smbus_read_byte_data(base + 0x02).unwrap() as u32;

        let adc = ((msb << 12) | (lsb << 4) | (xlsb >> 4)) as i32;
        println!("temp_adc {:?}", adc);
        adc
    }


    fn read_press_adc(&mut self, field :u8) -> u32 {
        let base : u8 = 0x1F + 0x11 * field;
        let msb = self.dev.smbus_read_byte_data(base + 0x00).unwrap() as u32;
        let lsb = self.dev.smbus_read_byte_data(base + 0x01).unwrap() as u32;
        let xlsb = self.dev.smbus_read_byte_data(base + 0x02).unwrap() as u32;

        let adc = (msb << 12) | (lsb << 4) | (xlsb >> 4);
        println!("press_adc {:?}", adc);
        adc
    }


    fn read_humd_adc(&mut self, field :u8) -> u16 {
        let base : u8 = 0x25 + 0x11 * field;
        let msb = self.dev.smbus_read_byte_data(base + 0x00).unwrap() as u16;
        let lsb = self.dev.smbus_read_byte_data(base + 0x01).unwrap() as u16;

        let adc = (msb << 8) | lsb;
        println!("humd_adc {:?}", adc);
        adc
    }


    fn read_temp(&mut self, field :u8) -> f64 {

        let temp_adc = self.read_temp_adc(field) as i32;
        let par_t1 = self.par_t1 as i32;
        let par_t2 = self.par_t2 as i32;
        let par_t3 = self.par_t3 as i32;

        let var1 = (temp_adc >> 3) - (self.par_t1 << 1);
        let var2 = (var1 * self.par_t2) >> 11;
        let var3 = ((((var1 >> 1) * (var1 >> 1)) >> 12) * (self.par_t3 << 4)) >> 14;
        let temp = ((var2 + var3) as f64) / 5120.0;

//        let var1 = ((temp_adc / 16384.0) - (par_t1 / 1024.0)) * par_t2;
//        println!("var1 {:?}", var1);

//        let var2 = (((temp_adc / 131072.0) - (par_t1 / 8192.0)) *
//            ((temp_adc / 131072.0) - (par_t1 / 8192.0))) *
//            (par_t3 * 16.0);

//        let temp = (var1 + var2) / 5120.0;

        self.temperature = temp;
        println!("temp_comp {:?}", temp);
        temp
    }


    fn read_press(&mut self, field: u8) -> f64 {

        let press_adc = f64::from(self.read_press_adc(field));

        let par_p8 = f64::from(self.par_p8);
        let par_p9 = f64::from(self.par_p9);
        let par_p10 = f64::from(self.par_p10);
        let t_fine = self.temperature * 5120.0;

        let var1 = (t_fine / 2.0) - 64000.0;
        let var2 = var1 * var1 * self.par_p6;

        let var2 = var2 + (var1 * self.par_p5);
        let var2 = (var2 / 4.0) + self.par_p4;
        let var1 = ((self.par_p3 * var1 * var1) / 16384.0 + (self.par_p2 * var1)) / 524288.0;

        let var1 = (1.0 + (var1 / 32768.0)) * self.par_p1;
        let press_comp = 1048576.0 - press_adc;
        let press_comp = ((press_comp - (var2 / 4096.0)) * 6250.0) / var1;
        let var1 = (par_p9 * press_comp * press_comp) / 2147483648.0;
        let var2 = press_comp * (par_p8 / 32768.0);
        let var3 = (press_comp / 256.0) * (press_comp / 256.0) *
            (press_comp / 256.0) * (par_p10 / 131072.0);
        let press_comp = press_comp + (var1 + var2 + var3 + self.par_p7) / 16.0;
        println!("press_comp {:?}", press_comp);
        press_comp
    }


    fn read_humd(&mut self, field: u8) -> f64 {

        let humd_adc = f64::from(self.read_humd_adc(field));
        let par_h1 = f64::from(self.par_h1);
        let par_h2 = f64::from(self.par_h2);
        let par_h3 = f64::from(self.par_h3);
        let par_h4 = f64::from(self.par_h4);
        let par_h5 = f64::from(self.par_h5);
        let par_h6 = f64::from(self.par_h6);
        let par_h7 = f64::from(self.par_h7);

        let var1 = humd_adc - ((par_h1 * 16.0) + ((par_h3 / 2.0) * self.temperature));
        let var2 = var1 * ((par_h2 / 262144.0) * (1.0 + ((par_h4 / 16384.0) *
                self.temperature) + ((par_h5 / 1048576.0) * self.temperature * self.temperature)));
        let var3 = par_h6 / 16384.0;
        let var4 = par_h7 / 2097152.0;
        let humd_comp = var2 + ((var3 + (var4 * self.temperature)) * var2 * var2);

        println!("humdity_comp {:?}", humd_comp);
        humd_comp
    }



    fn force(&mut self) {
        // Write Humdity oversampling

        self.dev.smbus_write_byte_data(0x72, self.hum_oversampling).unwrap();

        // Write Pressure & temperature oversampling
        let tmp = (self.temp_oversampling << 5) | (self.pres_oversampling << 2);
        self.dev.smbus_write_byte_data(0x74, tmp).unwrap();

        self.dev.smbus_write_byte_data(0x74, tmp | 1).unwrap();

        loop {
            let mode = self.dev.smbus_read_byte_data(0x74).unwrap() & 0x03;
            if mode == 0 {
                break;
            }
        }
    }
}


fn main() {

    let mut drv =  Bme688::new().unwrap();

    drv.set_humdity_oversampling(1);
    drv.set_pressure_oversampling(16);
    drv.set_temperature_oversampling(2);

    drv.force();

    let temp = drv.read_temp(0);
    let temp = drv.read_press(0);
    let temp = drv.read_humd(0);
}
