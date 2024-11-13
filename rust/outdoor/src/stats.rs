use std::fmt;


//----------------------------------------------------------------------------------------------------------------------------------
pub struct Accumulated {
    max_value : f32,
    min_value : f32,
    sum : f64,
    num_of : u16,
}


//----------------------------------------------------------------------------------------------------------------------------------
pub struct Summary {
    max_value : f32,
    min_value : f32,
    ave_value : f32,
}


//----------------------------------------------------------------------------------------------------------------------------------
impl Summary {

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn new(accum : &Accumulated) -> Summary {
        if accum.num_of > 0 {
            Summary {
                max_value : accum.max_value,
                min_value : accum.min_value,
                ave_value : (accum.sum / (accum.num_of as f64)) as f32
            }
        } else {
            Summary {
                max_value : 0.0,
                min_value : 0.0,
                ave_value : 0.0
            }
        }
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_max(&self) -> f32 {
        self.max_value
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_min(&self) -> f32 {
        self.min_value
    }

    //------------------------------------------------------------------------------------------------------------------------------
    pub fn get_average(&self) -> f32 {
        self.ave_value
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({:.1} - {:.1} -  {:.1}) m/s", self.min_value, self.ave_value, self.max_value)
    }
}


//----------------------------------------------------------------------------------------------------------------------------------
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
    pub fn sample(&mut self) -> Summary {
        let result = Summary::new(&self);
        self.num_of = 0;
        result
    }
}

//----------------------------------------------------------------------------------------------------------------------------------
impl fmt::Debug for Accumulated {

    //------------------------------------------------------------------------------------------------------------------------------
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Acc({},{},{},{})", self.max_value, self.min_value, self.sum, self.num_of)
    }
}

//----------------------------------------------------------------------------------------------------------------------------------
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

        let summary = acc.sample();
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

    #[test]
    fn check_print_debug() {
        let acc = Accumulated::new();
        assert_eq!(format!("{:?}", acc), "Acc(0,0,0,0)");
    }
}
