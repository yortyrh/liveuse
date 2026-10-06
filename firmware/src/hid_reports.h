#pragma once

#include <cstdint>

// Instance 0 is the first Adafruit_USBD_HID::begin(), instance 1 the second.
void hid_report_set(uint8_t instance, uint8_t const *desc);
