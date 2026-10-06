#pragma once

#include <expected>
#include <string>
#include <vector>

#include "capture.hpp"

struct Box {
  int left = 0;
  int top = 0;
  int width = 0;
  int height = 0;
};

std::expected<std::vector<Box>, std::string> locate_boxes(const PixelBuffer &haystack, const PixelBuffer &needle,
                                                          bool grayscale, double confidence, const Region *region);
