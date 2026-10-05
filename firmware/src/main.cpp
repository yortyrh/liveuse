/**
 * mcp-mouse POC1 — Seeeduino XIAO / XIAO SAMD21
 *
 * USB composite on the same Type-C:
 *   - HID mouse (relative reports, smooth motion)
 *   - HID keyboard (TYPE / KEY commands)
 *   - CDC serial (command / echo control plane)
 *
 * Single Arduino loop: read a CDC line, run the motion, repeat.
 * TinyUSB is pumped by the core yield() inside delay() and Serial.
 *
 * Host service / MCP are later POCs — use any serial monitor for POC1.
 */

#include <Arduino.h>
#include <Adafruit_TinyUSB.h>

#include "keyboard.h"
#include "motion.h"

#include <ctype.h>
#include <stdlib.h>
#include <string.h>

static constexpr int16_t kMaxMoveDelta = 2000;
static constexpr size_t kLineBufSize = 96;

static uint8_t const desc_hid_report[] = {TUD_HID_REPORT_DESC_MOUSE()};
static uint8_t const desc_keyboard[] = {TUD_HID_REPORT_DESC_KEYBOARD()};

static Adafruit_USBD_HID g_usbHid;
static Adafruit_USBD_HID g_usbKeyboard;

#ifndef LED_BUILTIN
#define LED_BUILTIN 13
#endif

static void ledPulse() {
  digitalWrite(LED_BUILTIN, LOW);  // XIAO LED is typically active-low
  delay(2);
  digitalWrite(LED_BUILTIN, HIGH);
}

static void trimLeading(char *s) {
  char *start = s;
  while (*start && isspace(static_cast<unsigned char>(*start))) {
    ++start;
  }
  if (start != s) {
    memmove(s, start, strlen(start) + 1);
  }
}

static void trimTrailing(char *s) {
  size_t n = strlen(s);
  while (n > 0 && isspace(static_cast<unsigned char>(s[n - 1]))) {
    s[--n] = '\0';
  }
}

static char s_line[kLineBufSize];
static size_t s_lineLen = 0;

static void printHelp() {
  Serial.println(F("Commands (line-oriented, LF/CRLF):"));
  Serial.println(F("  HELP"));
  Serial.println(F("  STATUS"));
  Serial.println(F("  ECHO <text>          — echo text back on CDC"));
  Serial.println(F("  MOVE <dx> <dy>       — smooth relative HID move"));
  Serial.println(F("  CLICK left|right|double"));
  Serial.println(F("  SCROLL <steps>       — wheel notches, +up -down"));
  Serial.println(F("  DEMO circle|zigzag|both"));
  Serial.println(F("  TYPE <ascii>"));
  Serial.println(F("  KEY <chord>"));
  Serial.println(F("Anything else is echoed (composite CDC check)."));
}

static void printBanner() {
  Serial.println();
  Serial.println(F("=== mcp-mouse POC1 — XIAO SAMD21 ==="));
  Serial.println(F("USB: composite HID mouse + keyboard + CDC serial"));
  Serial.println(F("Motion: smooth relative (circle/zigzag demo + MOVE)"));
  Serial.println(F("===================================="));
  printHelp();
  Serial.flush();
}

static void runDemoBoth() {
  motionDemoCircle(g_usbHid);
  delay(600);
  motionDemoZigzag(g_usbHid);
}

static void handleLine(char *line) {
  // Leading whitespace only: TYPE payload keeps its trailing spaces.
  trimLeading(line);
  if (line[0] == '\0') {
    return;
  }

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

  // Payload is everything after the first run of spaces; do not trim it.
  if (strcmp(cmd, "TYPE") == 0) {
    keyboardType(g_usbKeyboard, rest);
    return;
  }

  // rest points into line; trimming the tail never moves it.
  trimTrailing(line);

  if (strcmp(cmd, "HELP") == 0 || strcmp(cmd, "?") == 0) {
    printHelp();
    return;
  }

  if (strcmp(cmd, "STATUS") == 0) {
    Serial.print(F("OK uptime_ms="));
    Serial.print(millis());
    Serial.println(F(" fw=0.3.0"));
    return;
  }

  if (strcmp(cmd, "ECHO") == 0) {
    Serial.print(F("ECHO "));
    Serial.println(rest);
    return;
  }

  if (strcmp(cmd, "KEY") == 0) {
    keyboardChord(g_usbKeyboard, rest);
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

    Serial.print(F("MOVE "));
    Serial.print(dx);
    Serial.print(' ');
    Serial.println(dy);
    ledPulse();
    motionSmoothMove(g_usbHid, static_cast<int16_t>(dx), static_cast<int16_t>(dy));
    Serial.println(F("MOVE done"));
    return;
  }

  if (strcmp(cmd, "CLICK") == 0) {
    char arg[16] = {0};
    size_t j = 0;
    while (*rest && !isspace(static_cast<unsigned char>(*rest)) && j + 1 < sizeof(arg)) {
      arg[j++] = static_cast<char>(tolower(static_cast<unsigned char>(*rest)));
      ++rest;
    }

    // Bits match hid_mouse_button_bm_t: left = bit 0, right = bit 1.
    uint8_t buttons = 0;
    uint8_t times = 1;
    if (strcmp(arg, "left") == 0) {
      buttons = 0x01;
    } else if (strcmp(arg, "right") == 0) {
      buttons = 0x02;
    } else if (strcmp(arg, "double") == 0) {
      buttons = 0x01;
      times = 2;
    } else {
      Serial.println(F("ERR CLICK left|right|double"));
      return;
    }

    Serial.print(F("CLICK "));
    Serial.println(arg);
    ledPulse();
    motionClick(g_usbHid, buttons, times);
    Serial.print(F("CLICK "));
    Serial.print(arg);
    Serial.println(F(" done"));
    return;
  }

  if (strcmp(cmd, "SCROLL") == 0) {
    char *end = nullptr;
    long steps = strtol(rest, &end, 10);
    if (end == rest) {
      Serial.println(F("ERR SCROLL needs steps"));
      return;
    }
    if (steps > kMaxMoveDelta) steps = kMaxMoveDelta;
    if (steps < -kMaxMoveDelta) steps = -kMaxMoveDelta;

    Serial.print(F("SCROLL "));
    Serial.println(steps);
    ledPulse();
    motionScroll(g_usbHid, static_cast<int16_t>(steps));
    Serial.println(F("SCROLL done"));
    return;
  }

  if (strcmp(cmd, "DEMO") == 0) {
    char arg[16] = {0};
    size_t j = 0;
    while (*rest && !isspace(static_cast<unsigned char>(*rest)) && j + 1 < sizeof(arg)) {
      arg[j++] = static_cast<char>(tolower(static_cast<unsigned char>(*rest)));
      ++rest;
    }

    ledPulse();
    if (strcmp(arg, "circle") == 0) {
      Serial.println(F("DEMO circle"));
      motionDemoCircle(g_usbHid);
      Serial.println(F("DEMO circle done"));
    } else if (strcmp(arg, "zigzag") == 0) {
      Serial.println(F("DEMO zigzag"));
      motionDemoZigzag(g_usbHid);
      Serial.println(F("DEMO zigzag done"));
    } else if (strcmp(arg, "both") == 0 || arg[0] == '\0') {
      Serial.println(F("DEMO both"));
      runDemoBoth();
      Serial.println(F("DEMO both done"));
    } else {
      Serial.println(F("ERR DEMO circle|zigzag|both"));
    }
    return;
  }

  Serial.print(F("ECHO "));
  Serial.println(line);
}

static void pollSerial() {
  while (Serial.available() > 0) {
    const int c = Serial.read();
    if (c < 0) {
      break;
    }
    if (c == '\r') {
      continue;
    }
    if (c == '\n') {
      s_line[s_lineLen] = '\0';
      handleLine(s_line);
      s_lineLen = 0;
      continue;
    }
    if (s_lineLen + 1 < kLineBufSize) {
      s_line[s_lineLen++] = static_cast<char>(c);
    } else {
      s_lineLen = 0;
      Serial.println(F("ERR line too long"));
    }
  }
}

void setup() {
  pinMode(LED_BUILTIN, OUTPUT);
  digitalWrite(LED_BUILTIN, HIGH);

  g_usbHid.setPollInterval(2);
  g_usbHid.setReportDescriptor(desc_hid_report, sizeof(desc_hid_report));
#if defined(HID_ITF_PROTOCOL_MOUSE)
  g_usbHid.setReportID(0);
#endif
  g_usbHid.begin();

  g_usbKeyboard.setPollInterval(2);
  g_usbKeyboard.setReportDescriptor(desc_keyboard, sizeof(desc_keyboard));
  g_usbKeyboard.begin();

  Serial.begin(115200);

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
}

void loop() {
  static bool wasOpen = false;

  // Host sets DTR when it opens the CDC port. Print on every rising edge.
  const bool open = Serial.dtr() != 0;
  if (open && !wasOpen) {
    delay(20);
    s_lineLen = 0;
    printBanner();
  }
  wasOpen = open;

  pollSerial();
}
