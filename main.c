#include <avr/io.h>
#include <util/delay.h>

int main(void) {


	// Some test code
	DDRB |= _BV(DDB5);

	while(1) {

		PORTB |= _BV(PORTB5);

		_delay_ms(3000);

		PORTB &= _BV(PORTB5);
		
		_delay_ms(3000);
	}
}
