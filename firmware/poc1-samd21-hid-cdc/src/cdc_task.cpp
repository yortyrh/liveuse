/**
 * Task A — cdcRecvTask
 *
 * Role: USB CDC (Serial) control plane.
 *   1. Read line-oriented commands from the host serial monitor / future host service.
 *   2. Echo unrecognized / ECHO lines (POC1 composite sanity check).
 *   3. Parse MOVE / DEMO / HELP / STATUS into MouseCmd and push onto g_cmdQueue.
 *
 * Does NOT touch HID reports — that is hidMotionTask's job (separation of concerns).
 */
#include "tasks.h"

#include "cmd_queue.h"

#include <Seeed_Arduino_FreeRTOS.h>
#include <ctype.h>
#include <stdlib.h>
#include <string.h>

static constexpr size_t kLineBufSize = 96;

static void trimInPlace(char *s) {
  // Left trim
  char *start = s;
  while (*start && isspace(static_cast<unsigned char>(*start))) {
    ++start;
  }
  if (start != s) {
    memmove(s, start, strlen(start) + 1);
  }
  // Right trim
  size_t n = strlen(s);
  while (n > 0 && isspace(static_cast<unsigned char>(s[n - 1]))) {
    s[--n] = '\0';
  }
}

static void printHelp() {
  Serial.println(F("Commands (line-oriented, LF/CRLF):"));
  Serial.println(F("  HELP"));
  Serial.println(F("  STATUS"));
  Serial.println(F("  ECHO <text>          — echo text back on CDC"));
  Serial.println(F("  MOVE <dx> <dy>       — smooth relative HID move"));
  Serial.println(F("  DEMO circle|zigzag|both"));
  Serial.println(F("Anything else is echoed (composite CDC check)."));
}

static bool enqueueCmd(const MouseCmd &cmd) {
  auto *q = static_cast<QueueHandle_t>(g_cmdQueue);
  if (q == nullptr) {
    return false;
  }
  // Non-blocking: if the HID task is busy, drop with a note (host can retry).
  if (xQueueSend(q, &cmd, 0) != pdPASS) {
    Serial.println(F("ERR queue full"));
    return false;
  }
  return true;
}

static void handleLine(char *line) {
  trimInPlace(line);
  if (line[0] == '\0') {
    return;
  }

  // Case-insensitive command match on the first token.
  char cmd[16] = {0};
  const char *rest = line;
  size_t i = 0;
  while (*rest && !isspace(static_cast<unsigned char>(*rest)) && i + 1 < sizeof(cmd)) {
    cmd[i++] = static_cast<char>(toupper(static_cast<unsigned char>(*rest)));
    ++rest;
  }
  cmd[i] = '\0';
  while (*rest && isspace(static_cast<unsigned char>(*rest))) {
    ++rest;
  }

  if (strcmp(cmd, "HELP") == 0 || strcmp(cmd, "?") == 0) {
    printHelp();
    return;
  }

  if (strcmp(cmd, "STATUS") == 0) {
    Serial.print(F("OK FreeRTOS heap free="));
    Serial.print(xPortGetFreeHeapSize());
    Serial.print(F(" min="));
    Serial.println(xPortGetMinimumEverFreeHeapSize());
    Serial.print(F("OK queue waiting="));
    Serial.println(uxQueueMessagesWaiting(static_cast<QueueHandle_t>(g_cmdQueue)));
    return;
  }

  if (strcmp(cmd, "ECHO") == 0) {
    Serial.print(F("ECHO "));
    Serial.println(rest);
    return;
  }

  if (strcmp(cmd, "MOVE") == 0) {
    char *end = nullptr;
    long dx = strtol(rest, &end, 10);
    if (end == rest) {
      Serial.println(F("ERR MOVE needs dx dy"));
      return;
    }
    while (*end && isspace(static_cast<unsigned char>(*end))) {
      ++end;
    }
    char *end2 = nullptr;
    long dy = strtol(end, &end2, 10);
    if (end2 == end) {
      Serial.println(F("ERR MOVE needs dx dy"));
      return;
    }
    if (dx > kMaxMoveDelta) dx = kMaxMoveDelta;
    if (dx < -kMaxMoveDelta) dx = -kMaxMoveDelta;
    if (dy > kMaxMoveDelta) dy = kMaxMoveDelta;
    if (dy < -kMaxMoveDelta) dy = -kMaxMoveDelta;

    MouseCmd m{};
    m.type = CmdType::Move;
    m.dx = static_cast<int16_t>(dx);
    m.dy = static_cast<int16_t>(dy);
    if (enqueueCmd(m)) {
      Serial.print(F("OK queued MOVE "));
      Serial.print(m.dx);
      Serial.print(' ');
      Serial.println(m.dy);
    }
    return;
  }

  if (strcmp(cmd, "DEMO") == 0) {
    char arg[16] = {0};
    size_t j = 0;
    while (*rest && !isspace(static_cast<unsigned char>(*rest)) && j + 1 < sizeof(arg)) {
      arg[j++] = static_cast<char>(tolower(static_cast<unsigned char>(*rest)));
      ++rest;
    }
    MouseCmd m{};
    if (strcmp(arg, "circle") == 0) {
      m.type = CmdType::DemoCircle;
    } else if (strcmp(arg, "zigzag") == 0) {
      m.type = CmdType::DemoZigzag;
    } else if (strcmp(arg, "both") == 0 || arg[0] == '\0') {
      m.type = CmdType::DemoBoth;
    } else {
      Serial.println(F("ERR DEMO circle|zigzag|both"));
      return;
    }
    if (enqueueCmd(m)) {
      Serial.print(F("OK queued DEMO "));
      Serial.println(arg[0] ? arg : "both");
    }
    return;
  }

  // Default: echo the whole line (POC1 CDC proof).
  Serial.print(F("ECHO "));
  Serial.println(line);
}

void cdcRecvTask(void *pvParameters) {
  (void)pvParameters;

  Serial.println(F("[cdcRecvTask] started — CDC control plane"));
  printHelp();

  char line[kLineBufSize];
  size_t len = 0;

  for (;;) {
    while (Serial.available() > 0) {
      const int c = Serial.read();
      if (c < 0) {
        break;
      }
      if (c == '\r') {
        continue;
      }
      if (c == '\n') {
        line[len] = '\0';
        handleLine(line);
        len = 0;
        continue;
      }
      if (len + 1 < kLineBufSize) {
        line[len++] = static_cast<char>(c);
      } else {
        // Overflow: discard until newline.
        len = 0;
        Serial.println(F("ERR line too long"));
      }
    }
    // Yield so hidMotionTask / TinyUSB task can run.
    delay(5);
  }
}
