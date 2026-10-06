#include "locate.hpp"

#include <algorithm>
#include <cmath>
#include <cstdlib>

namespace {

int tolerance_for(double confidence) {
  if (confidence >= 1.0) return 0;
  return static_cast<int>(std::lround(255.0 * (1.0 - confidence)));
}

int gray_of(const uint8_t *px) {
  return (px[2] * 299 + px[1] * 587 + px[0] * 114) / 1000;
}

bool same_pixel(const uint8_t *a, const uint8_t *b, int tolerance, bool grayscale) {
  if (grayscale) return std::abs(gray_of(a) - gray_of(b)) <= tolerance;
  return std::abs(a[0] - b[0]) <= tolerance && std::abs(a[1] - b[1]) <= tolerance &&
         std::abs(a[2] - b[2]) <= tolerance;
}

bool match_at(const PixelBuffer &haystack, const PixelBuffer &needle, int x, int y, int tolerance, bool grayscale) {
  for (int ny = 0; ny < needle.height; ++ny) {
    for (int nx = 0; nx < needle.width; ++nx) {
      const uint8_t *a = haystack.bgra.data() + (static_cast<size_t>(y + ny) * haystack.width + (x + nx)) * 4;
      const uint8_t *b = needle.bgra.data() + (static_cast<size_t>(ny) * needle.width + nx) * 4;
      if (!same_pixel(a, b, tolerance, grayscale)) return false;
    }
  }
  return true;
}

}  // namespace

std::expected<std::vector<Box>, std::string> locate_boxes(const PixelBuffer &haystack, const PixelBuffer &needle,
                                                          bool grayscale, double confidence, const Region *region) {
  if (confidence < 0.0 || confidence > 1.0) return std::unexpected(std::string("confidence out of range"));
  if (needle.width <= 0 || needle.height <= 0) return std::unexpected(std::string("empty needle"));
  int left = 0;
  int top = 0;
  int right = haystack.width;
  int bottom = haystack.height;
  if (region != nullptr) {
    if (region->left < 0 || region->top < 0 || region->width <= 0 || region->height <= 0 ||
        region->left + region->width > haystack.width || region->top + region->height > haystack.height) {
      return std::unexpected(std::string("region out of range"));
    }
    left = region->left;
    top = region->top;
    right = region->left + region->width;
    bottom = region->top + region->height;
  }
  std::vector<Box> found;
  if (needle.width > right - left || needle.height > bottom - top) {
    return std::unexpected(std::string("image not found"));
  }
  const int tolerance = tolerance_for(confidence);
  for (int y = top; y <= bottom - needle.height; ++y) {
    for (int x = left; x <= right - needle.width; ++x) {
      if (!match_at(haystack, needle, x, y, tolerance, grayscale)) continue;
      found.push_back(Box{x, y, needle.width, needle.height});
      if (found.size() >= 64) return found;
    }
  }
  if (found.empty()) return std::unexpected(std::string("image not found"));
  return found;
}
