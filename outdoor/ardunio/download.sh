#!/usr/bin/env bash
avrdude -c arduino -P /dev/ttyACM0 -b 115200 -p atmega328p -U flash:w:build/ardunio_wind.hex:i

