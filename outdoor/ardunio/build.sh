#!/usr/bin/env bash

meson setup --wipe --cross-file ardunio.ini build
meson compile -C build
