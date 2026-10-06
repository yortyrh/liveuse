#pragma once

#include <expected>
#include <functional>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "capture.hpp"
#include "link.hpp"
#include "locate.hpp"

struct ControlError {
  int status = 400;
  std::string message;
};

struct Rgb {
  int red = 0;
  int green = 0;
  int blue = 0;
};

struct PngImage {
  std::vector<uint8_t> bytes;
  int width = 0;
  int height = 0;
};

class Control {
 public:
  using Sleeper = std::function<void(double)>;

  Control(CdcLink &link, ScreenCapture &screen, Sleeper sleep);

  std::expected<std::pair<int, int>, ControlError> mouse_size();
  std::expected<PointerGrid, ControlError> pointer_grid();
  std::expected<std::pair<int, int>, ControlError> mouse_position();
  std::expected<bool, ControlError> mouse_on_screen(int x, int y);
  std::expected<std::vector<std::string>, ControlError> mouse_move_to(int x, int y, double duration,
                                                                      std::string_view tween);
  std::expected<std::vector<std::string>, ControlError> mouse_move(int dx, int dy, double duration,
                                                                   std::string_view tween);
  std::expected<std::vector<std::string>, ControlError> mouse_drag_to(int x, int y, double duration,
                                                                      std::string_view tween, std::string_view button);
  std::expected<std::vector<std::string>, ControlError> mouse_drag(int dx, int dy, double duration,
                                                                   std::string_view tween, std::string_view button);
  std::expected<std::vector<std::string>, ControlError> mouse_click(std::optional<int> x, std::optional<int> y,
                                                                    std::string_view button, int clicks,
                                                                    double interval);
  std::expected<std::vector<std::string>, ControlError> mouse_down(std::optional<int> x, std::optional<int> y,
                                                                   std::string_view button);
  std::expected<std::vector<std::string>, ControlError> mouse_up(std::optional<int> x, std::optional<int> y,
                                                                 std::string_view button);
  std::expected<std::vector<std::string>, ControlError> mouse_scroll(int clicks, std::optional<int> x,
                                                                     std::optional<int> y);
  std::expected<std::vector<std::string>, ControlError> mouse_hscroll(int clicks, std::optional<int> x,
                                                                      std::optional<int> y);
  std::expected<std::pair<std::vector<std::string>, int>, ControlError> keyboard_write(std::string_view text,
                                                                                       double interval);
  std::expected<std::vector<std::string>, ControlError> keyboard_press(std::string_view key, int presses,
                                                                       double interval);
  std::expected<std::vector<std::string>, ControlError> keyboard_down(std::string_view key);
  std::expected<std::vector<std::string>, ControlError> keyboard_up(std::string_view key);
  std::expected<std::vector<std::string>, ControlError> keyboard_hotkey(const std::vector<std::string> &keys);
  std::expected<std::vector<std::string>, ControlError> echo(std::string_view text);
  std::expected<std::vector<std::string>, ControlError> help();
  std::expected<std::vector<std::string>, ControlError> send_line(std::string_view line);
  std::expected<PngImage, ControlError> screenshot(const Region *region);
  std::expected<Rgb, ControlError> pixel(int x, int y);
  std::expected<bool, ControlError> pixel_matches(int x, int y, int red, int green, int blue, int tolerance);
  std::expected<std::vector<Box>, ControlError> locate(const PixelBuffer *haystack, const PixelBuffer &needle,
                                                       bool grayscale, double confidence, const Region *region);

 private:
  std::expected<std::vector<std::string>, ControlError> send(const std::vector<std::string> &lines, bool wait_done);
  std::expected<std::vector<std::string>, ControlError> move_delta(int dx, int dy, double duration,
                                                                   std::string_view tween);
  std::expected<std::vector<std::string>, ControlError> move_to_point(int x, int y, double duration,
                                                                      std::string_view tween);
  std::expected<PixelBuffer, ControlError> grab(const Region *region);

  CdcLink &link_;
  ScreenCapture &screen_;
  Sleeper sleep_;
};
