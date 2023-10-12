use crate::bme688::Bme688;

pub mod bme688;

fn main() {

    let mut drv =  Bme688::new().unwrap();

    drv.cache_params();

    drv.set_humdity_oversampling(16);
    drv.set_pressure_oversampling(16);
    drv.set_temperature_oversampling(16);

    drv.force();

    let temp = drv.read_temp(0);
    let temp = drv.read_press(0);
    let temp = drv.read_humd(0);

    drv.set_heater(0, 100);
}
