#pragma once

#include <cstddef>
#include <cstdint>
#include <expected>
#include <string>
#include <string_view>
#include <vector>

#include "capture.hpp"

std::expected<std::vector<uint8_t>, std::string> encode_png(const PixelBuffer &buffer);
std::expected<PixelBuffer, std::string> decode_png(const uint8_t *data, size_t size);
std::string base64_encode(const uint8_t *data, size_t size);
std::expected<std::vector<uint8_t>, std::string> base64_decode(std::string_view text);
