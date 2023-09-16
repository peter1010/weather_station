extern crate i2cdev;

use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use i2cdev::core::*;

const BME688_ADDR : u16 = 0x76;


struct Bme688 {
    mDev : LinuxI2CDevice,
    mHumOversampling : u8,
    mTempOversampling : u8,
    mPresOversampling : u8,
    par_t1 : i32,
    par_t2 : i32,
    par_t3 : i32
}


#[derive(Debug)]
enum Bme688Error {
    ConversionError,
}


fn calcOversampling(reqd : u8) -> Result<u8, Bme688Error> {
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

    fn new() -> Result<Bme688,LinuxI2CError> {
        let mut dev = LinuxI2CDevice::new("/dev/i2c-4", BME688_ADDR)?;

        let par_t1 = (i32::from(dev.smbus_read_byte_data(0xEA).unwrap()) << 8)
            + i32::from(dev.smbus_read_byte_data(0xE9).unwrap());

        let par_t2 = (i32::from(dev.smbus_read_byte_data(0x8B).unwrap()) << 8)
            + i32::from(dev.smbus_read_byte_data(0x8A).unwrap());

        let par_t3 = i32::from(dev.smbus_read_byte_data(0x8C).unwrap());

        println!("par_t1 {:?}", par_t1);
        println!("par_t2 {:?}", par_t2);
        println!("par_t3 {:?}", par_t3);

        let bme688 = Bme688 {
            mDev : dev,
            mHumOversampling : 0,
            mTempOversampling : 0,
            mPresOversampling : 0,
            par_t1,
            par_t2,
            par_t3
        };
        Ok(bme688)
    }


    fn set_humdity_oversampling(&mut self, oversampling : u8) {
        self.mHumOversampling = calcOversampling(oversampling).unwrap();
    }


    fn set_temperature_oversampling(&mut self, oversampling : u8) {
        self.mTempOversampling = calcOversampling(oversampling).unwrap();
    }


    fn set_pressure_oversampling(&mut self, oversampling : u8) {
        self.mPresOversampling = calcOversampling(oversampling).unwrap();
    }


    fn force(&mut self) {
        // Write Humdity oversampling

        self.mDev.smbus_write_byte_data(0x72, self.mHumOversampling);

        // Write Pressure & temperature oversampling
        let tmp = (self.mTempOversampling << 5) | (self.mPresOversampling << 2);
        self.mDev.smbus_write_byte_data(0x74, tmp);

        self.mDev.smbus_write_byte_data(0x74, tmp | 1);

        loop {
            let mode = self.mDev.smbus_read_byte_data(0x74).unwrap() & 0x03;
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

    let mut dev = LinuxI2CDevice::new("/dev/i2c-4", BME688_ADDR).unwrap();

    let mut temp_adc = i32::from(dev.smbus_read_byte_data(0x22).unwrap());
    temp_adc = (temp_adc << 8) + i32::from(dev.smbus_read_byte_data(0x23).unwrap());
    temp_adc = (temp_adc << 4) + (i32::from(dev.smbus_read_byte_data(0x24).unwrap()) >> 4);
    println!("temp_adc {:?}", temp_adc);

    let mut par_t1 = i32::from(dev.smbus_read_byte_data(0xEA).unwrap());
    par_t1 = (par_t1 << 8) + i32::from(dev.smbus_read_byte_data(0xE9).unwrap());
    println!("par_t1 {:?}", par_t1);

    let mut par_t2 = i32::from(dev.smbus_read_byte_data(0x8B).unwrap());
    par_t2 = (par_t2 << 8) + i32::from(dev.smbus_read_byte_data(0x8A).unwrap());
    println!("par_t2 {:?}", par_t2);

    let par_t3 = i32::from(dev.smbus_read_byte_data(0x8C).unwrap());
    println!("par_t3 {:?}", par_t3);


//    let var1 = ((f64::from(temp_adc) / 16384.0) - (f64::from(par_t1) / 1024.0)) * f64::from(par_t2);
//    println!("var1 {:?}", var1);

//    let var2 = (((f64::from(temp_adc) / 131072.0) - (f64::from(par_t1) / 8192.0)) *
//        ((f64::from(temp_adc) / 131072.0) - (f64::from(par_t1) / 8192.0))) *
//        (f64::from(par_t3) * 16.0);
//    let t_fine = var1 + var2;
//    let temp_comp = t_fine / 5120.0;
//    println!("temp_comp {:?}", temp_comp);

    let var1 = (temp_adc >> 3) - (par_t1 << 1);
    println!("var1 {:?}", var1);

    let var2 = (i64::from(var1) * i64::from(par_t2)) >> 11;
    println!("var2 {:?}", var2);

    let var3 = (((i64::from(var1 >> 1) * i64::from(var1 >> 1)) >> 12) * i64::from(par_t3 << 4)) >> 14;
    println!("var3 {:?}", var3);

    let t_fine = var2 + var3;
    println!("t_fine {:?}", t_fine);

    let temp_comp = ((t_fine * 5) + 128) >> 8;
    println!("temp_comp {:?}", temp_comp);

}
