#include "pointer.hpp"

#include <cmath>

std::expected<std::vector<PointerMove>, std::string> moves_for_objects(const PointerGrid &grid, int cursor_x,
                                                                       int cursor_y, double origin_x, double origin_y,
                                                                       const std::vector<ObjectSpot> &spots) {
  if (spots.empty()) return std::unexpected(std::string("objects is required"));
  if (grid.width <= 0 || grid.height <= 0 || grid.pixel_width <= 0 || grid.pixel_height <= 0) {
    return std::unexpected(std::string("no display"));
  }
  const double scale_x = static_cast<double>(grid.pixel_width) / static_cast<double>(grid.width);
  const double scale_y = static_cast<double>(grid.pixel_height) / static_cast<double>(grid.height);
  std::vector<PointerMove> moves;
  int previous_x = cursor_x;
  int previous_y = cursor_y;
  for (const ObjectSpot &spot : spots) {
    if (spot.box && (spot.width <= 0 || spot.height <= 0)) {
      return std::unexpected(std::string("object size out of range"));
    }
    const double aim_x = spot.box ? spot.x + spot.width / 2.0 : spot.x;
    const double aim_y = spot.box ? spot.y + spot.height / 2.0 : spot.y;
    PointerMove move;
    move.x = static_cast<int>(std::lround(origin_x + aim_x / scale_x));
    move.y = static_cast<int>(std::lround(origin_y + aim_y / scale_y));
    move.dx = move.x - previous_x;
    move.dy = move.y - previous_y;
    previous_x = move.x;
    previous_y = move.y;
    moves.push_back(move);
  }
  return moves;
}
