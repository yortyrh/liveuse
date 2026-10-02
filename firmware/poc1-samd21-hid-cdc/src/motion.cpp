/**
 * Smooth relative HID motion — many small dx/dy reports, never one giant jump.
 */
#include "motion.h"

#include <math.h>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

static constexpr float kCircleRadiusPx = 40.0f;
static constexpr uint16_t kCircleSteps = 120;
static constexpr uint16_t kZigzagSegmentSteps = 40;
static constexpr int8_t kZigzagStepPx = 2;

static void clampToInt8(float fx, float fy, int8_t *dx, int8_t *dy) {
  if (fx > 127.0f) fx = 127.0f;
  if (fx < -127.0f) fx = -127.0f;
  if (fy > 127.0f) fy = 127.0f;
  if (fy < -127.0f) fy = -127.0f;
  *dx = static_cast<int8_t>(lroundf(fx));
  *dy = static_cast<int8_t>(lroundf(fy));
}

bool hidSendRelative(Adafruit_USBD_HID &hid, int8_t dx, int8_t dy) {
  if (dx == 0 && dy == 0) {
    return true;
  }

#if defined(USBDevice)
  if (USBDevice.suspended()) {
    USBDevice.remoteWakeup();
  }
#elif defined(TinyUSBDevice)
  if (TinyUSBDevice.suspended()) {
    TinyUSBDevice.remoteWakeup();
  }
#endif

  if (!hid.ready()) {
    return false;
  }
  // report_id 0: single-report mouse descriptor (no Report ID byte)
  return hid.mouseMove(0, dx, dy);
}

void motionSmoothMove(Adafruit_USBD_HID &hid, int16_t total_dx, int16_t total_dy) {
  // Prefer ~2 px/report so paths feel human; scale step count with distance.
  const float dist = sqrtf(static_cast<float>(total_dx) * static_cast<float>(total_dx) +
                           static_cast<float>(total_dy) * static_cast<float>(total_dy));
  uint16_t steps = static_cast<uint16_t>(lroundf(dist / 2.0f));
  if (steps < 1) {
    steps = 1;
  }
  if (steps > 500) {
    steps = 500;
  }

  float acc_x = 0.0f;
  float acc_y = 0.0f;
  float sent_x = 0.0f;
  float sent_y = 0.0f;

  for (uint16_t i = 1; i <= steps; ++i) {
    const float t = static_cast<float>(i) / static_cast<float>(steps);
    // Ease-in-out (smoothstep) for slightly more human acceleration.
    const float e = t * t * (3.0f - 2.0f * t);
    const float target_x = static_cast<float>(total_dx) * e;
    const float target_y = static_cast<float>(total_dy) * e;

    float ddx = target_x - sent_x;
    float ddy = target_y - sent_y;
    acc_x += ddx;
    acc_y += ddy;

    int8_t dx = 0;
    int8_t dy = 0;
    clampToInt8(acc_x, acc_y, &dx, &dy);
    acc_x -= static_cast<float>(dx);
    acc_y -= static_cast<float>(dy);
    sent_x += static_cast<float>(dx);
    sent_y += static_cast<float>(dy);

    // Retry briefly if HID not ready (USB busy).
    for (int attempt = 0; attempt < 5; ++attempt) {
      if (hidSendRelative(hid, dx, dy)) {
        break;
      }
      delay(1);
    }
    delay(kReportIntervalMs);
  }
}

void motionDemoCircle(Adafruit_USBD_HID &hid) {
  float last_x = 0.0f;
  float last_y = 0.0f;

  for (uint16_t i = 0; i < kCircleSteps; ++i) {
    const float t0 = (2.0f * static_cast<float>(M_PI) * static_cast<float>(i)) /
                     static_cast<float>(kCircleSteps);
    const float t1 = (2.0f * static_cast<float>(M_PI) * static_cast<float>(i + 1)) /
                     static_cast<float>(kCircleSteps);

    const float x0 = kCircleRadiusPx * cosf(t0);
    const float y0 = kCircleRadiusPx * sinf(t0);
    const float x1 = kCircleRadiusPx * cosf(t1);
    const float y1 = kCircleRadiusPx * sinf(t1);

    if (i == 0) {
      last_x = x0;
      last_y = y0;
    }

    float ddx = x1 - last_x;
    float ddy = y1 - last_y;
    last_x = x1;
    last_y = y1;

    int8_t dx = 0;
    int8_t dy = 0;
    clampToInt8(ddx, ddy, &dx, &dy);

    for (int attempt = 0; attempt < 5; ++attempt) {
      if (hidSendRelative(hid, dx, dy)) {
        break;
      }
      delay(1);
    }
    delay(kReportIntervalMs);
  }
}

void motionDemoZigzag(Adafruit_USBD_HID &hid) {
  // right → down → left → up
  for (uint8_t leg = 0; leg < 4; ++leg) {
    for (uint16_t s = 0; s < kZigzagSegmentSteps; ++s) {
      int8_t dx = 0;
      int8_t dy = 0;
      switch (leg) {
        case 0:
          dx = kZigzagStepPx;
          break;
        case 1:
          dy = kZigzagStepPx;
          break;
        case 2:
          dx = static_cast<int8_t>(-kZigzagStepPx);
          break;
        default:
          dy = static_cast<int8_t>(-kZigzagStepPx);
          break;
      }
      for (int attempt = 0; attempt < 5; ++attempt) {
        if (hidSendRelative(hid, dx, dy)) {
          break;
        }
        delay(1);
      }
      delay(kReportIntervalMs);
    }
  }
}
