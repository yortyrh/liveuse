#pragma once

#include <expected>
#include <string>
#include <vector>

#include "capture.hpp"

struct ObjectSpot {
  double x = 0;
  double y = 0;
  double width = 0;
  double height = 0;
  bool box = false;
};

struct PointerMove {
  int x = 0;
  int y = 0;
  int dx = 0;
  int dy = 0;
};

std::expected<std::vector<PointerMove>, std::string> moves_for_objects(const PointerGrid &grid, int cursor_x,
                                                                       int cursor_y, double origin_x, double origin_y,
                                                                       const std::vector<ObjectSpot> &spots);
