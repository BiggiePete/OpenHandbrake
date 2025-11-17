#include "Arduino.h"
#include "HID-Project.h"

const int pinLed = LED_BUILTIN;
const int pinButton = 2;
void buttonChange();

void setup()
{
  pinMode(pinLed, OUTPUT);
  pinMode(pinButton, INPUT_PULLUP);

  // Sends a clean report to the host. This is important on any Arduino type.

  Gamepad.begin();
  Gamepad.releaseAll();
  attachInterrupt(PA_3, &buttonChange, CHANGE);
  pinMode(PB_4, INPUT_ANALOG);
}

void loop()
{
  if (!digitalRead(pinButton))
  {
    digitalWrite(pinLed, HIGH);

    // read in the ADC and get a 16 bit value to write to the x axis

    // Move x/y Axis to a new position (16bit)
    Gamepad.yAxis((uint16_t)analogRead(PB_4) - 512);

    // Functions above only set the values.
    // This writes the report to the host.
    Gamepad.write();

    // Simple debounce
    delay(100);
    digitalWrite(pinLed, LOW);
  }
}

// callback for if the button is pressed or changed
void buttonChange()
{
  // first lets get the state of the button
  int state = digitalRead(PA_3);
  if (state == HIGH)
    Gamepad.press(1);
  else
    Gamepad.release(1);
}