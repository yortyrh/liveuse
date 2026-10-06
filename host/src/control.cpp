#include "control.hpp"

#include <algorithm>
#include <cmath>
#include <thread>
#include <chrono>

#include "cdc.hpp"
#include "locate.hpp"
#include "png.hpp"
#include "tween.hpp"

namespace {

std::unexpected<ControlError> fail(int status, std::string message) {
  return std::unexpected(ControlError{status, std::move(message)});
}

bool firmware_error(const std::vector<std::string> &lines, std::string *message) {
  for (const std::string &line : lines) {
    if (line.rfind("ERR", 0) == 0) {
      *message = line;
      return true;
    }
  }
  return false;
}

}  // namespace

Control::Control(CdcLink &link, ScreenCapture &screen, Sleeper sleep)
    : link_(link), screen_(screen), sleep_(std::move(sleep)) {
  if (!sleep_) {
    sleep_ = [](double seconds) {
      if (seconds > 0) std::this_thread::sleep_for(std::chrono::duration<double>(seconds));
    };
  }
}

std::expected<std::vector<std::string>, ControlError> Control::send(const std::vector<std::string> &lines,
                                                                    bool wait_done) {
  if (!link_.connected()) {
    auto port = only_device_port(link_.list_ports());
    if (!port) return fail(port.error().status, port.error().message);
    auto opened = link_.connect(*port);
    if (!opened) return fail(opened.error().status == 0 ? 502 : opened.error().status, opened.error().message);
  }
  auto got = link_.write_lines(lines, wait_done);
  if (!got) return fail(got.error().status == 0 ? 502 : got.error().status, got.error().message);
  std::string err;
  if (firmware_error(*got, &err)) return fail(502, err);
  return *got;
}

std::expected<std::pair<int, int>, ControlError> Control::mouse_size() {
  auto got = screen_.size();
  if (!got) return fail(503, got.error());
  return *got;
}

std::expected<PointerGrid, ControlError> Control::pointer_grid() {
  auto points = screen_.size();
  if (!points) return fail(503, points.error());
  auto pixels = screen_.pixels();
  if (!pixels) return fail(503, pixels.error());
  if (points->first <= 0 || points->second <= 0 || pixels->first <= 0 || pixels->second <= 0) {
    return fail(503, "no display");
  }
  return PointerGrid{points->first, points->second, pixels->first, pixels->second};
}

std::expected<std::pair<int, int>, ControlError> Control::mouse_position() {
  auto got = screen_.cursor();
  if (!got) return fail(503, got.error());
  return *got;
}

std::expected<bool, ControlError> Control::mouse_on_screen(int x, int y) {
  auto got = mouse_size();
  if (!got) return std::unexpected(got.error());
  return x >= 0 && y >= 0 && x < got->first && y < got->second;
}

std::expected<std::vector<std::string>, ControlError> Control::move_delta(int dx, int dy, double duration,
                                                                          std::string_view tween) {
  if (duration < 0) return fail(400, "duration out of range");
  if (dx == 0 && dy == 0) return std::vector<std::string>{};
  const int steps = duration <= 0 ? 1 : std::clamp(static_cast<int>(std::lround(duration / 0.02)), 1, 250);
  std::vector<std::string> lines;
  int sent_x = 0;
  int sent_y = 0;
  for (int i = 1; i <= steps; ++i) {
    auto eased = tween_at(tween, static_cast<double>(i) / static_cast<double>(steps));
    if (!eased) return fail(400, eased.error());
    const int target_x = static_cast<int>(std::lround(static_cast<double>(dx) * *eased));
    const int target_y = static_cast<int>(std::lround(static_cast<double>(dy) * *eased));
    int step_x = target_x - sent_x;
    int step_y = target_y - sent_y;
    sent_x = target_x;
    sent_y = target_y;
    while (step_x != 0 || step_y != 0) {
      const int piece_x = std::clamp(step_x, -2000, 2000);
      const int piece_y = std::clamp(step_y, -2000, 2000);
      auto formatted = format_move(piece_x, piece_y);
      if (!formatted) return fail(formatted.error().status, formatted.error().message);
      auto got = send({*formatted}, true);
      if (!got) return std::unexpected(got.error());
      lines.insert(lines.end(), got->begin(), got->end());
      step_x -= piece_x;
      step_y -= piece_y;
    }
    if (duration > 0) sleep_(duration / static_cast<double>(steps));
  }
  return lines;
}

std::expected<std::vector<std::string>, ControlError> Control::move_to_point(int x, int y, double duration,
                                                                             std::string_view tween) {
  auto pos = mouse_position();
  if (!pos) return std::unexpected(pos.error());
  return move_delta(x - pos->first, y - pos->second, duration, tween);
}

std::expected<std::vector<std::string>, ControlError> Control::mouse_move_to(int x, int y, double duration,
                                                                             std::string_view tween) {
  return move_to_point(x, y, duration, tween);
}

std::expected<std::vector<std::string>, ControlError> Control::mouse_move(int dx, int dy, double duration,
                                                                          std::string_view tween) {
  return move_delta(dx, dy, duration, tween);
}

std::expected<std::vector<std::string>, ControlError> Control::mouse_drag_to(int x, int y, double duration,
                                                                             std::string_view tween,
                                                                             std::string_view button) {
  auto down = format_mouse_button("MDOWN", button);
  if (!down) return fail(down.error().status, down.error().message);
  auto pressed = send({*down}, true);
  if (!pressed) return std::unexpected(pressed.error());
  auto moved = move_to_point(x, y, duration, tween);
  if (!moved) return std::unexpected(moved.error());
  auto up = format_mouse_button("MUP", button);
  auto released = send({*up}, true);
  if (!released) return std::unexpected(released.error());
  std::vector<std::string> lines = *pressed;
  lines.insert(lines.end(), moved->begin(), moved->end());
  lines.insert(lines.end(), released->begin(), released->end());
  return lines;
}

std::expected<std::vector<std::string>, ControlError> Control::mouse_drag(int dx, int dy, double duration,
                                                                          std::string_view tween,
                                                                          std::string_view button) {
  auto down = format_mouse_button("MDOWN", button);
  if (!down) return fail(down.error().status, down.error().message);
  auto pressed = send({*down}, true);
  if (!pressed) return std::unexpected(pressed.error());
  auto moved = move_delta(dx, dy, duration, tween);
  if (!moved) return std::unexpected(moved.error());
  auto up = format_mouse_button("MUP", button);
  auto released = send({*up}, true);
  if (!released) return std::unexpected(released.error());
  std::vector<std::string> lines = *pressed;
  lines.insert(lines.end(), moved->begin(), moved->end());
  lines.insert(lines.end(), released->begin(), released->end());
  return lines;
}

std::expected<std::vector<std::string>, ControlError> Control::mouse_click(std::optional<int> x, std::optional<int> y,
                                                                           std::string_view button, int clicks,
                                                                           double interval) {
  if (clicks < 1) return fail(400, "clicks out of range");
  if (interval < 0) return fail(400, "interval out of range");
  if (x.has_value() != y.has_value()) return fail(400, "x and y are a pair");
  std::vector<std::string> lines;
  if (x.has_value()) {
    auto moved = move_to_point(*x, *y, 0, "linear");
    if (!moved) return std::unexpected(moved.error());
    lines.insert(lines.end(), moved->begin(), moved->end());
  }
  auto down = format_mouse_button("MDOWN", button);
  auto up = format_mouse_button("MUP", button);
  if (!down) return fail(down.error().status, down.error().message);
  for (int i = 0; i < clicks; ++i) {
    if (i > 0) sleep_(interval);
    auto pressed = send({*down}, true);
    if (!pressed) return std::unexpected(pressed.error());
    auto released = send({*up}, true);
    if (!released) return std::unexpected(released.error());
    lines.insert(lines.end(), pressed->begin(), pressed->end());
    lines.insert(lines.end(), released->begin(), released->end());
  }
  return lines;
}

std::expected<std::vector<std::string>, ControlError> Control::mouse_down(std::optional<int> x, std::optional<int> y,
                                                                          std::string_view button) {
  if (x.has_value() != y.has_value()) return fail(400, "x and y are a pair");
  std::vector<std::string> lines;
  if (x.has_value()) {
    auto moved = move_to_point(*x, *y, 0, "linear");
    if (!moved) return std::unexpected(moved.error());
    lines = *moved;
  }
  auto down = format_mouse_button("MDOWN", button);
  if (!down) return fail(down.error().status, down.error().message);
  auto pressed = send({*down}, true);
  if (!pressed) return std::unexpected(pressed.error());
  lines.insert(lines.end(), pressed->begin(), pressed->end());
  return lines;
}

std::expected<std::vector<std::string>, ControlError> Control::mouse_up(std::optional<int> x, std::optional<int> y,
                                                                        std::string_view button) {
  if (x.has_value() != y.has_value()) return fail(400, "x and y are a pair");
  std::vector<std::string> lines;
  if (x.has_value()) {
    auto moved = move_to_point(*x, *y, 0, "linear");
    if (!moved) return std::unexpected(moved.error());
    lines = *moved;
  }
  auto up = format_mouse_button("MUP", button);
  if (!up) return fail(up.error().status, up.error().message);
  auto released = send({*up}, true);
  if (!released) return std::unexpected(released.error());
  lines.insert(lines.end(), released->begin(), released->end());
  return lines;
}

std::expected<std::vector<std::string>, ControlError> Control::mouse_scroll(int clicks, std::optional<int> x,
                                                                            std::optional<int> y) {
  if (x.has_value() != y.has_value()) return fail(400, "x and y are a pair");
  std::vector<std::string> lines;
  if (x.has_value()) {
    auto moved = move_to_point(*x, *y, 0, "linear");
    if (!moved) return std::unexpected(moved.error());
    lines = *moved;
  }
  int left = clicks;
  while (left != 0) {
    const int piece = std::clamp(left, -2000, 2000);
    auto formatted = format_scroll(piece);
    if (!formatted) return fail(formatted.error().status, formatted.error().message);
    auto got = send({*formatted}, true);
    if (!got) return std::unexpected(got.error());
    lines.insert(lines.end(), got->begin(), got->end());
    left -= piece;
  }
  return lines;
}

std::expected<std::vector<std::string>, ControlError> Control::mouse_hscroll(int clicks, std::optional<int> x,
                                                                             std::optional<int> y) {
  if (x.has_value() != y.has_value()) return fail(400, "x and y are a pair");
  std::vector<std::string> lines;
  if (x.has_value()) {
    auto moved = move_to_point(*x, *y, 0, "linear");
    if (!moved) return std::unexpected(moved.error());
    lines = *moved;
  }
  int left = clicks;
  while (left != 0) {
    const int piece = std::clamp(left, -2000, 2000);
    auto formatted = format_hscroll(piece);
    if (!formatted) return fail(formatted.error().status, formatted.error().message);
    auto got = send({*formatted}, true);
    if (!got) return std::unexpected(got.error());
    lines.insert(lines.end(), got->begin(), got->end());
    left -= piece;
  }
  return lines;
}

std::expected<std::pair<std::vector<std::string>, int>, ControlError> Control::keyboard_write(std::string_view text,
                                                                                              double interval) {
  if (interval < 0) return fail(400, "interval out of range");
  if (interval == 0) {
    auto lines = format_type(text);
    if (!lines) return fail(lines.error().status, lines.error().message);
    auto got = send(*lines, false);
    if (!got) return std::unexpected(got.error());
    return std::pair<std::vector<std::string>, int>{*got, static_cast<int>(lines->size())};
  }
  std::vector<std::string> all;
  int chunks = 0;
  for (size_t i = 0; i < text.size(); ++i) {
    if (i > 0) sleep_(interval);
    auto lines = format_type(text.substr(i, 1));
    if (!lines) return fail(lines.error().status, lines.error().message);
    auto got = send(*lines, false);
    if (!got) return std::unexpected(got.error());
    all.insert(all.end(), got->begin(), got->end());
    chunks += static_cast<int>(lines->size());
  }
  return std::pair<std::vector<std::string>, int>{all, chunks};
}

std::expected<std::vector<std::string>, ControlError> Control::keyboard_press(std::string_view key, int presses,
                                                                              double interval) {
  if (presses < 1) return fail(400, "presses out of range");
  if (interval < 0) return fail(400, "interval out of range");
  auto formatted = format_press_key(key);
  if (!formatted) return fail(formatted.error().status, formatted.error().message);
  std::vector<std::string> lines;
  for (int i = 0; i < presses; ++i) {
    if (i > 0) sleep_(interval);
    auto got = send({*formatted}, true);
    if (!got) return std::unexpected(got.error());
    lines.insert(lines.end(), got->begin(), got->end());
  }
  return lines;
}

std::expected<std::vector<std::string>, ControlError> Control::keyboard_down(std::string_view key) {
  auto formatted = format_key_hold("KDOWN", key);
  if (!formatted) return fail(formatted.error().status, formatted.error().message);
  return send({*formatted}, true);
}

std::expected<std::vector<std::string>, ControlError> Control::keyboard_up(std::string_view key) {
  auto formatted = format_key_hold("KUP", key);
  if (!formatted) return fail(formatted.error().status, formatted.error().message);
  return send({*formatted}, true);
}

std::expected<std::vector<std::string>, ControlError> Control::keyboard_hotkey(const std::vector<std::string> &keys) {
  auto formatted = format_hotkey(keys);
  if (!formatted) return fail(formatted.error().status, formatted.error().message);
  return send({*formatted}, true);
}

std::expected<std::vector<std::string>, ControlError> Control::echo(std::string_view text) {
  auto formatted = format_echo(text);
  if (!formatted) return fail(formatted.error().status, formatted.error().message);
  return send({*formatted}, true);
}

std::expected<std::vector<std::string>, ControlError> Control::help() {
  return send({"HELP"}, true);
}

std::expected<std::vector<std::string>, ControlError> Control::send_line(std::string_view line) {
  auto formatted = format_raw(line);
  if (!formatted) return fail(formatted.error().status, formatted.error().message);
  const bool typed = formatted->rfind("TYPE ", 0) == 0;
  return send({*formatted}, !typed);
}

std::expected<PixelBuffer, ControlError> Control::grab(const Region *region) {
  auto got = screen_.capture(region);
  if (!got) {
    const int status = got.error() == "region out of range" ? 400 : 503;
    return fail(status, got.error());
  }
  return *got;
}

std::expected<PngImage, ControlError> Control::screenshot(const Region *region) {
  auto buffer = grab(region);
  if (!buffer) return std::unexpected(buffer.error());
  auto png = encode_png(*buffer);
  if (!png) return fail(503, png.error());
  return PngImage{std::move(*png), buffer->width, buffer->height};
}

std::expected<Rgb, ControlError> Control::pixel(int x, int y) {
  auto buffer = grab(nullptr);
  if (!buffer) return std::unexpected(buffer.error());
  if (x < 0 || y < 0 || x >= buffer->width || y >= buffer->height) return fail(400, "pixel out of range");
  const uint8_t *px = buffer->bgra.data() + (static_cast<size_t>(y) * buffer->width + x) * 4;
  return Rgb{px[2], px[1], px[0]};
}

std::expected<bool, ControlError> Control::pixel_matches(int x, int y, int red, int green, int blue, int tolerance) {
  if (tolerance < 0 || red < 0 || red > 255 || green < 0 || green > 255 || blue < 0 || blue > 255) {
    return fail(400, "color out of range");
  }
  auto got = pixel(x, y);
  if (!got) return std::unexpected(got.error());
  return std::abs(got->red - red) <= tolerance && std::abs(got->green - green) <= tolerance &&
         std::abs(got->blue - blue) <= tolerance;
}

std::expected<std::vector<Box>, ControlError> Control::locate(const PixelBuffer *haystack, const PixelBuffer &needle,
                                                              bool grayscale, double confidence, const Region *region) {
  PixelBuffer owned;
  const PixelBuffer *source = haystack;
  if (source == nullptr) {
    auto buffer = grab(nullptr);
    if (!buffer) return std::unexpected(buffer.error());
    owned = std::move(*buffer);
    source = &owned;
  }
  auto found = locate_boxes(*source, needle, grayscale, confidence, region);
  if (!found) {
    const int status = found.error() == "image not found" ? 404 : 400;
    return fail(status, found.error());
  }
  return *found;
}
