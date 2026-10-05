#include <iostream>
#include <string>
#include <vector>

#include "cdc.hpp"
#include "keymap.h"

namespace {

int failures = 0;

void check(bool ok, const char *name) {
  if (!ok) {
    std::cerr << "FAIL " << name << "\n";
    ++failures;
  }
}

template <typename T>
bool is_error(const std::expected<T, CdcError> &r, int status) {
  return !r.has_value() && r.error().status == status;
}

}  // namespace

int main() {
  {
    auto r = format_type("hi");
    check(r.has_value() && *r == std::vector<std::string>{"TYPE hi"}, "type_hi");
  }
  check(is_error(format_type(""), 400), "type_empty");
  check(is_error(format_type("a\nb"), 400), "type_newline");
  {
    auto r = format_type(std::string(76, 'a'));
    check(r.has_value() && r->size() == 2 && (*r)[0].size() == 80 && (*r)[1].size() == 6,
          "type_chunks");
  }
  check(is_error(format_type(std::string(2001, 'a')), 400), "type_too_long");
  {
    auto r = format_type("@");
    check(r.has_value() && *r == std::vector<std::string>{"TYPE @"}, "type_at");
    Chord at = tap_ascii('@');
    check(at.tap.modifier == 0x02 && at.tap.usage == 0x1F, "tap_ascii_at");
  }

  {
    auto r = format_key("shift+cmd+4");
    check(r.has_value() && *r == "KEY cmd+shift+4", "key_canonical");
  }
  {
    auto r = format_key("A");
    check(r.has_value() && *r == "KEY a", "key_lower");
  }
  check(is_error(format_key("cmd"), 400), "key_modifier_only");

  {
    auto r = format_move(10, -4);
    check(r.has_value() && *r == "MOVE 10 -4", "move_ok");
  }
  check(is_error(format_move(2001, 0), 400), "move_range");
  {
    auto r = format_click("Right");
    check(r.has_value() && *r == "CLICK right", "click_right");
  }
  check(is_error(format_click("middle"), 400), "click_middle");
  {
    auto r = format_scroll(-3);
    check(r.has_value() && *r == "SCROLL -3", "scroll_ok");
  }
  {
    auto r = format_demo("Circle");
    check(r.has_value() && *r == "DEMO circle", "demo_circle");
  }
  check(is_error(format_echo("a\nb"), 400), "echo_newline");
  check(is_error(format_echo(std::string(76, 'a')), 400), "echo_too_long");
  {
    auto r = format_echo(std::string(75, 'a'));
    check(r.has_value() && r->size() == 80, "echo_max_line");
  }
  {
    auto r = format_raw("HELP");
    check(r.has_value() && *r == "HELP", "raw_help");
  }
  check(is_error(format_raw(std::string(81, 'x')), 400), "raw_too_long");

  return failures == 0 ? 0 : 1;
}
