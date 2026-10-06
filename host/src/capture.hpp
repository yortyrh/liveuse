#pragma once

#include <cstdint>
#include <expected>
#include <memory>
#include <string>
#include <utility>
#include <vector>

struct PixelBuffer {
  int width = 0;
  int height = 0;
  std::vector<uint8_t> bgra;
};

struct Region {
  int left = 0;
  int top = 0;
  int width = 0;
  int height = 0;
};

struct PointerGrid {
  int width = 0;
  int height = 0;
  int pixel_width = 0;
  int pixel_height = 0;
};

inline constexpr const char *kCaptureUnimplemented = "screen capture is not implemented on this OS";

class ScreenCapture {
 public:
  virtual ~ScreenCapture() = default;
  virtual std::expected<std::pair<int, int>, std::string> size() = 0;
  virtual std::expected<std::pair<int, int>, std::string> pixels() { return size(); }
  virtual std::expected<std::pair<int, int>, std::string> cursor() = 0;
  virtual std::expected<PixelBuffer, std::string> capture(const Region *region) = 0;
};

std::unique_ptr<ScreenCapture> make_screen_capture();
