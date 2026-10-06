#include "keyboard.h"

#include <Arduino.h>
#include <cstring>

#include "keymap.h"

static uint8_t g_mod = 0;
static uint8_t g_keys[6] = {};

static bool tap(Adafruit_USBD_HID &hid, uint8_t modifier, uint8_t usage) {
  uint8_t down_keys[6];
  memcpy(down_keys, g_keys, sizeof(down_keys));
  bool placed = false;
  for (uint8_t &key : down_keys) {
    if (key == usage) {
      placed = true;
      break;
    }
  }
  if (!placed) {
    for (uint8_t &key : down_keys) {
      if (key == 0) {
        key = usage;
        placed = true;
        break;
      }
    }
  }
  if (!placed || !tud_hid_n_ready(1)) {
    return false;
  }
  const auto down_mod = static_cast<uint8_t>(g_mod | modifier);
  if (!tud_hid_n_keyboard_report(1, 0, down_mod, down_keys)) {
    return false;
  }
  delay(12);
  if (!tud_hid_n_keyboard_report(1, 0, g_mod, g_keys)) {
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

static bool send_held(Adafruit_USBD_HID &) {
  if (!tud_hid_n_ready(1)) {
    return false;
  }
  return tud_hid_n_keyboard_report(1, 0, g_mod, g_keys);
}

static bool add_usage(uint8_t usage) {
  if (usage == 0) {
    return true;
  }
  for (const auto key : g_keys) {
    if (key == usage) {
      return true;
    }
  }
  for (auto &key : g_keys) {
    if (key == 0) {
      key = usage;
      return true;
    }
  }
  return false;
}

static void remove_usage(uint8_t usage) {
  if (usage == 0) {
    return;
  }
  for (auto key : g_keys) {
    if (key == usage) {
      key = 0;
    }
  }
}

static bool apply_step(const HotkeyStep &step, const bool down) {
  if (down) {
    g_mod = static_cast<uint8_t>(g_mod | step.modifier);
    return add_usage(step.usage);
  }
  g_mod = static_cast<uint8_t>(g_mod & static_cast<uint8_t>(~step.modifier));
  remove_usage(step.usage);
  return true;
}

static bool change_keys(Adafruit_USBD_HID &hid, const char *payload, const bool down, const char *verb) {
  const Hotkey parsed = parse_hotkey(payload);
  if (parsed.error != KeyError::Ok || parsed.count == 0) {
    Serial.print(F("ERR "));
    Serial.print(verb);
    Serial.println(F(" bad"));
    return false;
  }
  const uint8_t saved_mod = g_mod;
  uint8_t saved_keys[6];
  memcpy(saved_keys, g_keys, sizeof(g_keys));
  for (int i = 0; i < parsed.count; ++i) {
    if (!apply_step(parsed.steps[i], down)) {
      g_mod = saved_mod;
      memcpy(g_keys, saved_keys, sizeof(g_keys));
      Serial.print(F("ERR "));
      Serial.print(verb);
      Serial.println(F(" bad"));
      return false;
    }
  }
  if (!send_held(hid)) {
    g_mod = saved_mod;
    memcpy(g_keys, saved_keys, sizeof(g_keys));
    Serial.print(F("ERR "));
    Serial.print(verb);
    Serial.println(F(" bad"));
    return false;
  }
  Serial.print(verb);
  Serial.print(' ');
  Serial.print(parsed.canonical);
  Serial.println(F(" done"));
  return true;
}

void keyboardDown(Adafruit_USBD_HID &hid, const char *payload) {
  change_keys(hid, payload, true, "KDOWN");
}

void keyboardUp(Adafruit_USBD_HID &hid, const char *payload) {
  change_keys(hid, payload, false, "KUP");
}

void keyboardHotkey(Adafruit_USBD_HID &hid, const char *payload) {
  const Hotkey parsed = parse_hotkey(payload);
  if (parsed.error != KeyError::Ok || parsed.count == 0) {
    Serial.println(F("ERR HOTKEY bad"));
    return;
  }
  uint8_t mod = g_mod;
  uint8_t keys[6];
  memcpy(keys, g_keys, sizeof(keys));

  auto send_local = [&](uint8_t use_mod, uint8_t *use_keys) -> bool {
    if (!tud_hid_n_ready(1)) {
      return false;
    }
    return tud_hid_n_keyboard_report(1, 0, use_mod, use_keys);
  };
  auto add_local = [&](uint8_t usage) -> bool {
    if (usage == 0) {
      return true;
    }
    for (uint8_t key : keys) {
      if (key == usage) {
        return true;
      }
    }
    for (uint8_t &key : keys) {
      if (key == 0) {
        key = usage;
        return true;
      }
    }
    return false;
  };

  for (int i = 0; i < parsed.count; ++i) {
    mod = static_cast<uint8_t>(mod | parsed.steps[i].modifier);
    if (!add_local(parsed.steps[i].usage) || !send_local(mod, keys)) {
      send_held(hid);
      Serial.println(F("ERR HOTKEY bad"));
      return;
    }
    delay(12);
  }
  for (int i = parsed.count - 1; i >= 0; --i) {
    mod = static_cast<uint8_t>(mod & static_cast<uint8_t>(~parsed.steps[i].modifier));
    const uint8_t usage = parsed.steps[i].usage;
    if (usage != 0) {
      for (uint8_t &key : keys) {
        if (key == usage) {
          key = 0;
        }
      }
    }
    if (!send_local(mod, keys)) {
      send_held(hid);
      Serial.println(F("ERR HOTKEY bad"));
      return;
    }
    delay(12);
  }
  send_held(hid);
  Serial.print(F("HOTKEY "));
  Serial.print(parsed.canonical);
  Serial.println(F(" done"));
}
