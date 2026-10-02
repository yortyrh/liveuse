/**
 * mcp-mouse POC1 — Seeeduino XIAO / XIAO SAMD21
 *
 * USB composite on the same Type-C:
 *   - HID mouse (relative reports, smooth motion)
 *   - CDC serial (command / echo control plane)
 *
 * FreeRTOS architecture (portfolio-visible, not a fake RTOS loop):
 *
 *   ┌──────────────┐   MouseCmd    ┌────────────────┐
 *   │ cdcRecvTask  │──────────────►│  g_cmdQueue    │
 *   │  (Task A)    │   xQueueSend  │  depth 16      │
 *   └──────────────┘               └───────┬────────┘
 *                                          │ xQueueReceive
 *                                          ▼
 *                                  ┌────────────────┐
 *                                  │ hidMotionTask  │
 *                                  │  (Task B)      │
 *                                  └───────┬────────┘
 *                                          │ mouseMove(dx,dy)
 *                                          ▼
 *                                  USB HID + CDC (TinyUSB)
 *
 * Plus Seeed FreeRTOS helper `tinyusb_task()` which pumps TinyUSB Device_Task
 * and flushes CDC on its own high-priority background task.
 *
 * Host service / MCP are later POCs — use any serial monitor for POC1.
 */

#include <Arduino.h>
#include <Adafruit_TinyUSB.h>
#include <Seeed_Arduino_FreeRTOS.h>

#include "cmd_queue.h"
#include "tasks.h"

// Declared in Seeed FreeRTOS arch/samd21/FreeRTOSVariant.h (compiled into the lib
// when USE_TINYUSB is set). Creates a high-priority task that pumps TinyUSB.
#if defined(USE_TINYUSB)
extern void tinyusb_task();
#endif

// ---------------------------------------------------------------------------
// USB HID mouse (composite with CDC when USE_TINYUSB is set in platformio.ini)
// ---------------------------------------------------------------------------

// Single-report boot-compatible mouse descriptor (TinyUSB template).
static uint8_t const desc_hid_report[] = {TUD_HID_REPORT_DESC_MOUSE()};

Adafruit_USBD_HID g_usbHid;

// Shared command queue (created in setup before vTaskStartScheduler).
void *g_cmdQueue = nullptr;

static TaskHandle_t s_cdcTask = nullptr;
static TaskHandle_t s_hidTask = nullptr;

#ifndef LED_BUILTIN
#define LED_BUILTIN 13
#endif

void setup() {
  pinMode(LED_BUILTIN, OUTPUT);
  digitalWrite(LED_BUILTIN, HIGH);

  // HID must begin before Serial when building a composite device.
  g_usbHid.setPollInterval(2);
  g_usbHid.setReportDescriptor(desc_hid_report, sizeof(desc_hid_report));
#if defined(HID_ITF_PROTOCOL_MOUSE)
  g_usbHid.setReportID(0);
#endif
  g_usbHid.begin();

  Serial.begin(115200);

  // Wait for USB mount with timeout (do not hang forever on bare metal flash).
  const uint32_t deadline = millis() + 5000;
#if defined(USBDevice)
  while (!USBDevice.mounted() && millis() < deadline) {
    delay(10);
  }
#elif defined(TinyUSBDevice)
  while (!TinyUSBDevice.mounted() && millis() < deadline) {
    delay(10);
  }
#else
  delay(1500);
#endif

  delay(200);
  Serial.println();
  Serial.println(F("=== mcp-mouse POC1 — XIAO SAMD21 ==="));
  Serial.println(F("USB: composite HID mouse + CDC serial"));
  Serial.println(F("RTOS: FreeRTOS — cdcRecvTask | queue | hidMotionTask"));
  Serial.println(F("Motion: smooth relative (circle/zigzag demo + MOVE)"));
  Serial.println(F("===================================="));

  // Command queue: 16 × sizeof(MouseCmd) ≈ few hundred bytes — see RAM budget doc.
  g_cmdQueue = xQueueCreate(kCmdQueueLength, sizeof(MouseCmd));
  if (g_cmdQueue == nullptr) {
    Serial.println(F("FATAL: xQueueCreate failed"));
    for (;;) {
      digitalWrite(LED_BUILTIN, LOW);
      delay(100);
      digitalWrite(LED_BUILTIN, HIGH);
      delay(100);
    }
  }

  // Stack sizes in words (Cortex-M0+: 1 word = 4 bytes). Mid values from RAM budget.
  // Priority: TinyUSB (via tinyusb_task) highest; CDC slightly above HID so parsing
  // stays responsive while a long demo runs.
  const uint16_t kCdcStackWords = 320;
  const uint16_t kHidStackWords = 384;

  BaseType_t ok_a = xTaskCreate(cdcRecvTask, "cdcRecv", kCdcStackWords, nullptr,
                                tskIDLE_PRIORITY + 3, &s_cdcTask);
  BaseType_t ok_b = xTaskCreate(hidMotionTask, "hidMotion", kHidStackWords, nullptr,
                                tskIDLE_PRIORITY + 2, &s_hidTask);

  if (ok_a != pdPASS || ok_b != pdPASS) {
    Serial.println(F("FATAL: xTaskCreate failed"));
    for (;;) {
      delay(500);
    }
  }

#if defined(USE_TINYUSB)
  // Seeed FreeRTOS helper: dedicated task pumping TinyUSB + CDC flush.
  tinyusb_task();
#endif

  Serial.println(F("Starting FreeRTOS scheduler…"));
  Serial.flush();

  // Never returns — Arduino loop() becomes unused (or idle-adjacent).
  vTaskStartScheduler();

  // Should be unreachable.
  Serial.println(F("FATAL: scheduler returned"));
  for (;;) {
    delay(1000);
  }
}

void loop() {
  // With the scheduler running, loop() is not the application main path.
  // Keep empty — real work lives in named FreeRTOS tasks.
}
