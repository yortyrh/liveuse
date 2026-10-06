#include "capture.hpp"

namespace {

class StubCapture final : public ScreenCapture {
 public:
  std::expected<std::pair<int, int>, std::string> size() override {
    return std::unexpected(std::string(kCaptureUnimplemented));
  }
  std::expected<std::pair<int, int>, std::string> cursor() override {
    return std::unexpected(std::string(kCaptureUnimplemented));
  }
  std::expected<PixelBuffer, std::string> capture(const Region *) override {
    return std::unexpected(std::string(kCaptureUnimplemented));
  }
};

}  // namespace

std::unique_ptr<ScreenCapture> make_screen_capture() { return std::make_unique<StubCapture>(); }
