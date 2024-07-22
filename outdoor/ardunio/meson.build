project('ardunio_wind', 'c')

srcs = ['main.c']

exec = executable('ardunio_wind.elf', srcs, build_by_default : false)

objcopy = find_program('objcopy')

custom_target('ardunio_wind',
  output : 'ardunio_wind.hex',
  depends : exec,
  input : exec,
  command : [objcopy, '-O', 'ihex', '-R', '.eeprom', '@INPUT@', '@OUTPUT@'],
  build_by_default : true
)
