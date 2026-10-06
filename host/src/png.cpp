#include "png.hpp"

#include <vector>

#define STB_IMAGE_IMPLEMENTATION
#define STBI_NO_STDIO
#include "stb_image.h"
#define STB_IMAGE_WRITE_IMPLEMENTATION
#define STBI_WRITE_NO_STDIO
#include "stb_image_write.h"

namespace {

constexpr char kB64[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

void png_write(void *context, void *data, int size) {
  auto *out = static_cast<std::vector<uint8_t> *>(context);
  auto *bytes = static_cast<uint8_t *>(data);
  out->insert(out->end(), bytes, bytes + size);
}

}  // namespace

std::string base64_encode(const uint8_t *data, size_t size) {
  std::string out;
  out.reserve((size + 2) / 3 * 4);
  for (size_t i = 0; i < size; i += 3) {
    uint32_t v = static_cast<uint32_t>(data[i]) << 16;
    if (i + 1 < size) v |= static_cast<uint32_t>(data[i + 1]) << 8;
    if (i + 2 < size) v |= data[i + 2];
    out.push_back(kB64[(v >> 18) & 63]);
    out.push_back(kB64[(v >> 12) & 63]);
    out.push_back(i + 1 < size ? kB64[(v >> 6) & 63] : '=');
    out.push_back(i + 2 < size ? kB64[v & 63] : '=');
  }
  return out;
}

std::expected<std::vector<uint8_t>, std::string> base64_decode(std::string_view text) {
  if (text.size() % 4 != 0) return std::unexpected(std::string("bad base64"));
  auto val = [](char c) -> int {
    if (c >= 'A' && c <= 'Z') return c - 'A';
    if (c >= 'a' && c <= 'z') return c - 'a' + 26;
    if (c >= '0' && c <= '9') return c - '0' + 52;
    if (c == '+') return 62;
    if (c == '/') return 63;
    return -1;
  };
  std::vector<uint8_t> out;
  for (size_t i = 0; i < text.size(); i += 4) {
    int pad = 0;
    uint32_t v = 0;
    for (size_t j = 0; j < 4; ++j) {
      char c = text[i + j];
      if (c == '=') {
        if (i + 4 != text.size() || j < 2) return std::unexpected(std::string("bad base64"));
        ++pad;
        v <<= 6;
      } else {
        if (pad) return std::unexpected(std::string("bad base64"));
        int d = val(c);
        if (d < 0) return std::unexpected(std::string("bad base64"));
        v = (v << 6) | static_cast<uint32_t>(d);
      }
    }
    out.push_back(static_cast<uint8_t>(v >> 16));
    if (pad < 2) out.push_back(static_cast<uint8_t>(v >> 8));
    if (pad < 1) out.push_back(static_cast<uint8_t>(v));
  }
  return out;
}

std::expected<std::vector<uint8_t>, std::string> encode_png(const PixelBuffer &buffer) {
  if (buffer.width <= 0 || buffer.height <= 0) return std::unexpected(std::string("empty image"));
  if (buffer.bgra.size() < static_cast<size_t>(buffer.width) * static_cast<size_t>(buffer.height) * 4) {
    return std::unexpected(std::string("short pixel buffer"));
  }
  std::vector<uint8_t> rgba(static_cast<size_t>(buffer.width) * static_cast<size_t>(buffer.height) * 4);
  for (int i = 0; i < buffer.width * buffer.height; ++i) {
    rgba[i * 4 + 0] = buffer.bgra[i * 4 + 2];
    rgba[i * 4 + 1] = buffer.bgra[i * 4 + 1];
    rgba[i * 4 + 2] = buffer.bgra[i * 4 + 0];
    rgba[i * 4 + 3] = buffer.bgra[i * 4 + 3];
  }
  std::vector<uint8_t> png;
  if (stbi_write_png_to_func(png_write, &png, buffer.width, buffer.height, 4, rgba.data(), buffer.width * 4) == 0) {
    return std::unexpected(std::string("png encode failed"));
  }
  return png;
}

std::expected<PixelBuffer, std::string> decode_png(const uint8_t *data, size_t size) {
  int width = 0;
  int height = 0;
  int channels = 0;
  unsigned char *rgba = stbi_load_from_memory(data, static_cast<int>(size), &width, &height, &channels, 4);
  if (rgba == nullptr || width <= 0 || height <= 0) return std::unexpected(std::string("png decode failed"));
  PixelBuffer buffer;
  buffer.width = width;
  buffer.height = height;
  buffer.bgra.resize(static_cast<size_t>(width) * static_cast<size_t>(height) * 4);
  for (int i = 0; i < width * height; ++i) {
    buffer.bgra[i * 4 + 0] = rgba[i * 4 + 2];
    buffer.bgra[i * 4 + 1] = rgba[i * 4 + 1];
    buffer.bgra[i * 4 + 2] = rgba[i * 4 + 0];
    buffer.bgra[i * 4 + 3] = rgba[i * 4 + 3];
  }
  stbi_image_free(rgba);
  return buffer;
}
