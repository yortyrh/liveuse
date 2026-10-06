#include "capture.hpp"

#include <CoreGraphics/CoreGraphics.h>
#include <dlfcn.h>

namespace {

class MacCapture final : public ScreenCapture {
 public:
  std::expected<std::pair<int, int>, std::string> size() override {
    const CGDirectDisplayID display = CGMainDisplayID();
    const int width = static_cast<int>(CGDisplayPixelsWide(display));
    const int height = static_cast<int>(CGDisplayPixelsHigh(display));
    if (width <= 0 || height <= 0) return std::unexpected(std::string("no display"));
    return std::pair<int, int>{width, height};
  }

  std::expected<std::pair<int, int>, std::string> pixels() override {
    CGDisplayModeRef mode = CGDisplayCopyDisplayMode(CGMainDisplayID());
    if (mode == nullptr) return std::unexpected(std::string("no display"));
    const int width = static_cast<int>(CGDisplayModeGetPixelWidth(mode));
    const int height = static_cast<int>(CGDisplayModeGetPixelHeight(mode));
    CGDisplayModeRelease(mode);
    if (width <= 0 || height <= 0) return std::unexpected(std::string("no display"));
    return std::pair<int, int>{width, height};
  }

  std::expected<std::pair<int, int>, std::string> cursor() override {
    CGEventRef event = CGEventCreate(nullptr);
    if (event == nullptr) return std::unexpected(std::string("cursor unavailable"));
    const CGPoint point = CGEventGetLocation(event);
    CFRelease(event);
    return std::pair<int, int>{static_cast<int>(point.x), static_cast<int>(point.y)};
  }

  std::expected<PixelBuffer, std::string> capture(const Region *region) override {
    auto dimensions = size();
    if (!dimensions) return std::unexpected(dimensions.error());
    CGRect rect = CGRectMake(0, 0, dimensions->first, dimensions->second);
    if (region != nullptr) {
      if (region->left < 0 || region->top < 0 || region->width <= 0 || region->height <= 0 ||
          region->left + region->width > dimensions->first || region->top + region->height > dimensions->second) {
        return std::unexpected(std::string("region out of range"));
      }
      rect = CGRectMake(region->left, region->top, region->width, region->height);
    }
    using CreateImage = CGImageRef (*)(CGDirectDisplayID, CGRect);
    auto *create_image = reinterpret_cast<CreateImage>(dlsym(RTLD_DEFAULT, "CGDisplayCreateImageForRect"));
    if (create_image == nullptr) return std::unexpected(std::string("screen capture is not permitted"));
    CGImageRef image = create_image(CGMainDisplayID(), rect);
    if (image == nullptr) return std::unexpected(std::string("screen capture is not permitted"));
    const size_t width = CGImageGetWidth(image);
    const size_t height = CGImageGetHeight(image);
    PixelBuffer buffer;
    buffer.width = static_cast<int>(width);
    buffer.height = static_cast<int>(height);
    buffer.bgra.assign(width * height * 4, 0);
    CGColorSpaceRef space = CGColorSpaceCreateDeviceRGB();
    const CGBitmapInfo info = static_cast<CGBitmapInfo>(static_cast<uint32_t>(kCGImageAlphaPremultipliedFirst) |
                                                         static_cast<uint32_t>(kCGBitmapByteOrder32Little));
    CGContextRef context = CGBitmapContextCreate(buffer.bgra.data(), width, height, 8, width * 4, space, info);
    CGColorSpaceRelease(space);
    if (context == nullptr) {
      CGImageRelease(image);
      return std::unexpected(std::string("screen capture failed"));
    }
    // CGDisplayCreateImageForRect is already top-down in this bitmap. A Y flip inverts the PNG.
    CGContextDrawImage(context, CGRectMake(0, 0, static_cast<CGFloat>(width), static_cast<CGFloat>(height)), image);
    CGContextRelease(context);
    CGImageRelease(image);
    return buffer;
  }
};

}  // namespace

std::unique_ptr<ScreenCapture> make_screen_capture() { return std::make_unique<MacCapture>(); }
