#include "keyboard.h"

#include <Arduino.h>
#include <string.h>

#include "keymap.h"

static bool tap(Adafruit_USBD_HID &hid, uint8_t modifier, uint8_t usage) {
  uint8_t keys[6] = {usage, 0, 0, 0, 0, 0};
  if (!hid.ready()) {
    return false;
  }
  if (!hid.keyboardReport(0, modifier, keys)) {
    return false;
  }
  delay(12);
  uint8_t up[6] = {0, 0, 0, 0, 0, 0};
  if (!hid.keyboardReport(0, 0, up)) {
    return false;
  }
  delay(12);
  return true;
}

void keyboardType(Adafruit_USBD_HID &hid, const char *payload) {
  if (payload == nullptr || payload[0] == '\0') {
    Serial.println(F("ERR TYPE bad"));
    return;
  }

  const size_t len = strlen(payload);

  // Validate every byte before sending any report.
  for (size_t i = 0; i < len; ++i) {
    const Chord c = tap_ascii(static_cast<unsigned char>(payload[i]));
    if (c.error != KeyError::Ok) {
      Serial.println(F("ERR TYPE bad"));
      return;
    }
  }

  for (size_t i = 0; i < len; ++i) {
    const Chord c = tap_ascii(static_cast<unsigned char>(payload[i]));
    if (!tap(hid, c.tap.modifier, c.tap.usage)) {
      Serial.println(F("ERR TYPE bad"));
      return;
    }
  }
}

void keyboardChord(Adafruit_USBD_HID &hid, const char *chord) {
  const Chord c = parse_chord(chord);
  if (c.error != KeyError::Ok) {
    Serial.println(F("ERR KEY bad"));
    return;
  }
  if (!tap(hid, c.tap.modifier, c.tap.usage)) {
    Serial.println(F("ERR KEY bad"));
    return;
  }
  Serial.print(F("KEY "));
  Serial.print(c.canonical);
  Serial.println(F(" done"));
}
