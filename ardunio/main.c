#include <avr/io.h>
#include <util/delay.h>

#define FOSC 16000000 // Clock Speed
#define BAUD 9600

#define SERIAL_BUFFER_SIZE  (256)

typedef struct {
	int headIdx;
	int tailIdx;
	char buffer[SERIAL_BUFFER_SIZE];
} T_SerialBuffer;

T_SerialBuffer SerialBuffer;


void init_serial(void)
{
	const unsigned int ubrr = FOSC/16/BAUD-1;

	/*Set baud rate */
	UBRR0H = (unsigned char)(ubrr>>8);
	UBRR0L = (unsigned char)ubrr;

	/* Enable receiver and transmitter */
	UCSR0B = (1<<RXEN0) | (1<<TXEN0);

	/* Set frame format: 8data, 2stop bit */
	UCSR0C = (1<<USBS0) | (3<<UCSZ00);

	SerialBuffer.headIdx = SerialBuffer.tailIdx = 0;
}


void run_serial(void)
{
	if ((UCSR0A & (1<<UDRE0))) {
		const int idx = SerialBuffer.tailIdx;

		if (idx != SerialBuffer.headIdx) {
			UDR0 = SerialBuffer.buffer[idx];
			SerialBuffer.tailIdx = (idx < SERIAL_BUFFER_SIZE-1) ? idx + 1 : 0;
		}
	}
}

void serial_write(char byte)
{
	int idx = SerialBuffer.headIdx;
	int next_idx = (idx < SERIAL_BUFFER_SIZE-1) ? idx + 1 : 0;

	if (next_idx != SerialBuffer.tailIdx) {
		SerialBuffer.buffer[idx] = byte;
		SerialBuffer.headIdx = next_idx;
	}
}

void serial_writeln(char * str)
{
	while (*str != '\0') {
		serial_write(*str++);
	}
}

/*
1 contact closure per 1.25 m/s
100 mph = 44.704 m/s = 35.76 Hz
*/

typedef struct {
	unsigned int tens;
	unsigned int resolution;
	unsigned int value;
	char buffer[10];
	int idx;
	char leading_ch;
} T_SpeedReporter;

T_SpeedReporter Speed;

void init_speed_reporter(unsigned int resolution)
{
	Speed.tens = 0;
	Speed.resolution = resolution;
}

void run_speed_reporter(void)
{
	unsigned int tens = Speed.tens;
	unsigned int value = Speed.value;

	if (tens > 0) {
		unsigned int digit = value / tens;
		value -= digit * tens;
		int idx = Speed.idx;
		const int point = (tens == Speed.resolution);
		if (point) {
			Speed.leading_ch = '0';
		}
		if (digit == 0) {
			Speed.buffer[idx] = Speed.leading_ch;
		} else {
			Speed.leading_ch = '0';
			if (digit <= 9) {
				Speed.buffer[idx] = digit + '0';
			} else {
				Speed.buffer[idx] = '?';
			}
		}
		idx++;
		if (point) {
			Speed.buffer[idx++] = '.';
		}
		tens /= 10UL;
		if (tens == 0) {
			Speed.buffer[idx++] = '\n';
			Speed.buffer[idx++] = '\0';
			serial_writeln(Speed.buffer);
		} else {
			Speed.value = value;
			Speed.idx = idx;
		}
		Speed.tens = tens;
	}
}

void report_speed(unsigned int speed)
{
	unsigned int tens = Speed.tens;
	if (tens == 0) {
		Speed.value = speed;
		Speed.idx = 0;
		Speed.tens = 10000UL;
		Speed.leading_ch = ' ';
	}
}

unsigned long ticks_per_metre(unsigned char val, unsigned long * pOneSecond)
{
	TCCR1B = val;

	unsigned long divider = 1;

	switch(val) {
		case 1: divider = 1; break;
		case 2: divider = 8; break;
		case 3: divider = 64; break;
		case 4: divider = 256; break;
		case 5: divider = 1024; break;
	}
	unsigned long one_second = FOSC / divider;
	*pOneSecond = one_second;
	// 1 tick = FOSC/ divider, 1.25 metre per count
	return (one_second * 125) / 100;
}


unsigned long delta_time(void)
{
	static unsigned int prev_time = 0;

	const unsigned int now_time = TCNT1;
	const unsigned int delta = now_time - prev_time;
	prev_time = now_time;
	return delta;
}


int has_moved(void)
{
	static unsigned char prevData = 0;

	int moved = 0;
	const unsigned char data = PINB &_BV(PORTB5);
	if (prevData != data) {
		moved = 1;
		prevData = data;
	}
	return moved;
}


int main(void)
{
	const unsigned int resolution = 10;

	init_serial();
	init_speed_reporter(resolution);

	unsigned long one_second;
	const unsigned long step = resolution * ticks_per_metre(3, &one_second);

	const unsigned long max_count = 0xFFFFFFFFUL - step;
	// Some test code
	// Set Port B as inputs
	DDRB = 0; // _BV(DDB5);

	// Select pull up
	PORTB |= _BV(PORTB5);

	unsigned long count = 0;
	unsigned long time = 0;

	serial_writeln("Start\n");

	while(1) {
		/* Wait for empty transmit buffer */
		run_serial();

		run_speed_reporter();

		time += delta_time();

		if (has_moved()) {
			count += step;
		}

		if ((count >= max_count) || (time > 2 * one_second)) {
			report_speed((count + time/2)/time);
			count = 0;
			time = 0;
		}
	}
}
