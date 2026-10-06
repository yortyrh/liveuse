#pragma once

#include <expected>
#include <string>
#include <string_view>
#include <vector>

struct CdcError {
  int status;
  std::string message;
};

std::expected<std::vector<std::string>, CdcError> format_type(std::string_view text);
std::expected<std::string, CdcError> format_key(std::string_view chord);
std::expected<std::string, CdcError> format_move(int dx, int dy);
std::expected<std::string, CdcError> format_click(std::string_view button);
std::expected<std::string, CdcError> format_scroll(int steps);
std::expected<std::string, CdcError> format_echo(std::string_view text);
std::expected<std::string, CdcError> format_raw(std::string_view line);
std::expected<std::string, CdcError> format_mouse_button(std::string_view verb, std::string_view button);
std::expected<std::string, CdcError> format_hscroll(int steps);
std::expected<std::string, CdcError> format_key_hold(std::string_view verb, std::string_view key);
std::expected<std::string, CdcError> format_hotkey(const std::vector<std::string> &keys);
std::expected<std::string, CdcError> format_press_key(std::string_view key);
