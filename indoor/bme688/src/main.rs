extern crate i2cdev;

use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use i2cdev::core::*;

const BME688_ADDR : u16 = 0x76;


struct Bme688 {
    dev : LinuxI2CDevice,
    hum_oversampling : u8,
    temp_oversampling : u8,
    pres_oversampling : u8,
    par_t1 : u16,
    par_t2 : i16,
    par_t3 : i8,
    par_p1 : u16,
    par_p2 : i16,
    par_p3 : i8,
    par_p4 : i16,
    par_p5 : i16,
    par_p6 : i8,
    par_p7 : i8,
    par_p8 : i16,
    par_p9 : i16,
    par_p10 : u8,
    t_fine : f64
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

        let par_t1 = ((dev.smbus_read_byte_data(0xEA).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0xE9).unwrap() as u16);

        let par_t2 = (((dev.smbus_read_byte_data(0x8B).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x8A).unwrap() as u16)) as i16;

        let par_t3 = dev.smbus_read_byte_data(0x8C).unwrap() as i8;

        println!("par_t1 {:?}", par_t1);
        println!("par_t2 {:?}", par_t2);
        println!("par_t3 {:?}", par_t3);

        let par_p1 = ((dev.smbus_read_byte_data(0x8F).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x8E).unwrap() as u16);

        let par_p2 = (((dev.smbus_read_byte_data(0x91).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x90).unwrap() as u16)) as i16;

        let par_p3 = dev.smbus_read_byte_data(0x92).unwrap() as i8;

        let par_p4 = (((dev.smbus_read_byte_data(0x95).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x94).unwrap() as u16)) as i16;

        let par_p5 = (((dev.smbus_read_byte_data(0x97).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x96).unwrap() as u16)) as i16;

        let par_p6 = dev.smbus_read_byte_data(0x99).unwrap() as i8;

        let par_p7 = dev.smbus_read_byte_data(0x98).unwrap() as i8;

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

        let this = Self {
            dev,
            hum_oversampling : 0,
            temp_oversampling : 0,
            pres_oversampling : 0,
            par_t1,
            par_t2,
            par_t3,
            par_p1, par_p2, par_p3, par_p4, par_p5, par_p6, par_p7, par_p8, par_p9, par_p10,
            t_fine : 0.0
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

    fn read_temp_adc(&mut self, field :u8) -> u32 {
        let base : u8 = 0x22 + 0x11 * field;
        let msb = self.dev.smbus_read_byte_data(base + 0x00).unwrap() as u32;
        let lsb = self.dev.smbus_read_byte_data(base + 0x01).unwrap() as u32;
        let xlsb = self.dev.smbus_read_byte_data(base + 0x02).unwrap() as u32;

        let adc = (msb << 12) | (lsb << 4) | (xlsb >> 4);
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

        let temp_adc = f64::from(self.read_temp_adc(field));
        let par_t1 = f64::from(self.par_t1);
        let par_t2 = f64::from(self.par_t2);
        let par_t3 = f64::from(self.par_t3);

        let var1 = ((temp_adc / 16384.0) - (par_t1 / 1024.0)) * par_t2;
        println!("var1 {:?}", var1);

        let var2 = (((temp_adc / 131072.0) - (par_t1 / 8192.0)) *
            ((temp_adc / 131072.0) - (par_t1 / 8192.0))) *
            (par_t3 * 16.0);

        let t_fine = var1 + var2;

        self.t_fine = t_fine;

        let temp = t_fine / 5120.0;
        println!("temp_comp {:?}", temp);
        temp
    }


    fn read_press(&mut self, field: u8) -> f64 {

        let press_adc = f64::from(self.read_press_adc(field));
        let par_p1 = f64::from(self.par_p1);
        let par_p2 = f64::from(self.par_p2);
        let par_p3 = f64::from(self.par_p3);
        let par_p4 = f64::from(self.par_p4);
        let par_p5 = f64::from(self.par_p5);
        let par_p6 = f64::from(self.par_p6);
        let par_p7 = f64::from(self.par_p7);
        let par_p8 = f64::from(self.par_p8);
        let par_p9 = f64::from(self.par_p9);
        let par_p10 = f64::from(self.par_p10);

        let var1 = (self.t_fine / 2.0) - 64000.0;
        let var2 = var1 * var1 * (par_p6 / 131072.0);

        let var2 = var2 + (var1 * par_p5 * 2.0);
        let var2 = (var2 / 4.0) + (par_p4 * 65536.0);
        let var1 = (((par_p3 * var1 * var1) / 16384.0) + (par_p2 * var1)) / 524288.0;

        let var1 = (1.0 + (var1 / 32768.0)) * par_p1;
        let press_comp = 1048576.0 - press_adc;
        let press_comp = ((press_comp - (var2 / 4096.0)) * 6250.0) / var1;
        let var1 = (par_p9 * press_comp * press_comp) / 2147483648.0;
        let var2 = press_comp * (par_p8 / 32768.0);
        let var3 = (press_comp / 256.0) * (press_comp / 256.0) *
            (press_comp / 256.0) * (par_p10 / 131072.0);
        let press_comp = press_comp + (var1 + var2 + var3 +
            (par_p7 * 128.0)) / 16.0;
        println!("press_comp {:?}", press_comp);
        press_comp
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
}
