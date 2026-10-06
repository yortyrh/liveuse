#include <iostream>

#include "pointer.hpp"

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
  const PointerGrid retina{1792, 1120, 3584, 2240};
  auto point = moves_for_objects(retina, 10, 10, 0, 0, {ObjectSpot{400, 200, 0, 0, false}});
  check(point.has_value() && point->size() == 1 && (*point)[0].x == 200 && (*point)[0].y == 100 &&
            (*point)[0].dx == 190 && (*point)[0].dy == 90,
        "retina_point");

  auto box = moves_for_objects(retina, 0, 0, 100, 50, {ObjectSpot{0, 0, 20, 8, true}});
  check(box.has_value() && box->size() == 1 && (*box)[0].x == 105 && (*box)[0].y == 52 && (*box)[0].dx == 105 &&
            (*box)[0].dy == 52,
        "box_center_with_origin");

  auto path = moves_for_objects(retina, 0, 0, 0, 0,
                                {ObjectSpot{2, 2, 0, 0, false}, ObjectSpot{6, 0, 0, 0, false}});
  check(path.has_value() && path->size() == 2 && (*path)[0].x == 1 && (*path)[0].y == 1 && (*path)[0].dx == 1 &&
            (*path)[0].dy == 1 && (*path)[1].x == 3 && (*path)[1].y == 0 && (*path)[1].dx == 2 && (*path)[1].dy == -1,
        "sequential_deltas");

  const PointerGrid same{100, 50, 100, 50};
  auto one = moves_for_objects(same, 1, 2, 0, 0, {ObjectSpot{10, 20, 0, 0, false}});
  check(one.has_value() && (*one)[0].x == 10 && (*one)[0].y == 20 && (*one)[0].dx == 9 && (*one)[0].dy == 18,
        "scale_one");

  check(!moves_for_objects(retina, 0, 0, 0, 0, {}).has_value(), "empty");
  check(!moves_for_objects(PointerGrid{0, 10, 10, 10}, 0, 0, 0, 0, {ObjectSpot{1, 1, 0, 0, false}}).has_value(),
        "no_display");
  check(!moves_for_objects(retina, 0, 0, 0, 0, {ObjectSpot{0, 0, 0, 4, true}}).has_value(), "bad_box");
  return failures == 0 ? 0 : 1;
}
