/**
 * Smooth relative HID motion helpers (human-like small dx/dy steps).
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
 * Smoothly walk from (0,0) to (total_dx, total_dy) using many small reports.
 * Blocks until the path finishes (delay() between reports).
 */
void motionSmoothMove(Adafruit_USBD_HID &hid, int16_t total_dx, int16_t total_dy);

/** Parametric circle as successive relative deltas. */
void motionDemoCircle(Adafruit_USBD_HID &hid);

/** Closed square path (right/down/left/up) via small relative steps. */
void motionDemoZigzag(Adafruit_USBD_HID &hid);

/**
 * Press and release a mouse button `times` times.
 * `times` 2 is a double click. Gaps stay inside a typical OS double-click window.
 */
void motionClick(Adafruit_USBD_HID &hid, uint8_t buttons, uint8_t times);

/**
 * Vertical wheel. Positive is up, negative is down. One notch per HID report.
 */
void motionScroll(Adafruit_USBD_HID &hid, int16_t steps);
