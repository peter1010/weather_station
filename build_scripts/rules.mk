
CC=avr-gcc -c
CXX=avr-g++ -c
LD=avr-gcc
BIN=avr-objcopy

CPPFLAGS=-DF_CPU=16000000UL

CFLAGS=-Os -mmcu=atmega328p

LDFLAGS=$(CFLAGS)

MK_DEPENDS=-MMD

.SUFFIXES:

%.o : %.c
	$(CC) $(CPPFLAGS) $(MK_DEPENDS) $(CFLAGS) -o $@ $<

%.o : %.cpp
	$(CXX) $(CPPFLAGS) $(MK_DEPENDS) $(CFLAGS) $(CXXFLAGS) -o $@ $<

#%.elf : $(OBJS)
#	$(LD) $(LDFLAGS) $(OBJS) -o $@

%.hex : %.elf
	@echo "*** Create Hex file ***"
	$(BIN) -O ihex -R .eeprom $< $@
