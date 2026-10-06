/**
 * Relative HID motion.
 */
#include "motion.h"

#include <math.h>

static void clampToInt8(float fx, float fy, int8_t *dx, int8_t *dy) {
  if (fx > 127.0f) fx = 127.0f;
  if (fx < -127.0f) fx = -127.0f;
  if (fy > 127.0f) fy = 127.0f;
  if (fy < -127.0f) fy = -127.0f;
  *dx = static_cast<int8_t>(lroundf(fx));
  *dy = static_cast<int8_t>(lroundf(fy));
}

static void wakeHid() {
#if defined(USBDevice)
  if (USBDevice.suspended()) {
    USBDevice.remoteWakeup();
  }
#elif defined(TinyUSBDevice)
  if (TinyUSBDevice.suspended()) {
    TinyUSBDevice.remoteWakeup();
  }
#endif
}

bool hidSendRelative(Adafruit_USBD_HID &hid, int8_t dx, int8_t dy) {
  if (dx == 0 && dy == 0) {
    return true;
  }

  wakeHid();
  if (!hid.ready()) {
    return false;
  }
  // report_id 0: single-report mouse descriptor (no Report ID byte)
  return hid.mouseMove(0, dx, dy);
}

static bool sendWhenReady(Adafruit_USBD_HID &hid, bool (*send)(Adafruit_USBD_HID &)) {
  for (int attempt = 0; attempt < 5; ++attempt) {
    wakeHid();
    if (hid.ready() && send(hid)) {
      return true;
    }
    delay(1);
  }
  return false;
}

static uint8_t s_clickButtons = 0;
static int8_t s_scrollNotch = 0;

static bool sendClickPress(Adafruit_USBD_HID &hid) {
  return hid.mouseButtonPress(0, s_clickButtons);
}

static bool sendClickRelease(Adafruit_USBD_HID &hid) {
  return hid.mouseButtonRelease(0);
}

static bool sendScrollNotch(Adafruit_USBD_HID &hid) {
  return hid.mouseScroll(0, s_scrollNotch, 0);
}

void motionClick(Adafruit_USBD_HID &hid, uint8_t buttons, uint8_t times) {
  if (times < 1) {
    times = 1;
  }
  s_clickButtons = buttons;

  for (uint8_t i = 0; i < times; ++i) {
    if (i > 0) {
      // Stay inside a typical OS double-click window.
      delay(80);
    }
    sendWhenReady(hid, sendClickPress);
    delay(40);
    sendWhenReady(hid, sendClickRelease);
    delay(20);
  }
}

void motionSetButtons(Adafruit_USBD_HID &hid, uint8_t buttons) {
  for (int attempt = 0; attempt < 5; ++attempt) {
    wakeHid();
    if (hid.ready() && hid.mouseReport(0, buttons, 0, 0, 0, 0)) {
      return;
    }
    delay(1);
  }
}

void motionScroll(Adafruit_USBD_HID &hid, int16_t steps) {
  int16_t left = steps;
  while (left != 0) {
    if (left > 0) {
      s_scrollNotch = 1;
      --left;
    } else {
      s_scrollNotch = -1;
      ++left;
    }
    sendWhenReady(hid, sendScrollNotch);
    delay(kReportIntervalMs);
  }
}

void motionHScroll(Adafruit_USBD_HID &hid, int16_t steps) {
  int16_t left = steps;
  while (left != 0) {
    const int8_t notch = left > 0 ? 1 : -1;
    if (left > 0) {
      --left;
    } else {
      ++left;
    }
    for (int attempt = 0; attempt < 5; ++attempt) {
      wakeHid();
      if (hid.ready() && hid.mouseScroll(0, 0, notch)) {
        break;
      }
      delay(1);
    }
    delay(kReportIntervalMs);
  }
}

void motionMove(Adafruit_USBD_HID &hid, int16_t total_dx, int16_t total_dy) {
  // ~8 px/report keeps the path visible. 1 ms between reports is a glide,
  // not a ±127 jump.
  const float dist = sqrtf(static_cast<float>(total_dx) * static_cast<float>(total_dx) +
                           static_cast<float>(total_dy) * static_cast<float>(total_dy));
  if (dist < 0.5f) {
    return;
  }
  uint16_t steps = static_cast<uint16_t>(lroundf(dist / 8.0f));
  if (steps < 1) {
    steps = 1;
  }

  float acc_x = 0.0f;
  float acc_y = 0.0f;
  float sent_x = 0.0f;
  float sent_y = 0.0f;

  for (uint16_t i = 1; i <= steps; ++i) {
    const float t = static_cast<float>(i) / static_cast<float>(steps);
    const float target_x = static_cast<float>(total_dx) * t;
    const float target_y = static_cast<float>(total_dy) * t;

    acc_x += target_x - sent_x;
    acc_y += target_y - sent_y;

    int8_t dx = 0;
    int8_t dy = 0;
    clampToInt8(acc_x, acc_y, &dx, &dy);
    acc_x -= static_cast<float>(dx);
    acc_y -= static_cast<float>(dy);
    sent_x += static_cast<float>(dx);
    sent_y += static_cast<float>(dy);

    for (int attempt = 0; attempt < 5; ++attempt) {
      if (hidSendRelative(hid, dx, dy)) {
        break;
      }
      delay(1);
    }
    delay(1);
  }
}
