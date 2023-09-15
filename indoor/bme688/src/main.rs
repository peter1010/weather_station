extern crate i2cdev;

use i2cdev::linux::LinuxI2CDevice;
use i2cdev::core::*;

const BME688_ADDR : u16 = 0x76;


fn main() {
    let mut dev = LinuxI2CDevice::new("/dev/i2c-4", BME688_ADDR).unwrap();

    let val = dev.smbus_read_byte_data(0xD0).unwrap();

    println!("Hello, world! {:?}", val);

    // Write Humdity oversampling 1x
    let humdity_oversampling = 1;
    dev.smbus_write_byte_data(0x72, humdity_oversampling);

    let pressure_oversampling = 5; // 16
    let temperature_oversampling = 2;

    // Write Pressure (16x) & temperature (2x) oversampling
    dev.smbus_write_byte_data(0x74, (temperature_oversampling << 5) | (pressure_oversampling << 2));

    dev.smbus_write_byte_data(0x74, (temperature_oversampling << 5) | (pressure_oversampling << 2) | 1);

    loop {
        let mode = dev.smbus_read_byte_data(0x74).unwrap() & 0x03;
        if mode == 0 {
            break;
        }
    }
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

    let mut par_t3 = i32::from(dev.smbus_read_byte_data(0x8C).unwrap());
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
