//use chrono;
use chrono::{DateTime, Utc};
use chrono::Timelike;


pub struct Accumulated {
    max_value : f32,
    min_value : f32,
    sum : f64,
    num_of : u16,
}

pub struct Summary {
    max_value : f32,
    min_value : f32,
    ave_value : f32,
    unix_time : i64
}


impl Summary {
    pub fn new(accum : &Accumulated, period_in_secs : i32) -> Summary {
        let now = chrono::Utc::now();
        let secs : i32 = (now.second() + 60 * now.minute()) as i32;
        let mut secs_adj = secs % period_in_secs;
        if secs_adj > (period_in_secs/2) {
            secs_adj -= period_in_secs;
        }
        Summary {
            max_value : accum.max_value,
            min_value : accum.min_value,
            ave_value : (accum.sum / (accum.num_of as f64)) as f32,
            unix_time : now.timestamp() - (secs_adj as i64)
        }
    }

    pub fn print(& self) {
        let dt = DateTime::from_timestamp(self.unix_time, 0).expect("invalid timestamp");
        println!("{} {} {} {}", dt, self.max_value, self.ave_value, self.min_value);
    }
}



impl Accumulated {

    pub const fn new() -> Self {
        Accumulated {
            max_value : 0.0,
            min_value : 0.0,
            sum : 0.0,
            num_of : 0
        }
    }

    pub fn reset(&mut self) {
        self.num_of = 0;
    }

    pub fn add(&mut self, value : f32) {
        if self.num_of > 0 {
            if value > self.max_value {
                self.max_value = value;
            } else {
                if value < self.min_value {
                    self.min_value = value;
                }
            }
            self.num_of += 1;
            self.sum += value as f64;
        } else {
            self.max_value = value;
            self.min_value = value;
            self.sum = value as f64;
            self.num_of = 1;
        }
    }

    pub fn sample(&mut self, period_in_secs : i32) -> Summary {
        let result = Summary::new(&self, period_in_secs);
        result.print();
        self.num_of = 0;
        result
    }
}
