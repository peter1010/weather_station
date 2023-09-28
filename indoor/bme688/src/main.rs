extern crate i2cdev;

use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use i2cdev::core::*;

const BME688_ADDR : u16 = 0x76;


struct Bme688 {
    dev : LinuxI2CDevice,
    hum_oversampling : u8,
    temp_oversampling : u8,
    pres_oversampling : u8,

    par_ta : f64,
    par_tb : f64,
    par_tc : f64,

    par_pvar1a : f64,
    par_pvar1b : f64,
    par_pvar1c : f64,

    par_pvar2a : f64,
    par_pvar2b : f64,
    par_pvar2c : f64,

    par_p7 : i32,
    par_p8 : i32,
    par_p9 : i32,
    par_p10 :i32,

    par_h1 : u16,
    par_h2 : u16,
    par_h3 : i8,
    par_h4 : i8,
    par_h5 : i8,
    par_h6 : i8,
    par_h7 : i8,

    temperature : f64,
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

        let par_p7 = (dev.smbus_read_byte_data(0x98).unwrap() as i8) as i32;

        let par_p8 = ((((dev.smbus_read_byte_data(0x9D).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x9C).unwrap() as u16)) as i16) as i32;

        let par_p9 = ((((dev.smbus_read_byte_data(0x9F).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x9E).unwrap() as u16)) as i16) as i32;

        let par_p10 = (dev.smbus_read_byte_data(0xA0).unwrap() as u32) as i32;

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
            par_ta : 0.0, par_tb : 0.0, par_tc : 0.0,
            par_pvar1a : 0.0, par_pvar1b : 0.0, par_pvar1c : 0.0,
            par_pvar2a : 0.0, par_pvar2b : 0.0, par_pvar2c: 0.0,
            par_p7, par_p8, par_p9, par_p10,
            par_h1, par_h2, par_h3, par_h4, par_h5, par_h6, par_h7,
            temperature : 0.0,
        };
        Ok(this)
    }


    fn get_temperature_params(&mut self) {
        // par_t1 is a u16
        // par_t2 is a i16
        // par_t3 is a i8
        let dev = &mut self.dev;

        let par_t1 = ((dev.smbus_read_byte_data(0xEA).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0xE9).unwrap() as u16);

        let par_t2 = (((dev.smbus_read_byte_data(0x8B).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x8A).unwrap() as u16)) as i16;

        let par_t3 = dev.smbus_read_byte_data(0x8C).unwrap() as i8;

        println!("");
        println!("Calculating Temp equation...");
        println!("par_t1 {:?}", par_t1);
        println!("par_t2 {:?}", par_t2);
        println!("par_t3 {:?}", par_t3);

        // From the Datasheet...
        // var1 = ((temp_adc / 16384) - (par_t1 / 1024)) * par_t2;
        // var2 = (((temp_adc / 131072) - (par_t1 / 8192)) * ((temp_adc / 131072) - (par_t1 / 8192))) * (par_t3 * 16);
        // t_fine = var1 + var2;
        // temp_comp = t_fine / 5120.0

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

        // 2^-30 / 5120
        let denom = 2.0_f64.powi(-40) * 0.2;

        self.par_ta = (a as f64) * denom;
        self.par_tb = (b as f64) * denom;
        self.par_tc = (c as f64) * denom;

        println!("Temp(C) = {:?} * adc^2 + {:?} * adc + {:?}", self.par_ta, self.par_tb, self.par_tc);
    }


    fn get_pressure_params(&mut self) {
        // par_p4 is a i16
        // par_p5 is a i16
        // par_p6 is a i8

        let dev = &mut self.dev;

        let par_p4 = (((dev.smbus_read_byte_data(0x95).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x94).unwrap() as u16)) as i16;

        let par_p5 = (((dev.smbus_read_byte_data(0x97).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x96).unwrap() as u16)) as i16;

        let par_p6 = dev.smbus_read_byte_data(0x99).unwrap() as i8;

        println!("");
        println!("Calculating Pressure var2 equation...");
        println!("par_p4 {:?}", par_p4);
        println!("par_p5 {:?}", par_p5);
        println!("par_p6 {:?}", par_p6);


        // From the Datasheet...
        // temp = t_fine * 5120
        // var1 = (t_fine / 2) - 64000;
        // var1a = var1 * var1 * (par_p6 / 131072);
        // var1b = var1a + (var1 * par_p5 * 2);
        // var2 = (var1b / 4) + (par_p4 * 65536);
        //
        // var2 = (var1b / 2^2) + (par_p4 * 2^16);
        // var2 = ((var1a + (var1 * par_p5 * 2)) / 2^2) + (par_p4 * 2^16);
        // var2 = (var1a / 2^2) + (var1 * par_p5 /2) + (par_p4 * 2^16);
        // var2 = (((var1^2 * par_p6 /2^17 / 2^2) + (var1 * par_p5)/2 + (par_p4 * 2^16);
        // var2 = (var1^2  * par_p6 / 2^19) + (var1 * par_p5)/2 + (par_p4 * 2^16);

        // 64000 = 2^9 * 125
        // 32000 = 2^8 * 125
        // 131072 = 2^17
        // 65536 = 2^16
        // 5120 = 5 * 2^10

        // var2 = A * t_fine^2 + B * t_fine + C
        // A = 5120^2 * 2^-2 * par_p6 / 2^19
        //   = 5^2 * 2^20 * 2^-2 * par_p6 / 2^19
        //   = 5^2 * par_p6 / 2^1
        // A' = 5^2 * par_p6
        // B = 5120 * (-64000 * par_p6 / 2^19 + par_p5 / 2 / 2)
        //   = 5 * 2^10 * (-125 * 2^9 * par_p6 / 2^19 + par_p5 / 2^2)
        //   = 5 * (-125 * par_p6 + par_p5 * 2^8)
        // B' = 5 * 2 * (-5^3 * par_p6 + par_p5 *2^8)
        // C = 64000^2 * par_p6 / 2^19 - 32000 * par_p5 + (par_p4 * 2^16)
        //   = (125^2 * 2^18 * par_p6 / 2^19) - (2^8 * 125 * par_p5) + (par_p4 * 2^16)
        //   = (125^2 * par_p6 / 2) - (125 * 2^8 * par_p5) + (par_p4 * 2^16)
        // C' = (125^2 * par_p6) - (125 * 2^9 *par_p5) + (par_p4 * 2^17)

        let a = 25 * (par_p6 as i16);
        let b = 10 * ((-125 * (par_p6 as i32)) + ((par_p5 as i32) << 8));
        let c = (125 * 125 * (par_p6 as i64)) - ((125 * (par_p5 as i64)) << 9) + ((par_p4 as i64) << 17);

        // 2^-30 / 5120
        let denom = 2.0_f64.powi(-13);

        self.par_pvar2a = (a as f64) * denom;
        self.par_pvar2b = (b as f64) * denom;
        self.par_pvar2c = (c as f64) * denom;

        println!("Pres var2 = {:?} * temp^2 + {:?} * temp + {:?}", self.par_pvar2a, self.par_pvar2b, self.par_pvar2c);

        let par_p1 = ((dev.smbus_read_byte_data(0x8F).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x8E).unwrap() as u16);

        let par_p2 = (((dev.smbus_read_byte_data(0x91).unwrap() as u16) << 8)
            | (dev.smbus_read_byte_data(0x90).unwrap() as u16)) as i16;

        let par_p3 = dev.smbus_read_byte_data(0x92).unwrap() as i8;

        println!("");
        println!("Calculating Pressure var1 equation...");
        println!("par_p1 {:?}", par_p1);
        println!("par_p2 {:?}", par_p2);
        println!("par_p3 {:?}", par_p3);


        // From the Datasheet...
        // t_fine = temp * 5120
        // var1 = (t_fine / 2) - 64000;
        // var1a = (((par_p3 * var1 * var1) / 16384) + (par_p2 * var1)) / 524288;
        // var1b = (1 + (var1a / 32768)) * par_p1;
        //
        // var1b = par_p1 * (1 + var1a / 2^15);
        // var1b = par_p1 * (1 + ((((par_p3 * var1 * var1) /2^14) + (par_p2 * var1)) / 2^19) /2^15))
        // var1b = par_p1 * (1 + (par_p3 * var1^2) / 2^48 + ((par_p2 * var1) / 2^34))


        // 64000 = 2^9 * 125
        // 16384 = 2^14
        // 32768 = 2^15
        // 524288 = 2^19
        // 5120 = 5 * 2^10


        // var1 = A * t_fine^2 + B * t_fine + C
        // A = 5120^2 * par_p1 * par_p3 / 2^2 * 2^48
        //   = 25 *2^20 * par_p1 * par_p3 / 2^2 * 2^48
        //   = 25 * par_p1 * par_p3 / 2^30
        // A' = 25 * par_p1 * par_p3
        // B = 5120 * par_p1 *(-64000 * par_p3 / 2^48 +  par_p2 / 2^1 / 2^34)
        //   = 5120 * par_p1 *(-125 * 2^9 * par_p3 / 2^48 +  par_p2 / 2^35)
        //   = 5 * 12^10 *par_p1 *(-125 * par_p3 / 2^39 + par_p2 / 2^35)
        //   = 5 * par_p1 * (-125 * par_p3 / 2^29 + par_p2 / 2^25)
        // B' = 5 * par_p1 *(-125 * par_p3 * 2 + par_p2 * 2^5)
        // C = par_p1 * (1 + 64000^2 * par_p3 / 2^48 - (64000 * par_p2 / 2^34))
        //   = par_p1 * (1 + 125^2 * 2^18 * par_p3 / 2^48) - (125 * 2^9 * par_p2 / 2^34)
        //   = par_p1 * (1 + 125^2 * par_p3 / 2^30) - (125 * par_p2 / 2^25)
        // C' = par_p1 * (2^30 + (125^2 * par_p3) - (125 * par_p2 * 2^5))

        let a = 25 * (par_p1 as i32) * (par_p3 as i32);
        let b = 5 * ((par_p1 as i64) * ((-125 * (par_p3 as i64)) + ((par_p2 as i64) << 4))) << 1;
        let c = (par_p1 as i64) * ((1 << 30) + (125 * 125 * (par_p3 as i64))  - ((125 * (par_p2 as i64)) << 5));

        // 35, 19, 15

        let denom = 2.0_f64.powi(-30);

        self.par_pvar1a = (a as f64) * denom;
        self.par_pvar1b = (b as f64) * denom;
        self.par_pvar1c = (c as f64) * denom;

        println!("Pres var1 = {:?} * temp^2 + {:?} * temp + {:?}", self.par_pvar1a, self.par_pvar1b, self.par_pvar1c);
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


    fn read_press_adc(&mut self, field :u8) -> i32 {
        let base : u8 = 0x1F + 0x11 * field;
        let msb = self.dev.smbus_read_byte_data(base + 0x00).unwrap() as u32;
        let lsb = self.dev.smbus_read_byte_data(base + 0x01).unwrap() as u32;
        let xlsb = self.dev.smbus_read_byte_data(base + 0x02).unwrap() as u32;

        let adc = ((msb << 12) | (lsb << 4) | (xlsb >> 4)) as i32;
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

        let adc = self.read_temp_adc(field) as f64;

        let temp = self.par_ta * adc * adc + self.par_tb * adc + self.par_tc;

        self.temperature = temp;

        println!("temp_comp {:?}", temp);
        temp
    }

    fn get_press_temp_vars(&mut self) -> (i64, i64) {

        let temp = self.temperature;
        let var2 = self.par_pvar2a  * temp * temp + self.par_pvar2b  * temp + self.par_pvar2c;

        let var2 = var2 as i64;
        println!("var2 = {:?}", var2);

        let var1 = self.par_pvar1a  * temp * temp + self.par_pvar1b  * temp + self.par_pvar1c;

        let var1 = var1 as i64;
        println!("var1 =? {:?}", var1);
        (var1, var2)
    }


    fn read_press(&mut self, field: u8) -> f64 {

        let press_adc = self.read_press_adc(field);
        // At most 22 bits

        let (var1, var2) = self.get_press_temp_vars();

        let press_comp = (((1048576 - press_adc) as i64 - var2) * 6250) / var1;

        // A = par_p10 / (2^53)
        // B = par_p9 / (2^35)
        // C = 1 + par_p8 / 2^19
        // D = par_p7 * 2^3
        let var1 = ((self.par_p9 as i64) * (press_comp * press_comp)) >> 31;
        let var2 = (press_comp * (self.par_p8 as i64)) >> 15;
        let var3 = ((press_comp >> 8) * (press_comp >> 8) * (press_comp >> 8) * (self.par_p10 as i64)) >> 17;
        let press_comp = press_comp + ((var1 + var2 + var3 + ((self.par_p7 as i64) << 7)) >> 4);


        println!("press_comp {:?}", press_comp + 350 * 250/30 );
        press_comp as f64
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

    drv.get_temperature_params();
    drv.get_pressure_params();

    drv.set_humdity_oversampling(1);
    drv.set_pressure_oversampling(16);
    drv.set_temperature_oversampling(2);

    drv.force();

    let temp = drv.read_temp(0);
    let temp = drv.read_press(0);
    let temp = drv.read_humd(0);
}
