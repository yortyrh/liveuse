#include "cdc.hpp"

#include <algorithm>
#include <cctype>
#include <cstddef>

#include "keymap.h"

namespace {

constexpr std::size_t kMaxTypeChars = 2000;
constexpr std::size_t kChunkChars = 75;
constexpr std::size_t kMaxLineChars = 80;
constexpr int kMaxDelta = 2000;

std::unexpected<CdcError> bad(std::string message) {
  return std::unexpected(CdcError{400, std::move(message)});
}

std::string lower(std::string_view s) {
  std::string out(s);
  std::transform(out.begin(), out.end(), out.begin(),
                 [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
  return out;
}

std::expected<void, CdcError> check_line(std::string_view text, const char *what) {
  if (text.empty()) return bad(std::string(what) + " is empty");
  if (text.size() > kMaxLineChars) return bad(std::string(what) + " is too long");
  if (text.find_first_of("\r\n") != std::string_view::npos) {
    return bad(std::string(what) + " must be a single line");
  }
  return {};
}

}  // namespace

std::expected<std::vector<std::string>, CdcError> format_type(std::string_view text) {
  if (text.empty()) return bad("text is empty");
  if (text.size() > kMaxTypeChars) return bad("text is too long");
  for (char ch : text) {
    if (ch == '\r' || ch == '\n') return bad("text must not contain line breaks");
    const auto c = static_cast<unsigned char>(ch);
    if (c < 0x20 || c > 0x7E) return bad("text contains an unsupported character");
    Chord tap = tap_ascii(c);
    if (tap.error != KeyError::Ok) {
      return bad(std::string("text rejected: ") + key_error_text(tap.error));
    }
  }
  std::vector<std::string> lines;
  for (std::size_t pos = 0; pos < text.size(); pos += kChunkChars) {
    lines.push_back("TYPE " + std::string(text.substr(pos, kChunkChars)));
  }
  return lines;
}

std::expected<std::string, CdcError> format_key(std::string_view chord) {
  const std::string text(chord);
  Chord parsed = parse_chord(text.c_str());
  if (parsed.error != KeyError::Ok) {
    return bad(std::string("key rejected: ") + key_error_text(parsed.error));
  }
  return "KEY " + std::string(parsed.canonical);
}

std::expected<std::string, CdcError> format_move(int dx, int dy) {
  if (dx < -kMaxDelta || dx > kMaxDelta || dy < -kMaxDelta || dy > kMaxDelta) {
    return bad("move out of range");
  }
  return "MOVE " + std::to_string(dx) + " " + std::to_string(dy);
}

std::expected<std::string, CdcError> format_click(std::string_view button) {
  const std::string name = lower(button);
  if (name != "left" && name != "right" && name != "double") {
    return bad("unknown button");
  }
  return "CLICK " + name;
}

std::expected<std::string, CdcError> format_scroll(int steps) {
  if (steps < -kMaxDelta || steps > kMaxDelta) return bad("scroll out of range");
  return "SCROLL " + std::to_string(steps);
}

std::expected<std::string, CdcError> format_demo(std::string_view pattern) {
  const std::string name = lower(pattern);
  if (name != "circle" && name != "zigzag" && name != "both") {
    return bad("unknown demo pattern");
  }
  return "DEMO " + name;
}

std::expected<std::string, CdcError> format_echo(std::string_view text) {
  if (auto ok = check_line(text, "echo text"); !ok) return std::unexpected(ok.error());
  return "ECHO " + std::string(text);
}

std::expected<std::string, CdcError> format_raw(std::string_view line) {
  if (auto ok = check_line(line, "line"); !ok) return std::unexpected(ok.error());
  return std::string(line);
}
