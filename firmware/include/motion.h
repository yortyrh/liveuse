/**
 * Relative HID motion. MOVE sends the delta in small steps.
 */
#pragma once

#include <Arduino.h>
#include <Adafruit_TinyUSB.h>
#include <stdint.h>

/** ~125 Hz feel — matches common HID mouse poll intervals. */
static constexpr uint16_t kReportIntervalMs = 8;

/** Emit one relative mouse report; returns false if HID not ready. */
bool hidSendRelative(Adafruit_USBD_HID &hid, int8_t dx, int8_t dy);

/**
 * Glide in a straight line. Steps stay small so the pointer travels
 * instead of jumping. The gap between reports is 1 ms.
 */
void motionMove(Adafruit_USBD_HID &hid, int16_t total_dx, int16_t total_dy);

/**
 * Press and release a mouse button `times` times.
 * `times` 2 is a double click. Gaps stay inside a typical OS double-click window.
 */
void motionClick(Adafruit_USBD_HID &hid, uint8_t buttons, uint8_t times);

/** Replace the held button mask. Later moves keep these buttons down. */
void motionSetButtons(Adafruit_USBD_HID &hid, uint8_t buttons);

/**
 * Vertical wheel. Positive is up, negative is down. One notch per HID report.
 */
void motionScroll(Adafruit_USBD_HID &hid, int16_t steps);

/** Horizontal wheel. Positive is right, negative is left. */
void motionHScroll(Adafruit_USBD_HID &hid, int16_t steps);
