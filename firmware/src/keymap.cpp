#include "keymap.h"

#include <cctype>
#include <cstdio>
#include <cstring>

namespace {

struct Named {
  const char *name;
  uint8_t usage;
};

const Named kNamed[] = {
    {"enter", 0x28},       {"return", 0x28}, {"esc", 0x29},
    {"escape", 0x29},      {"backspace", 0x2A}, {"tab", 0x2B},
    {"space", 0x2C},       {"minus", 0x2D},  {"equal", 0x2E},
    {"leftbracket", 0x2F}, {"rightbracket", 0x30}, {"backslash", 0x31},
    {"semicolon", 0x33},   {"quote", 0x34},  {"grave", 0x35},
    {"comma", 0x36},       {"dot", 0x37},    {"slash", 0x38},
    {"delete", 0x4C},      {"del", 0x4C},    {"right", 0x4F},
    {"left", 0x50},        {"down", 0x51},   {"up", 0x52},
    {"insert", 0x49},      {"home", 0x4A},   {"pageup", 0x4B},
    {"pgup", 0x4B},        {"end", 0x4D},    {"pagedown", 0x4E},
    {"pgdn", 0x4E},
};

bool named_usage(const char *token, uint8_t *usage) {
  if (token[0] == 'f' && token[1] >= '1' && token[1] <= '9' && token[2] == '\0') {
    *usage = static_cast<uint8_t>(0x3A + (token[1] - '1'));
    return true;
  }
  if (std::strcmp(token, "f10") == 0) { *usage = 0x43; return true; }
  if (std::strcmp(token, "f11") == 0) { *usage = 0x44; return true; }
  if (std::strcmp(token, "f12") == 0) { *usage = 0x45; return true; }
  for (const Named &row : kNamed) {
    if (std::strcmp(token, row.name) == 0) {
      *usage = row.usage;
      return true;
    }
  }
  return false;
}

Chord fail(KeyError err) {
  Chord out{};
  out.error = err;
  return out;
}

}  // namespace

const char *key_error_text(KeyError err) {
  switch (err) {
    case KeyError::Ok: return "ok";
    case KeyError::Empty: return "empty";
    case KeyError::BadChar: return "bad char";
    case KeyError::BadChord: return "bad chord";
  }
  return "bad chord";
}

Chord tap_ascii(unsigned char c) {
  if (c >= 'a' && c <= 'z') {
    Chord out{};
    out.tap = {0, static_cast<uint8_t>(0x04 + (c - 'a'))};
    return out;
  }
  if (c >= 'A' && c <= 'Z') {
    Chord out{};
    out.tap = {0x02, static_cast<uint8_t>(0x04 + (c - 'A'))};
    return out;
  }
  if (c >= '1' && c <= '9') {
    Chord out{};
    out.tap = {0, static_cast<uint8_t>(0x1E + (c - '1'))};
    return out;
  }
  Chord out{};
  switch (c) {
    case ' ': out.tap = {0, 0x2C}; break;
    case '!': out.tap = {0x02, 0x1E}; break;
    case '"': out.tap = {0x02, 0x34}; break;
    case '#': out.tap = {0x02, 0x20}; break;
    case '$': out.tap = {0x02, 0x21}; break;
    case '%': out.tap = {0x02, 0x22}; break;
    case '&': out.tap = {0x02, 0x24}; break;
    case '\'': out.tap = {0, 0x34}; break;
    case '(': out.tap = {0x02, 0x26}; break;
    case ')': out.tap = {0x02, 0x27}; break;
    case '*': out.tap = {0x02, 0x25}; break;
    case '+': out.tap = {0x02, 0x2E}; break;
    case ',': out.tap = {0, 0x36}; break;
    case '-': out.tap = {0, 0x2D}; break;
    case '.': out.tap = {0, 0x37}; break;
    case '/': out.tap = {0, 0x38}; break;
    case '0': out.tap = {0, 0x27}; break;
    case ':': out.tap = {0x02, 0x33}; break;
    case ';': out.tap = {0, 0x33}; break;
    case '<': out.tap = {0x02, 0x36}; break;
    case '=': out.tap = {0, 0x2E}; break;
    case '>': out.tap = {0x02, 0x37}; break;
    case '?': out.tap = {0x02, 0x38}; break;
    case '@': out.tap = {0x02, 0x1F}; break;
    case '[': out.tap = {0, 0x2F}; break;
    case '\\': out.tap = {0, 0x31}; break;
    case ']': out.tap = {0, 0x30}; break;
    case '^': out.tap = {0x02, 0x23}; break;
    case '_': out.tap = {0x02, 0x2D}; break;
    case '`': out.tap = {0, 0x35}; break;
    case '{': out.tap = {0x02, 0x2F}; break;
    case '|': out.tap = {0x02, 0x31}; break;
    case '}': out.tap = {0x02, 0x30}; break;
    case '~': out.tap = {0x02, 0x35}; break;
    default: return fail(KeyError::BadChar);
  }
  return out;
}

Chord parse_chord(const char *text) {
  if (text == nullptr || text[0] == '\0') {
    return fail(KeyError::Empty);
  }
  uint8_t modifier = 0;
  uint8_t seen_mod_bits = 0;
  uint8_t usage = 0;
  bool have_key = false;
  char key_name[32] = {};
  const char *cursor = text;
  while (*cursor) {
    char token[32] = {};
    size_t n = 0;
    while (*cursor && *cursor != '+') {
      if (n + 1 >= sizeof(token)) {
        return fail(KeyError::BadChord);
      }
      token[n++] = static_cast<char>(std::tolower(static_cast<unsigned char>(*cursor)));
      ++cursor;
    }
    token[n] = '\0';
    if (*cursor == '+') {
      ++cursor;
    }
    if (n == 0) {
      return fail(KeyError::BadChord);
    }
    uint8_t bit = 0;
    const char *canon = nullptr;
    if (std::strcmp(token, "cmd") == 0 || std::strcmp(token, "command") == 0 ||
        std::strcmp(token, "meta") == 0 || std::strcmp(token, "win") == 0 ||
        std::strcmp(token, "winleft") == 0 || std::strcmp(token, "commandleft") == 0) {
      bit = 0x08;
      canon = "cmd";
    } else if (std::strcmp(token, "winright") == 0 || std::strcmp(token, "cmdrigh") == 0 ||
               std::strcmp(token, "commandright") == 0) {
      bit = 0x80;
      canon = "winright";
    } else if (std::strcmp(token, "ctrl") == 0 || std::strcmp(token, "control") == 0 ||
               std::strcmp(token, "ctrlleft") == 0) {
      bit = 0x01;
      canon = "ctrl";
    } else if (std::strcmp(token, "ctrlright") == 0) {
      bit = 0x10;
      canon = "ctrlright";
    } else if (std::strcmp(token, "alt") == 0 || std::strcmp(token, "opt") == 0 ||
               std::strcmp(token, "option") == 0 || std::strcmp(token, "altleft") == 0 ||
               std::strcmp(token, "optionleft") == 0) {
      bit = 0x04;
      canon = "alt";
    } else if (std::strcmp(token, "altright") == 0 || std::strcmp(token, "optionright") == 0) {
      bit = 0x40;
      canon = "altright";
    } else if (std::strcmp(token, "shift") == 0 || std::strcmp(token, "shiftleft") == 0) {
      bit = 0x02;
      canon = "shift";
    } else if (std::strcmp(token, "shiftright") == 0) {
      bit = 0x20;
      canon = "shiftright";
    }
    if (bit != 0) {
      if ((seen_mod_bits & bit) != 0) {
        return fail(KeyError::BadChord);
      }
      seen_mod_bits = static_cast<uint8_t>(seen_mod_bits | bit);
      modifier = static_cast<uint8_t>(modifier | bit);
      (void)canon;
      continue;
    }
    if (have_key) {
      return fail(KeyError::BadChord);
    }
    uint8_t named = 0;
    if (n == 1 && token[0] >= 'a' && token[0] <= 'z') {
      usage = static_cast<uint8_t>(0x04 + (token[0] - 'a'));
    } else if (n == 1 && token[0] >= '1' && token[0] <= '9') {
      usage = static_cast<uint8_t>(0x1E + (token[0] - '1'));
    } else if (n == 1 && token[0] == '0') {
      usage = 0x27;
    } else if (!named_usage(token, &named)) {
      return fail(KeyError::BadChord);
    } else {
      usage = named;
    }
    std::memcpy(key_name, token, n + 1);
    if (std::strcmp(token, "return") == 0) {
      std::strcpy(key_name, "enter");
    } else if (std::strcmp(token, "escape") == 0) {
      std::strcpy(key_name, "esc");
    } else if (std::strcmp(token, "del") == 0) {
      std::strcpy(key_name, "delete");
    } else if (std::strcmp(token, "pgup") == 0) {
      std::strcpy(key_name, "pageup");
    } else if (std::strcmp(token, "pgdn") == 0) {
      std::strcpy(key_name, "pagedown");
    }
    have_key = true;
  }
  if (!have_key) {
    return fail(KeyError::BadChord);
  }
  Chord out{};
  out.tap = {modifier, usage};
  char *dst = out.canonical;
  const char *parts[] = {"cmd", "winright", "ctrl", "ctrlright", "alt", "altright", "shift", "shiftright"};
  const uint8_t bits[] = {0x08, 0x80, 0x01, 0x10, 0x04, 0x40, 0x02, 0x20};
  for (int i = 0; i < 8; ++i) {
    if ((modifier & bits[i]) == 0) {
      continue;
    }
    if (dst != out.canonical) {
      *dst++ = '+';
    }
    std::strcpy(dst, parts[i]);
    dst += std::strlen(parts[i]);
  }
  if (dst != out.canonical) {
    *dst++ = '+';
  }
  std::strcpy(dst, key_name);
  return out;
}

namespace {

bool modifier_token(const char *token, uint8_t *bit, const char **canon) {
  if (std::strcmp(token, "cmd") == 0 || std::strcmp(token, "command") == 0 ||
      std::strcmp(token, "meta") == 0 || std::strcmp(token, "win") == 0 ||
      std::strcmp(token, "winleft") == 0 || std::strcmp(token, "commandleft") == 0) {
    *bit = 0x08;
    *canon = "cmd";
    return true;
  }
  if (std::strcmp(token, "winright") == 0 || std::strcmp(token, "cmdrigh") == 0 ||
      std::strcmp(token, "commandright") == 0) {
    *bit = 0x80;
    *canon = "winright";
    return true;
  }
  if (std::strcmp(token, "ctrl") == 0 || std::strcmp(token, "control") == 0 ||
      std::strcmp(token, "ctrlleft") == 0) {
    *bit = 0x01;
    *canon = "ctrl";
    return true;
  }
  if (std::strcmp(token, "ctrlright") == 0) {
    *bit = 0x10;
    *canon = "ctrlright";
    return true;
  }
  if (std::strcmp(token, "alt") == 0 || std::strcmp(token, "opt") == 0 ||
      std::strcmp(token, "option") == 0 || std::strcmp(token, "altleft") == 0 ||
      std::strcmp(token, "optionleft") == 0) {
    *bit = 0x04;
    *canon = "alt";
    return true;
  }
  if (std::strcmp(token, "altright") == 0 || std::strcmp(token, "optionright") == 0) {
    *bit = 0x40;
    *canon = "altright";
    return true;
  }
  if (std::strcmp(token, "shift") == 0 || std::strcmp(token, "shiftleft") == 0) {
    *bit = 0x02;
    *canon = "shift";
    return true;
  }
  if (std::strcmp(token, "shiftright") == 0) {
    *bit = 0x20;
    *canon = "shiftright";
    return true;
  }
  return false;
}

void canon_alias(const char *token, char *key_name) {
  if (std::strcmp(token, "return") == 0) {
    std::strcpy(key_name, "enter");
  } else if (std::strcmp(token, "escape") == 0) {
    std::strcpy(key_name, "esc");
  } else if (std::strcmp(token, "del") == 0) {
    std::strcpy(key_name, "delete");
  } else if (std::strcmp(token, "pgup") == 0) {
    std::strcpy(key_name, "pageup");
  } else if (std::strcmp(token, "pgdn") == 0) {
    std::strcpy(key_name, "pagedown");
  }
}

bool fill_step(const char *token, HotkeyStep *step) {
  std::memset(step, 0, sizeof(*step));
  uint8_t bit = 0;
  const char *canon = nullptr;
  if (modifier_token(token, &bit, &canon)) {
    step->modifier = bit;
    std::strcpy(step->canonical, canon);
    return true;
  }
  if (token[0] != '\0' && token[1] == '\0') {
    const Chord tap = tap_ascii(static_cast<unsigned char>(token[0]));
    if (tap.error != KeyError::Ok) {
      return false;
    }
    step->modifier = tap.tap.modifier;
    step->usage = tap.tap.usage;
    if (tap.tap.modifier != 0) {
      unsigned char plain = 0;
      for (unsigned char u = 0x20; u <= 0x7E; ++u) {
        const Chord other = tap_ascii(u);
        if (other.error == KeyError::Ok && other.tap.modifier == 0 && other.tap.usage == tap.tap.usage) {
          plain = u;
          break;
        }
      }
      if (plain == 0) {
        return false;
      }
      std::snprintf(step->canonical, sizeof(step->canonical), "shift+%c", plain);
    } else {
      step->canonical[0] = static_cast<char>(std::tolower(static_cast<unsigned char>(token[0])));
      step->canonical[1] = '\0';
    }
    return true;
  }
  uint8_t named = 0;
  if (token[0] >= 'a' && token[0] <= 'z' && token[1] == '\0') {
    step->usage = static_cast<uint8_t>(0x04 + (token[0] - 'a'));
    step->canonical[0] = token[0];
    step->canonical[1] = '\0';
    return true;
  }
  if (!named_usage(token, &named)) {
    return false;
  }
  step->usage = named;
  std::strncpy(step->canonical, token, sizeof(step->canonical) - 1);
  canon_alias(token, step->canonical);
  return true;
}

}  // namespace

bool key_has_no_hid_usage(const char *name) {
  if (name == nullptr) {
    return false;
  }
  char token[32] = {};
  size_t n = 0;
  for (const char *p = name; *p; ++p) {
    if (n + 1 >= sizeof(token)) {
      return false;
    }
    token[n++] = static_cast<char>(std::tolower(static_cast<unsigned char>(*p)));
  }
  static const char *kNone[] = {
      "accept", "add", "apps", "browserback", "browserfavorites", "browserforward",
      "browserhome", "browserrefresh", "browsersearch", "browserstop", "capslock",
      "clear", "convert", "decimal", "divide", "execute", "final", "fn", "hanguel",
      "hangul", "hanja", "help", "junja", "kana", "kanji", "launchapp1", "launchapp2",
      "launchmail", "launchmediaselect", "modechange", "multiply", "nexttrack",
      "nonconvert", "num0", "num1", "num2", "num3", "num4", "num5", "num6", "num7",
      "num8", "num9", "numlock", "pause", "playpause", "prevtrack", "print",
      "printscreen", "prntscrn", "prtsc", "prtscr", "scrolllock", "select",
      "separator", "sleep", "stop", "subtract", "volumedown", "volumemute",
      "volumeup", "yen",
  };
  for (const char *row : kNone) {
    if (std::strcmp(token, row) == 0) {
      return true;
    }
  }
  return false;
}

Hotkey parse_hotkey(const char *text) {
  Hotkey out{};
  if (text == nullptr || text[0] == '\0') {
    out.error = KeyError::Empty;
    return out;
  }
  const char *cursor = text;
  char joined[96] = {};
  size_t joined_len = 0;
  while (*cursor) {
    char token[32] = {};
    size_t n = 0;
    while (*cursor && *cursor != '+') {
      if (n + 1 >= sizeof(token)) {
        out.error = KeyError::BadChord;
        out.count = 0;
        return out;
      }
      token[n++] = static_cast<char>(*cursor);
      ++cursor;
    }
    token[n] = '\0';
    if (*cursor == '+') {
      ++cursor;
    }
    if (n == 0 || out.count >= 8) {
      out.error = KeyError::BadChord;
      out.count = 0;
      return out;
    }
    char lower[32] = {};
    for (size_t i = 0; i < n; ++i) {
      lower[i] = static_cast<char>(std::tolower(static_cast<unsigned char>(token[i])));
    }
    HotkeyStep step{};
    const char *parsed = (n == 1) ? token : lower;
    if (!fill_step(parsed, &step)) {
      out.error = KeyError::BadChord;
      out.count = 0;
      return out;
    }
    for (int i = 0; i < out.count; ++i) {
      if (step.usage != 0 && out.steps[i].usage == step.usage) {
        out.error = KeyError::BadChord;
        out.count = 0;
        return out;
      }
      if (step.modifier != 0 && (out.steps[i].modifier & step.modifier) != 0 && step.usage == 0 &&
          out.steps[i].usage == 0) {
        out.error = KeyError::BadChord;
        out.count = 0;
        return out;
      }
    }
    const size_t piece = std::strlen(step.canonical);
    if (joined_len + piece + (joined_len ? 1 : 0) >= sizeof(joined)) {
      out.error = KeyError::BadChord;
      out.count = 0;
      return out;
    }
    if (joined_len != 0) {
      joined[joined_len++] = '+';
    }
    std::memcpy(joined + joined_len, step.canonical, piece);
    joined_len += piece;
    out.steps[out.count++] = step;
  }
  std::memcpy(out.canonical, joined, joined_len + 1);
  return out;
}
