use chrono::{Utc, Timelike};

pub struct Clock {
    period_in_secs : i32
}

impl Clock {
    pub fn new(period_in_secs : i32) -> Self {
        if 60 * 60 % period_in_secs != 0 {
            panic!("Period must be a factor of 1 hour");
        }
        Clock {
            period_in_secs
        }
    }

    pub fn get_nearest_tick(&self) -> i64 {
        let now = Utc::now();
        let secs : i32 = (now.second() + 60 * now.minute()) as i32;
        let mut secs_adj = secs % self.period_in_secs;
        if secs_adj > (self.period_in_secs/2) {
            secs_adj -= self.period_in_secs;
        }
        now.timestamp() - (secs_adj as i64)
    }

    pub fn secs_to_next_tick(&self) -> u32 {
        let now = chrono::Utc::now();

        let secs = (now.second() + 60 * now.minute()) as i32;
        let delay = (self.period_in_secs - secs % self.period_in_secs) as u32;
        println!("Duration {}", delay);
        delay
    }
}


#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc, Datelike};
    use super::*;

    #[test]
    fn check_nearest_tick() {
        let ticker = Clock::new(60*15);

        let now = Utc::now();
        let unix_time = ticker.get_nearest_tick();

        let now_test = DateTime::from_timestamp(unix_time, 0).expect("invalid timestamp");
        assert_eq!(now.year_ce(), now_test.year_ce());
        assert_eq!(now.month(), now_test.month());
        assert_eq!(now.day(), now_test.day());
        assert_eq!(now.hour(), now_test.hour());
        assert_eq!(0, now_test.second());

        if now.minute() < 8 {
            assert_eq!(now_test.minute(), 0)
        } else if now.minute() < 23 {
            assert_eq!(now_test.minute(), 15)
        } else if now.minute() < 38 {
            assert_eq!(now_test.minute(), 30)
        } else {
            assert_eq!(now_test.minute(), 45)
        }
    }

    #[test]
    fn secs_to_next_tick() {
        let ticker = Clock::new(60*15);

        let now = Utc::now();
        let delay_to_hour = 60 * 60 - 60 * now.minute() - now.second();
        let delay = ticker.secs_to_next_tick();

        if now.minute() < 15 {
            assert_eq!(delay, delay_to_hour - 60 * 45)
        } else if now.minute() < 30 {
            assert_eq!(delay, delay_to_hour - 60 * 30)
        } else if now.minute() < 45 {
            assert_eq!(delay, delay_to_hour - 60 * 15)
        } else {
            assert_eq!(delay, delay_to_hour)
        }
    }


}
