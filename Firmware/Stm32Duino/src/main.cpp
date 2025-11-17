#include "Arduino.h"
#include "HID-Project.h"

volatile bool buttonState = false;
void buttonChange();

void setup()
{
  pinMode(PB_4, INPUT_ANALOG);
  attachInterrupt(PA_3, &buttonChange, CHANGE);

  // Sends a clean report to the host. This is important on any Arduino type.

  Gamepad.begin();
  Gamepad.releaseAll();
}

void loop()
{

  // read in the ADC and get a 16 bit value to write to the x axis

  // Move x/y Axis to a new position (16bit)
  Gamepad.yAxis((uint16_t)analogRead(PB_4) - 512);

  // Functions above only set the values.
  // This writes the report to the host.
  switch (buttonState)
  {
  case true:
    Gamepad.press(1);
    break;
  case false:
    Gamepad.release(1);
    break;
  }
  Gamepad.write();

  // limit to 10 readings a second
  delay(100);
}

// callback for if the button is pressed or changed
void buttonChange()
{
  // first lets get the state of the button
  int state = digitalRead(PA_3);
  if (state == HIGH)
    buttonState = true;
  else
    buttonState = false;
}