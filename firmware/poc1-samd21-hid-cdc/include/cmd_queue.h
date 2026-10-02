/**
 * Command queue contract between FreeRTOS tasks.
 *
 * Design:
 *   cdcRecvTask  --parse CDC lines-->  g_cmdQueue  -->  hidMotionTask
 *
 * Keep messages small (fits SAMD21 RAM budget; see docs/samd21-rtos-ram-budget.md).
 */
#pragma once

#include <stdint.h>

/** Maximum absolute |dx| / |dy| requested in one MOVE command before smooth split. */
static constexpr int16_t kMaxMoveDelta = 2000;

enum class CmdType : uint8_t {
  Nop = 0,
  /** Smooth relative cursor path totaling (dx, dy). */
  Move,
  /** Built-in smooth demo patterns. */
  DemoCircle,
  DemoZigzag,
  DemoBoth,
};

struct MouseCmd {
  CmdType type;
  int16_t dx;  // used by Move
  int16_t dy;  // used by Move
};

/** Depth of the FreeRTOS command queue (items, not bytes). */
static constexpr unsigned kCmdQueueLength = 16;

/**
 * Shared FreeRTOS queue handle — created once in setup() before the scheduler.
 * Declared as void* here so this header stays FreeRTOS-header free for includes;
 * cast to QueueHandle_t at use sites (or include Seeed_Arduino_FreeRTOS.h first).
 */
extern void *g_cmdQueue;
