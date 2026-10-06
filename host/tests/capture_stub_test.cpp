#include <iostream>

#include "capture.hpp"

int main() {
  auto capture = make_screen_capture();
  auto size = capture->size();
  if (size || size.error() != kCaptureUnimplemented) {
    std::cerr << "FAIL stub\n";
    return 1;
  }
  return 0;
}
