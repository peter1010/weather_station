#include <avr/io.h>
#include <util/delay.h>

#define FOSC 16000000 // Clock Speed
// #define FOSC 1843200 // Clock Speed
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
	unsigned long tens;
	unsigned long value;
	char buffer[20];
	int idx;
} T_SpeedReporter;

T_SpeedReporter Speed;

void init_speed_reporter(void)
{
	Speed.tens = 0;
}

void run_speed_reporter(void)
{
	unsigned long tens = Speed.tens;
	unsigned long value = Speed.value;

	if (tens > 0) {
		unsigned long digit = value / tens;
		value -= digit * tens;
		int idx = Speed.idx;
		if (digit <= 9) {
			Speed.buffer[idx] = digit + '0';
		} else {
			Speed.buffer[idx] = '?';
		}
		idx++;
		tens /= 10UL;
		if (tens == 0) {
			Speed.buffer[idx++] = '\r';
			Speed.buffer[idx++] = '\0';
			serial_writeln(Speed.buffer);
		} else {
			Speed.value = value;
			Speed.idx = idx;
		}
		Speed.tens = tens;
	}
}

void report_speed(unsigned long speed)
{
	unsigned long tens = Speed.tens;
	if (tens == 0) {
		Speed.value = speed;
		Speed.idx = 0;
		Speed.tens = 1000000000UL;
	}
}

int main(void) {

	init_serial();
	init_speed_reporter();

	//Timer wraps every 1/3 of one second
	TCCR1B = 3;  // 16MHz / 64 => 250000

	// 1 tick = 4ms, step 1.25/0.000004
	unsigned long step = 31250000;

	// Some test code
	// Set Port B as inputs
	DDRB = 0; // _BV(DDB5);

	// Select pull up
	PORTB |= _BV(PORTB5);

	unsigned char prevData = PINB & _BV(PORTB5);
	unsigned long count = 0;
	unsigned long time = 0;
	unsigned int prev_time = 0;

	serial_writeln("Start\n");

	while(1) {
		/* Wait for empty transmit buffer */
		run_serial();

		run_speed_reporter();

		unsigned int now_time = TCNT1;
		unsigned int delta = now_time - prev_time;
		prev_time = now_time;

		time += delta;

		unsigned char data = PINB &_BV(PORTB5);
		if (prevData != data) {
			count += step;
			prevData = data;
		}

		if ((count > 10 * step) || (time > 100000)) {
			report_speed(count/time);
			count = 0;
			time = 0;
		}
	}

	while(1) {
		if ((count > 10) || (time > 10000)) {
			{
				unsigned int now_time = TCNT1;
				unsigned int delta = now_time - prev_time;
				prev_time = now_time;
				count = 0;

				unsigned long tens = 100000000UL;
				unsigned int i = 1;
				unsigned long temp = delta;
				while(tens > 0) {
					unsigned long digit = temp / tens;
					temp -= digit * tens;
					if (digit <= 9) {
						serial_write(digit + '0');
					} else {
						serial_write('*');
					}
					i++;
					tens /= 10UL;
				}
				serial_write('\r');
			}
		}
	}
}
