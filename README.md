# weather_station

Weather station based on raspberry pi and perhaps later an arduino(s)

## Indoor sensors
 - Temperaure
 - Humdity
 - Pressure
 - Gas

Using BME688 module connected to Raspberry pi.

### Steps

1. Decide which pins to use on raspberry pi for the i2c interface.

For me, this had to be I/O pins 9 & 10 as the only I2c port was used for the RTC.
So since this will be bit banged I2C, the appropriate overlay has to be configured
in /boot/config.txt like so:

> dtoverlay=i2c-gpio,bus=4,i2c_gpio_delay=2,i2c_gpio_sda=10,i2c_gpio_scl=9

Note, the bus is set to 4. Adjust if necessary

2. Check i2c-dev module is loaded

For me this was not auto-loaded, so the module i2c-dev had to
be added to list of modules to load like so:

Create and add *i2c_dev* to */etc/modules-load.d/sensors.conf*

After a reboot the devices */dev/ic2-** should now appear. The * is the i2c bus number

3. Update udev rules

I wanted to create a symbol link */dev/i2c_bme688* with right permissions in dev. To do this
requires a udev rule. Create a file e.g. */etc/udev/rules.d/99_my.rules*

Add the following line.

> SUBSYSTEM=="i2c-dev", ACTION=="add", ATTR{name}=="4.i2c", SYMLINK+= "i2c-bme688", MODE="666"

Note, the name was discovered by running the command *udevadmin info -a /dev/i2c-4*

4. Build the rust project ....
