extern crate i2cdev;

use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use i2cdev::core::*;

const BME688_ADDR : u16 = 0x76;


pub struct Bme688 {
    dev : LinuxI2CDevice,
    hum_oversampling : u8,
    temp_oversampling : u8,
    pres_oversampling : u8,

    // Temperature params
    par_ta : f64,
    par_tb : f64,
    par_tc : f64,

    // Pressure params
    par_pvar1a : f64,
    par_pvar1b : f64,
    par_pvar1c : f64,

    // last know var1 result. var1 is calculated from temperature
    par_pvar1 : f64,

    par_pvar2a : f64,
    par_pvar2b : f64,
    par_pvar2c : f64,

    // last know var2 result. var2 is calculated from temperature
    par_pvar2 : i64,

    par_pa : f64,
    par_pb : f64,
    par_pc : f64,
    par_pd : f64,

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

    // Gas heater params
    par_g1 : f64,
    par_g2 : f64,
    par_g3 : f64,
    par_g4 : f64,
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

fn two_to_pow(exp : i8) -> f64 {
    return 2.0_f64.powi(exp as i32)
}

impl Bme688 {

    pub fn new() -> Result<Self,LinuxI2CError> {
        let dev = LinuxI2CDevice::new("/dev/i2c-bme688", BME688_ADDR)?;

        let this = Self {
            dev,
            hum_oversampling : 0,
            temp_oversampling : 0,
            pres_oversampling : 0,
            par_ta : 0.0, par_tb : 0.0, par_tc : 0.0,
            par_pvar1a : 0.0, par_pvar1b : 0.0, par_pvar1c : 0.0, par_pvar1: 0.0,
            par_pvar2a : 0.0, par_pvar2b : 0.0, par_pvar2c: 0.0, par_pvar2: 0,
            par_pa : 0.0, par_pb : 0.0, par_pc : 0.0, par_pd : 0.0,
            par_h1 : 0, par_h2 : 0.0, par_h3 : 0.0, par_h4: 0.0, par_h5: 0.0, par_h6: 0.0, par_h7: 0.0,
            par_hvar3 : 0, par_hvar4 : 0.0, par_hvar5 : 0.0,
            temperature : 0.0,
            par_g1 : 0.0, par_g2 : 0.0, par_g3 : 0.0, par_g4 : 0.0,
        };
        Ok(this)
    }

    fn read_u8(&mut self, addr : u8) -> u8 {
        self.dev.smbus_read_byte_data(addr).unwrap()
    }

    fn write_u8(&mut self, addr :u8, value : u8) {
        self.dev.smbus_write_byte_data(addr, value).unwrap();
    }

    fn read_u16(&mut self, addr : u8) -> u16 {
        // little-endian
        ((self.read_u8(addr+1) as u16) << 8) | (self.read_u8(addr) as u16)
    }

    fn read_i8(&mut self, addr : u8) -> i8 {
        self.dev.smbus_read_byte_data(addr).unwrap() as i8
    }

    fn read_i16(&mut self, addr : u8) -> i16 {
        self.read_u16(addr) as i16
    }


    pub fn cache_temperature_params(&mut self) {
        let par_t1 = self.read_u16(0xE9);
        let par_t2 = self.read_i16(0x8A);
        let par_t3 = self.read_i8(0x8C);

        // From the Datasheet...
        // var1 = ((temp_adc / 16384) - (par_t1 / 1024)) * par_t2;
        // var2 = (((temp_adc / 131072) - (par_t1 / 8192)) * ((temp_adc / 131072) - (par_t1 / 8192))) * (par_t3 * 16);
        // t_fine = var1 + var2;
        // temp = t_fine / 5120.0

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

//        println!("Temp(C) = {:?} * adc^2 + {:?} * adc + {:?}", self.par_ta, self.par_tb, self.par_tc);
    }


    pub fn cache_pressure_params(&mut self) {
        let par_p4 = self.read_i16(0x94);
        let par_p5 = self.read_i16(0x96);
        let par_p6 = self.read_i8(0x99);

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

        //         16 = 2^4
        //        256 = 2^8
        //       5120 = 5 * 2^10
        //      16384 = 2^14
        //      32768 = 2^15
        //      64000 = 2^9 * 125
        //      65536 = 2^16
        //      32000 = 2^8 * 125
        //     131072 = 2^17
        //     524288 = 2^19
        // 2147483648 = 2^31;

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

        let denom = two_to_pow(-13);

        self.par_pvar2a = (a as f64) * denom;
        self.par_pvar2b = (b as f64) * denom;
        self.par_pvar2c = (c as f64) * denom;

//        println!("Pres var2 = {:?} * temp^2 + {:?} * temp + {:?}", self.par_pvar2a, self.par_pvar2b, self.par_pvar2c);

        let par_p1 = self.read_u16(0x8E);
        let par_p2 = self.read_i16(0x90);
        let par_p3 = self.read_i8(0x92);

        // From the Datasheet...
        // t_fine = temp * 5120
        // var1 = (t_fine / 2) - 64000;
        // var1a = (((par_p3 * var1 * var1) / 16384) + (par_p2 * var1)) / 524288;
        // var1b = (1 + (var1a / 32768)) * par_p1;
        //
        // var1b = par_p1 * (1 + var1a / 2^15);
        // var1b = par_p1 * (1 + ((((par_p3 * var1 * var1) /2^14) + (par_p2 * var1)) / 2^19) /2^15))
        // var1b = par_p1 * (1 + (par_p3 * var1^2) / 2^48 + ((par_p2 * var1) / 2^34))


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

        // 6250 = 2 * 3125
        let denom = two_to_pow(-31) * 0.00032;

        self.par_pvar1a = (a as f64) * denom;
        self.par_pvar1b = (b as f64) * denom;
        self.par_pvar1c = (c as f64) * denom;

//        println!("Pres var1 = {:?} * temp^2 + {:?} * temp + {:?}", self.par_pvar1a, self.par_pvar1b, self.par_pvar1c);

        let par_p7 = self.read_i8(0x98);
        let par_p8 = self.read_i16(0x9C);
        let par_p9 = self.read_i16(0x9E);
        let par_p10 = self.read_i8(0xA0);

        // From the Datasheet...
        // var1 = (par_p9 * press_comp * press_comp) / 2147483648;
        // var2 = press_comp * (par_p8 / 32768);
        // var3 = (press_comp / 256) * (press_comp / 256) * (press_comp / 256) * (par_p10 / 131072);
        // press_comp = press_comp + (var1_p + var2_p + var3_p + (par_p7 * 128)) / 16;


        // pressure = A * press_comp^3 + B * press_comp^2 + C * press_comp + D
        // A = par_p10/(256 * 256 * 256 * 131072 * 16)
        //   = par_p10 /(2^8 * 2^8 * 2^8 * 2^17 * 2^4)
        //   = par_p10 / (2^45)
        // A' = par_p10
        // B  = par_p9 / (2^31 * 2^4)
        //   = par_p9 / (2^35)
        // B' = par_p9 * 3^10
        // C  = 1 + par_p8 / (2^15 *2^4)
        //   = 1 + par_p8 / (2^19)
        // C' = 2^45 + par_p8 * 2^26
        // D = 128 * par_p7 / (16)
        //   = par_p7 * 2^3
        // D' = par_p7 * 2^48

        let a = par_p10;
        let b = (par_p9 as i32) << 10; 
        let c = (1_i64 << 45) + ((par_p8 as i64) << 26);
        let d = (par_p7 as i64) << 48;

        let denom = two_to_pow(-47) * 0.04;

        self.par_pa = (a as f64) * denom;
        self.par_pb = (b as f64) * denom;
        self.par_pc = (c as f64) * denom;
        self.par_pd = (d as f64) * denom;

//        println!("Pressure = {:?} * comp^3 + {:?} * comp^2 + {:?} * comp + {:?}", self.par_pa, self.par_pb, self.par_pc, self.par_pd);

    }


    pub fn cache_humditiy_params(&mut self) {

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

    pub fn cache_gas_params(&mut self) {
        let par_g1 = self.read_i8(0xED);

        let par_g2 = self.read_i16(0xEB);
        let par_g3 = self.read_i8(0xEE);

        let heat_res_range = (self.read_u8(0x02) >> 4) & 0x03;
        let res_heat_val = self.read_i8(0x00);

        println!("par_g1 {:?}", par_g1);
        println!("par_g2 {:?}", par_g2);
        println!("par_g3 {:?}", par_g3);
        println!("heat_res_range {:?}", heat_res_range);
        println!("res_heat_val {:?}", res_heat_val);

        // From the datasheet
        // var1 = (par_g1 / 16) + 49;
        // var2 = ((par_g2 / 32768) * 0.0005) + 0.00235;
        // var3 = par_g3 / 1024.0;
        // var4 = var1 * (1.0 + (var2 * target_temp));
        // var5 = var4 + (var3 * amb_temp);
        // res_heat_x = (uint8_t)(3.4 * ((var5 * (4 / (4 + res_heat_range)) * (1/(1 + (res_heat_val * 0.002)))) - 25

        let par_g1 = (((par_g1 as i16) + (16 * 49)) as f64) * two_to_pow(-4);
        let par_g2 = ((par_g2 as f64) * two_to_pow(-15)) * 0.0005 + 0.00235;
        let par_g3 = (par_g3 as f64) * two_to_pow(-10);
        let par_g4 =  (3.4 * 4.0) / ((1.0 + ((res_heat_val as f64) * 0.002)) * (4.0 + (heat_res_range as f64)));

        self.par_g1 = par_g1;
        self.par_g2 = par_g2;
        self.par_g3 = par_g3;
        self.par_g4 = par_g4;


        // so becomes..
        // var5 = (par_g1 * (1.0 + (par_g2 * target_temp))) + (par_g3 * amb_temp);
        // res_heat_x = (uint8_t)(var5 * par_g4) - 25


        println!("par_g1 {:?}", par_g1);
        println!("par_g2 {:?}", par_g2);
        println!("par_g3 {:?}", par_g3);
        println!("par_g4 {:?}", par_g4);
    }

    pub fn set_humdity_oversampling(&mut self, oversampling : u8) {
        self.hum_oversampling = calc_oversampling(oversampling).unwrap();
    }


    pub fn set_temperature_oversampling(&mut self, oversampling : u8) {
        self.temp_oversampling = calc_oversampling(oversampling).unwrap();
    }


    pub fn set_pressure_oversampling(&mut self, oversampling : u8) {
        self.pres_oversampling = calc_oversampling(oversampling).unwrap();
    }

    pub fn set_heater(&mut self, field:u8, temperature : u16) {
        let par_g1 = self.read_i8(0xED);

        let par_g2 = self.read_i16(0xEB);
        let par_g3 = self.read_i8(0xEE);

        let res_heat_range = (self.read_u8(0x02) >> 4) & 0x03;
        let res_heat_val = self.read_i8(0x00);

        // From the datasheet
        // var1 = (par_g1 / 16) + 49;
        // var2 = ((par_g2 / 32768) * 0.0005) + 0.00235;
        // var3 = par_g3 / 1024.0;
        // var4 = var1 * (1.0 + (var2 * target_temp));
        // var5 = var4 + (var3 * amb_temp);
        // res_heat_x = (uint8_t)(3.4 * ((var5 * (4 / (4 + res_heat_range)) * (1/(1 + (res_heat_val * 0.002)))) - 25))

        let var1 = ((par_g1 as f64) / 16.0) + 49.0;
        let var2 = ((par_g2 as f64) / 32768.0) * 0.0005 + 0.00235;
        let var3 = (par_g3 as f64) / 1024.0;
        let var4 = var1 * (1.0 + (var2 * (temperature as f64)));
        let var5 = var4 + (var3 * self.temperature);
        let res_heat_x = 3.4 * (var5 * (4.0 / (4.0 + (res_heat_range as f64))) * (1.0/(1.0 + ((res_heat_val as f64) * 0.002))) - 25.0);
        let res_heat_x = res_heat_x as u32;

        println!("res_heat_x {:?}", res_heat_x);

        let var4 = self.par_g1 * (1.0 + (self.par_g2 * (temperature as f64)));
        let var5 = var4 + (self.par_g3 * self.temperature);
        let res_heat_x = var5 * self.par_g4;
        let res_heat_x = (res_heat_x as u32) - 85;

        println!("res_heat_x {:?}", res_heat_x);
 
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


    fn read_press_adc(&mut self, field :u8) -> i32 {
        let base : u8 = 0x1F + 0x11 * field;
        // Big-endian!
        let msb = self.read_u8(base + 0x00) as u32;
        let lsb = self.read_u8(base + 0x01) as u32;
        let xlsb = self.read_u8(base + 0x02) as u32;

        let adc = ((msb << 12) | (lsb << 4) | (xlsb >> 4)) as i32;
//        println!("press_adc {:?}", adc);
        adc
    }


    fn read_humd_adc(&mut self, field :u8) -> u16 {
        let base : u8 = 0x25 + 0x11 * field;
        // Big-endian!
        let msb = self.read_u8(base + 0x00) as u16;
        let lsb = self.read_u8(base + 0x01) as u16;

        let adc = (msb << 8) | lsb;
        println!("humd_adc {:?}", adc);
        adc
    }


    pub fn read_temp(&mut self, field :u8) -> f64 {

        let adc = self.read_temp_adc(field) as f64;

        let temp = self.par_ta * adc * adc + self.par_tb * adc + self.par_tc;

        if self.temperature != temp {
            self.temperature = temp;
            self.cache_press_temp_vars();
            self.cache_humd_temp_vars();
        }

        println!("Temperature is {:.2} C", temp);
        temp
    }

    fn cache_press_temp_vars(&mut self) {

        let temp = self.temperature;
        let var2 = self.par_pvar2a  * temp * temp + self.par_pvar2b  * temp + self.par_pvar2c;

        self.par_pvar2 = var2 as i64;
//        println!("var2 = {:?}", self.par_pvar2);

        let var1 = self.par_pvar1a  * temp * temp + self.par_pvar1b  * temp + self.par_pvar1c;

        self.par_pvar1 = var1;
//        println!("var1 = {:?}", self.par_pvar1);
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

    pub fn read_press(&mut self, field: u8) -> f64 {

        let adc = self.read_press_adc(field);

        let comp = (((1048576 - adc) as i64 - self.par_pvar2) as f64) / self.par_pvar1;

        let pressure = comp * comp * comp * self.par_pa + comp * comp * self.par_pb + comp * self.par_pc + self.par_pd;

        println!("Pressure {:.0} millibars", pressure + 30_f64);
        pressure as f64
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

        // Write Pressure & temperature oversampling
        let tmp = (self.temp_oversampling << 5) | (self.pres_oversampling << 2);
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

