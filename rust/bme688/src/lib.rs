use i2cdev::linux::LinuxI2CDevice;
use i2cdev::core::*;
use std::fmt;
use weather_err::Result;

const BME688_ADDR : u16 = 0x76;



//----------------------------------------------------------------------------------------------------------------------------------
pub struct Summary {
    temperature : f32,
    humidity : f32,
    pressure : f32
}

//----------------------------------------------------------------------------------------------------------------------------------
impl Summary {
    pub fn new(temp: f32, humd : f32, press : f32) -> Self {
        Self {
            temperature : temp,
            humidity : humd,
            pressure : press
        }
    }

    pub fn get_temperature(&self) -> f32 {
        self.temperature
    }

    pub fn get_humidity(&self) -> f32 {
        self.humidity
    }

    pub fn get_pressure(&self) -> f32 {
        self.pressure
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.1}C {:.1}% {:.0} millibars", self.temperature, self.humidity, self.pressure)
    }
}



//----------------------------------------------------------------------------------------------------------------------------------
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
}


//----------------------------------------------------------------------------------------------------------------------------------
fn calc_oversampling(reqd : u8) -> u8 {
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
        panic!("Invalid oversample {} for BME688", reqd);
    }
    result
}

//----------------------------------------------------------------------------------------------------------------------------------
fn two_to_pow(exp : i8) -> f64 {
    return 2.0_f64.powi(exp as i32)
}

//----------------------------------------------------------------------------------------------------------------------------------
impl Bme688 {

    pub fn new(dev_name : &str) -> Self {
        let dev = match LinuxI2CDevice::new(dev_name, BME688_ADDR) {
            Ok(dev) => dev,
            Err(error) => panic!("Failed to open {} with address {} - {}", dev_name, BME688_ADDR, error)
        };

        Self {
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
        }
    }

    //------------------------------------------------------------------------------------------------------------------------------
    fn read_u8(&mut self, addr : u8) -> Result<u8> {
        Ok(self.dev.smbus_read_byte_data(addr)?)
    }

    //------------------------------------------------------------------------------------------------------------------------------
    fn write_u8(&mut self, addr :u8, value : u8) -> Result<()> {
        Ok(self.dev.smbus_write_byte_data(addr, value)?)
    }

    //------------------------------------------------------------------------------------------------------------------------------
    fn read_u16_le(&mut self, addr : u8) -> Result<u16> {
        // little-endian
        Ok(((self.read_u8(addr+1)? as u16) << 8) | (self.read_u8(addr)? as u16))
    }

    //------------------------------------------------------------------------------------------------------------------------------
    fn read_u16_be(&mut self, addr : u8) -> Result<u16> {
        // big-endian
        Ok(((self.read_u8(addr)? as u16) << 8) | (self.read_u8(addr + 1)? as u16))
    }

    //------------------------------------------------------------------------------------------------------------------------------
    fn read_i8(&mut self, addr : u8) -> Result<i8> {
        Ok(self.dev.smbus_read_byte_data(addr)? as i8)
    }

    //------------------------------------------------------------------------------------------------------------------------------
    fn read_i16_le(&mut self, addr : u8) -> Result<i16> {
        Ok(self.read_u16_le(addr)? as i16)
    }


    //------------------------------------------------------------------------------------------------------------------------------
    ///
    /// From the Datasheet...
    /// var1 = ((temp_adc / 16384) - (par_t1 / 1024)) * par_t2;
    /// var2 = (((temp_adc / 131072) - (par_t1 / 8192)) * ((temp_adc / 131072) - (par_t1 / 8192))) * (par_t3 * 16);
    /// t_fine = var1 + var2;
    /// temp = t_fine / 5120.0
    ///
    fn cache_temperature_params(&mut self) -> Result<()> {
        let par_t1 = self.read_u16_le(0xE9)?;
        let par_t2 = self.read_i16_le(0x8A)?;
        let par_t3 = self.read_i8(0x8C)?;

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
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    ///
    /// From the Datasheet...
    /// temp = t_fine * 5120
    /// var1 = (t_fine / 2) - 64000;
    /// var1a = var1 * var1 * (par_p6 / 131072);
    /// var1b = var1a + (var1 * par_p5 * 2);
    /// var2 = (var1b / 4) + (par_p4 * 65536);
    ///
    fn cache_pressure_params2(&mut self) -> Result <()> {
        let par_p4 = self.read_i16_le(0x94)?;
        let par_p5 = self.read_i16_le(0x96)?;
        let par_p6 = self.read_i8(0x99)?;

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
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    ///
    /// From the Datasheet...
    /// t_fine = temp * 5120
    /// var1 = (t_fine / 2) - 64000;
    /// var1a = (((par_p3 * var1 * var1) / 16384) + (par_p2 * var1)) / 524288;
    /// var1b = (1 + (var1a / 32768)) * par_p1;
    ///
    fn cache_pressure_params1(&mut self) -> Result<()> {
        let par_p1 = self.read_u16_le(0x8E)?;
        let par_p2 = self.read_i16_le(0x90)?;
        let par_p3 = self.read_i8(0x92)?;

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
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    ///
    /// From the Datasheet...
    /// var1 = (par_p9 * press_comp * press_comp) / 2147483648;
    /// var2 = press_comp * (par_p8 / 32768);
    /// var3 = (press_comp / 256) * (press_comp / 256) * (press_comp / 256) * (par_p10 / 131072);
    /// press_comp = press_comp + (var1_p + var2_p + var3_p + (par_p7 * 128)) / 16;
    ///
    fn cache_pressure_params3(&mut self) -> Result<()> {

        let par_p7 = self.read_i8(0x98)?;
        let par_p8 = self.read_i16_le(0x9C)?;
        let par_p9 = self.read_i16_le(0x9E)?;
        let par_p10 = self.read_i8(0xA0)?;


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
        Ok(())

    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn cache_humditiy_params(&mut self) -> Result<()>{

        let tmp = self.read_u8(0xE2)? as u16;
        let par_h1 = ((self.read_u8(0xE3)? as u16) << 4) | (tmp & 0x0F);
        let par_h2 = ((self.read_u8(0xE1)? as u16) << 4) | (tmp >> 4);
        let par_h3 = self.read_i8(0xE4)?;
        let par_h4 = self.read_i8(0xE5)?;
        let par_h5 = self.read_i8(0xE6)?;
        let par_h6 = self.read_i8(0xE7)?;
        let par_h7 = self.read_i8(0xE8)?;

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
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn cache_params(&mut self) -> Result<()>{
        self.cache_temperature_params()?;
        self.cache_pressure_params1()?;
        self.cache_pressure_params2()?;
        self.cache_pressure_params3()?;
        self.cache_humditiy_params()?;
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn set_humdity_oversampling(&mut self, oversampling : u8) {
        self.hum_oversampling = calc_oversampling(oversampling);
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn set_temperature_oversampling(&mut self, oversampling : u8) {
        self.temp_oversampling = calc_oversampling(oversampling);
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn set_pressure_oversampling(&mut self, oversampling : u8) {
        self.pres_oversampling = calc_oversampling(oversampling);
    }



    //------------------------------------------------------------------------------------------------------------------------------
    fn read_temp_adc(&mut self, field :u8) -> Result<i32> {
        let base : u8 = 0x22 + 0x11 * field;
        // Big-endian!
        let msb = self.read_u8(base + 0x00)? as u32;
        let lsb = self.read_u8(base + 0x01)? as u32;
        let xlsb = self.read_u8(base + 0x02)? as u32;

        let adc = ((msb << 12) | (lsb << 4) | (xlsb >> 4)) as i32;
//        println!("temp_adc {:?}", adc);
        Ok(adc)
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn read_press_adc(&mut self, field :u8) -> Result<i32> {
        let base : u8 = 0x1F + 0x11 * field;
        // Big-endian!
        let msb = self.read_u8(base + 0x00)? as u32;
        let lsb = self.read_u8(base + 0x01)? as u32;
        let xlsb = self.read_u8(base + 0x02)? as u32;

        let adc = ((msb << 12) | (lsb << 4) | (xlsb >> 4)) as i32;
//        println!("press_adc {:?}", adc);
        Ok(adc)
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn read_humd_adc(&mut self, field :u8) -> Result<u16> {
        let base : u8 = 0x25 + 0x11 * field;
        // Big-endian!
        let adc = self.read_u16_be(base)?;
//        println!("humd_adc {:?}", adc);
        Ok(adc)
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn read_temp(&mut self, field :u8) -> Result<f32> {

        let adc = self.read_temp_adc(field)? as f64;

        let temp = self.par_ta * adc * adc + self.par_tb * adc + self.par_tc;

        if self.temperature != temp {
            self.temperature = temp;
            self.cache_press_temp_vars();
            self.cache_humd_temp_vars();
        }

        // println!("Temperature is {:.2} C", temp);
        Ok(temp as f32)
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn cache_press_temp_vars(&mut self) {

        let temp = self.temperature;
        let var2 = self.par_pvar2a  * temp * temp + self.par_pvar2b  * temp + self.par_pvar2c;

        self.par_pvar2 = var2 as i64;
//        println!("var2 = {:?}", self.par_pvar2);

        let var1 = self.par_pvar1a  * temp * temp + self.par_pvar1b  * temp + self.par_pvar1c;

        self.par_pvar1 = var1;
//        println!("var1 = {:?}", self.par_pvar1);
    }


    //------------------------------------------------------------------------------------------------------------------------------
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


    //------------------------------------------------------------------------------------------------------------------------------
    fn read_press(&mut self, field: u8) -> Result<f32> {

        let adc = self.read_press_adc(field)?;

        let comp = (((1048576 - adc) as i64 - self.par_pvar2) as f64) / self.par_pvar1;

        let pressure = comp * comp * comp * self.par_pa + comp * comp * self.par_pb + comp * self.par_pc + self.par_pd;

        let pressure = pressure + 28_f64;
        // println!("Pressure {:.0} millibars", pressure);
        Ok(pressure as f32)
    }


    //------------------------------------------------------------------------------------------------------------------------------
    fn read_humd(&mut self, field: u8) -> Result<f32> {

        let adc = self.read_humd_adc(field)? as i32;

        // Becomes
        // var1 = humd_adc - var3;
        // var2 = var1 * var4;
        // humd_comp = var2 + var5 * var2 * var2);
        let var1 = (adc - self.par_hvar3) as f64;
        let var2 = var1 * self.par_hvar4;
        let humdity = var2 + self.par_hvar5 * var2 * var2;

        // println!("Humdity {:.2}%", humdity);
        Ok(humdity as f32)
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn sample(&mut self) -> Result<Summary> {
        let temp = self.read_temp(0)?;
        let press = self.read_press(0)?;
        let humd = self.read_humd(0)?;
        Ok(Summary::new(temp, humd, press))
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn one_shot(&mut self) -> Result<()> {
        // Write Humdity oversampling

        self.write_u8(0x72, self.hum_oversampling)?;

        // Write Pressure & temperature oversampling
        let tmp = (self.temp_oversampling << 5) | (self.pres_oversampling << 2);
        self.write_u8(0x74, tmp)?;

        self.write_u8(0x74, tmp | 1)?;
        Ok(())
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn is_ready(&mut self) -> Result<bool> {
        let mode = self.read_u8(0x74)? & 0x03;
        Ok(if mode == 0 {
            true
        } else {
            false
        })
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
        let mut sensor = Bme688::new("/dev/i2c-bme688");

        sensor.cache_params().unwrap();

        sensor.set_humdity_oversampling(16);
        sensor.set_pressure_oversampling(16);
        sensor.set_temperature_oversampling(16);


        sensor.one_shot().unwrap();
        loop {
            thread::sleep(Duration::from_secs(1));
            if sensor.is_ready().unwrap() {
                break;
            }
        }
        let summary = sensor.sample().unwrap();

        println!("{}", summary);
    }
}

