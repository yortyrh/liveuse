/**
 * Task B — hidMotionTask
 *
 * Role: USB HID mouse execution plane.
 *   1. Block on g_cmdQueue for the next MouseCmd.
 *   2. Run smooth relative motion (MOVE) or built-in DEMO patterns.
 *   3. Pulse the user LED while moving (visual activity without host tools).
 *
 * Does NOT parse CDC — that is cdcRecvTask's job.
 */
#include "tasks.h"

#include "cmd_queue.h"
#include "motion.h"

#include <Seeed_Arduino_FreeRTOS.h>

#ifndef LED_BUILTIN
#define LED_BUILTIN 13
#endif

static void ledPulse() {
  digitalWrite(LED_BUILTIN, LOW);  // XIAO LED is typically active-low
  delay(2);
  digitalWrite(LED_BUILTIN, HIGH);
}

void hidMotionTask(void *pvParameters) {
  (void)pvParameters;
  auto *q = static_cast<QueueHandle_t>(g_cmdQueue);

  Serial.println(F("[hidMotionTask] started — HID execution plane"));

  // After USB settles, auto-queue a demo so plugging in shows motion without a host app.
  delay(2000);
  {
    MouseCmd boot{};
    boot.type = CmdType::DemoBoth;
    if (xQueueSend(q, &boot, 0) == pdPASS) {
      Serial.println(F("[hidMotionTask] boot DEMO both queued"));
    }
  }

  for (;;) {
    MouseCmd cmd{};
    if (xQueueReceive(q, &cmd, portMAX_DELAY) != pdPASS) {
      continue;
    }

    ledPulse();

    switch (cmd.type) {
      case CmdType::Move:
        Serial.print(F("[hidMotionTask] MOVE "));
        Serial.print(cmd.dx);
        Serial.print(' ');
        Serial.println(cmd.dy);
        motionSmoothMove(g_usbHid, cmd.dx, cmd.dy);
        Serial.println(F("[hidMotionTask] MOVE done"));
        break;

      case CmdType::DemoCircle:
        Serial.println(F("[hidMotionTask] DEMO circle"));
        motionDemoCircle(g_usbHid);
        Serial.println(F("[hidMotionTask] DEMO circle done"));
        break;

      case CmdType::DemoZigzag:
        Serial.println(F("[hidMotionTask] DEMO zigzag"));
        motionDemoZigzag(g_usbHid);
        Serial.println(F("[hidMotionTask] DEMO zigzag done"));
        break;

      case CmdType::DemoBoth:
        Serial.println(F("[hidMotionTask] DEMO both (circle then zigzag)"));
        motionDemoCircle(g_usbHid);
        delay(600);
        motionDemoZigzag(g_usbHid);
        Serial.println(F("[hidMotionTask] DEMO both done"));
        break;

      default:
        break;
    }
  }
}
