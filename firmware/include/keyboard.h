#pragma once

#include <Adafruit_TinyUSB.h>

void keyboardType(Adafruit_USBD_HID &hid, const char *payload);
void keyboardChord(Adafruit_USBD_HID &hid, const char *chord);
