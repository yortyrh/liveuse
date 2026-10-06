#include <iostream>

#include "locate.hpp"
#include "png.hpp"

namespace {
int failures = 0;
void check(bool ok, const char *name) {
  if (!ok) {
    std::cerr << "FAIL " << name << "\n";
    ++failures;
  }
}
}  // namespace

int main() {
  PixelBuffer hay;
  hay.width = 6;
  hay.height = 4;
  hay.bgra.assign(static_cast<size_t>(hay.width * hay.height * 4), 255);
  PixelBuffer needle;
  needle.width = 2;
  needle.height = 2;
  needle.bgra.assign(16, 0);
  for (int y = 0; y < 2; ++y) {
    for (int x = 0; x < 2; ++x) {
      uint8_t *px = hay.bgra.data() + (static_cast<size_t>((1 + y) * hay.width + (2 + x)) * 4);
      px[0] = 0;
      px[1] = 0;
      px[2] = 255;
      px[3] = 255;
      uint8_t *np = needle.bgra.data() + (static_cast<size_t>(y * 2 + x) * 4);
      np[0] = 0;
      np[1] = 0;
      np[2] = 255;
      np[3] = 255;
    }
  }
  auto found = locate_boxes(hay, needle, false, 1, nullptr);
  check(found.has_value() && found->size() == 1 && (*found)[0].left == 2 && (*found)[0].top == 1, "found");
  needle.bgra[2] = 250;
  auto missing = locate_boxes(hay, needle, false, 1, nullptr);
  check(!missing && missing.error() == "image not found", "missing");
  auto fuzzy = locate_boxes(hay, needle, false, 0.9, nullptr);
  check(fuzzy.has_value(), "tolerance");
  auto png = encode_png(hay);
  check(png.has_value(), "encode");
  if (png) {
    auto back = decode_png(png->data(), png->size());
    check(back.has_value() && back->width == hay.width && back->height == hay.height, "decode_size");
  }
  return failures == 0 ? 0 : 1;
}
