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
  char canonical[64];
};

Chord tap_ascii(unsigned char c);
Chord parse_chord(const char *text);
const char *key_error_text(KeyError err);
