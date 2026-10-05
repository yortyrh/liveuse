#include "keymap.h"

#include <cstdio>
#include <cstring>

static int g_fails = 0;

static void check(bool cond, const char *what) {
  if (!cond) {
    std::fprintf(stderr, "FAIL %s\n", what);
    ++g_fails;
  }
}

int main() {
  const Chord letter = tap_ascii('a');
  check(letter.error == KeyError::Ok && letter.tap.modifier == 0 && letter.tap.usage == 0x04, "a");
  const Chord upper = tap_ascii('A');
  check(upper.error == KeyError::Ok && upper.tap.modifier == 0x02 && upper.tap.usage == 0x04, "A");
  const Chord digit = tap_ascii('2');
  check(digit.error == KeyError::Ok && digit.tap.modifier == 0 && digit.tap.usage == 0x1F, "2");
  const Chord at = tap_ascii('@');
  check(at.error == KeyError::Ok && at.tap.modifier == 0x02 && at.tap.usage == 0x1F, "@");
  check(tap_ascii('\n').error == KeyError::BadChar, "newline");

  const Chord space = parse_chord("cmd+space");
  check(space.error == KeyError::Ok && std::strcmp(space.canonical, "cmd+space") == 0, "cmd+space");
  check(space.tap.modifier == 0x08 && space.tap.usage == 0x2C, "cmd+space bits");

  const Chord shot = parse_chord("shift+cmd+4");
  check(shot.error == KeyError::Ok && std::strcmp(shot.canonical, "cmd+shift+4") == 0, "canonical order");
  check(shot.tap.modifier == (0x08 | 0x02) && shot.tap.usage == 0x21, "cmd+shift+4 bits");

  const Chord aye = parse_chord("A");
  check(aye.error == KeyError::Ok && std::strcmp(aye.canonical, "a") == 0 && aye.tap.modifier == 0, "KEY A");

  check(parse_chord("").error == KeyError::Empty, "empty");
  check(parse_chord("cmd").error == KeyError::BadChord, "modifier only");
  check(parse_chord("a+b").error == KeyError::BadChord, "two keys");
  check(parse_chord("cmd+cmd+a").error == KeyError::BadChord, "repeat modifier");
  check(parse_chord("foo").error == KeyError::BadChord, "unknown");
  return g_fails == 0 ? 0 : 1;
}
