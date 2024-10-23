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

//----------------------------------------------------------------------------------------------------------------------------------
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


//----------------------------------------------------------------------------------------------------------------------------------
// If send UART is ready and there is data in the send buffer, send it
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

//----------------------------------------------------------------------------------------------------------------------------------
// Add byte to the serial send buffer
void serial_write(char byte)
{
	int idx = SerialBuffer.headIdx;
	int next_idx = (idx < SERIAL_BUFFER_SIZE-1) ? idx + 1 : 0;

	if (next_idx != SerialBuffer.tailIdx) {
		SerialBuffer.buffer[idx] = byte;
		SerialBuffer.headIdx = next_idx;
	}
}

//----------------------------------------------------------------------------------------------------------------------------------
// Add character string  to the serial send buffer
void serial_writeln(char * str)
{
	while (*str != '\0') {
		serial_write(*str++);
	}
}

/*
1 contact closure per 1.25 m/s
100 mph = 44.704 m/s = 35.76 Hz

1 mph = (60 * 60 * 10000)/(63360 * 254) m/s
1 mph = (60 * 60 * 10000)/(64 * 99 * 10 * 254)
1 mph = (56250)/(99 * 254)
1 mph = (28125)/(9 * 11 * 127)
1 mph = 3125/1397
*/


typedef struct {
	unsigned int resolution;
} T_SpeedReporter;

T_SpeedReporter Speed;

void init_speed_reporter(unsigned int resolution)
{
	Speed.resolution = resolution;
}


//----------------------------------------------------------------------------------------------------------------------------------
void report_speed(unsigned int speed)
{
	unsigned int tens = 10000U;
	char leading_ch = ' ';
	int idx = 0;
	char buffer[10];
	
	while (tens > 0) {
		unsigned int digit = speed / tens;
		speed -= digit * tens;
		const int point = (tens == Speed.resolution);
		if (point) {
			leading_ch = '0';
		}
		if (digit == 0) {
			buffer[idx] = leading_ch;
		} else {
			leading_ch = '0';
			if (digit <= 9) {
				buffer[idx] = digit + '0';
			} else {
				buffer[idx] = '?';
			}
		}
		idx++;
		if (point) {
			buffer[idx++] = '.';
		}
		tens /= 10UL;
	}
	buffer[idx++] = '\n';
	buffer[idx++] = '\0';
	serial_writeln(buffer);
}

//----------------------------------------------------------------------------------------------------------------------------------
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
	return (one_second * 5) / 4;
}


//----------------------------------------------------------------------------------------------------------------------------------
static unsigned long delta_time(void)
{
	static unsigned int prev_time = 0;

	const unsigned int now_time = TCNT1;
	const unsigned int delta = now_time - prev_time;
	prev_time = now_time;
	return delta;
}

//----------------------------------------------------------------------------------------------------------------------------------
static int has_rotated(void)
{
	static unsigned char prev_raw_state = 0;

	const unsigned char state = PINB &_BV(PORTB5);

	int rotated = 0;
	if (prev_raw_state != state) {
		// Pull Up, so zero means contact closed
		if (state == 0) {
			rotated = 1;
		}
		prev_raw_state = state;
	}
	return rotated;
}


//----------------------------------------------------------------------------------------------------------------------------------
int main(void)
{
	const unsigned int resolution = 10;

	init_serial();
	init_speed_reporter(resolution);

	unsigned long one_second;
	const unsigned long step = resolution * ticks_per_metre(3, &one_second);
	const unsigned long debounce_period = one_second/50;

	const unsigned long max_count = 0xFFFFFFFFUL - step;
	// Some test code
	// Set Port B as inputs
	DDRB = 0; // _BV(DDB5);

	// Select pull up
	PORTB |= _BV(PORTB5);

	unsigned long count = 0;
	unsigned long time = 0;
	const unsigned long measurement_period = 2 * one_second;

	serial_writeln("Start\n");

	while(1) {

		// Wait for start of first rotation..
		while(1) {
			if (has_rotated()) {
				count = 0;
				delta_time();
				time = 0;
				break;
			}
			
			/* empty UART transmit buffer */
			run_serial();
		}

		while(1) {

			time += delta_time();

			if (has_rotated()) {
				count += step;
				if (count >= max_count) {
					break;
				}
			}

			if (time > measurement_period) {
				break;
			}

			/* empty UART transmit buffer */
			run_serial();
		}
		
		report_speed((count + time/2)/time);
	}
}
