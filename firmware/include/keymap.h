#pragma once

#include <stdint.h>

struct KeyTap {
  uint8_t modifier;
  uint8_t usage;
};

enum class KeyError : int {
  Ok = 0,
  Empty,
  BadChar,
  BadChord,
};

struct Chord {
  KeyError error;
  KeyTap tap;
  char canonical[96];
};

struct HotkeyStep {
  uint8_t modifier;
  uint8_t usage;
  char canonical[32];
};

struct Hotkey {
  KeyError error;
  int count;
  HotkeyStep steps[8];
  char canonical[96];
};

Chord tap_ascii(unsigned char c);
Chord parse_chord(const char *text);
Hotkey parse_hotkey(const char *text);
bool key_has_no_hid_usage(const char *name);
const char *key_error_text(KeyError err);
