use chrono::DateTime;
use clock;

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

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn new(accum : &Accumulated, ticker : &clock::Clock) -> Summary {
        Summary {
            max_value : accum.max_value,
            min_value : accum.min_value,
            ave_value : (accum.sum / (accum.num_of as f64)) as f32,
            unix_time : ticker.get_nearest_tick()
        }
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn print(& self) {
        let dt = DateTime::from_timestamp(self.unix_time, 0).expect("invalid timestamp");
        println!("{} {} {} {}", dt, self.max_value, self.ave_value, self.min_value);
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn sql_insert_cmd(& self, table : &str) -> String {
        format!("INSERT INTO {} VALUES ({},{},{},{});",
            table, self.unix_time, self.max_value, self.ave_value, self.min_value)
    }
}



impl Accumulated {

    //------------------------------------------------------------------------------------------------------------------------------
    pub const fn new() -> Self {
        Accumulated {
            max_value : 0.0,
            min_value : 0.0,
            sum : 0.0,
            num_of : 0
        }
    }


    //------------------------------------------------------------------------------------------------------------------------------
    pub fn add(&mut self, value : f32) {
        if self.num_of > 0 {
            if value > self.max_value {
                self.max_value = value;
            } else if value < self.min_value {
                self.min_value = value;
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

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn sample(&mut self, ticker : &clock::Clock) -> Summary {
        let result = Summary::new(&self, ticker);
        result.print();
        self.num_of = 0;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn check_accumulated_and_summary() {
        let mut acc = Accumulated::new();
        acc.add(20.4);
        acc.add(10.6);
        acc.add(5.6);
        assert_eq!(acc.max_value, 20.4);
        assert_eq!(acc.min_value, 5.6);
        assert_eq!(acc.num_of, 3);
        assert_relative_eq!(acc.sum, 20.4 + 10.6 + 5.6, max_relative = 0.01);

        let summary = acc.sample(&clock::Clock::new(60*15));
        assert_eq!(acc.num_of, 0);

        assert_eq!(summary.max_value, 20.4);
        assert_eq!(summary.min_value, 5.6);
        assert_relative_eq!(summary.ave_value, (20.4 + 10.6 + 5.6) / 3.0, max_relative = 0.01);


        acc.add(3.4);
        acc.add(9.6);
        assert_eq!(acc.max_value, 9.6);
        assert_eq!(acc.min_value, 3.4);
        assert_eq!(acc.num_of, 2);
        assert_relative_eq!(acc.sum, 9.6 + 3.4, max_relative = 0.01);
    }
}
