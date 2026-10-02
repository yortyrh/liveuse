#pragma once

#include <Arduino.h>
#include <Adafruit_TinyUSB.h>

/** FreeRTOS task: CDC serial RX → parse → enqueue MouseCmd. */
void cdcRecvTask(void *pvParameters);

/** FreeRTOS task: dequeue MouseCmd → smooth HID mouse motion. */
void hidMotionTask(void *pvParameters);

/** Shared TinyUSB HID mouse instance (composite with CDC when USE_TINYUSB). */
extern Adafruit_USBD_HID g_usbHid;
