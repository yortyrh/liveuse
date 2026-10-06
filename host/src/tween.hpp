#pragma once

#include <expected>
#include <string>
#include <string_view>

std::expected<double, std::string> tween_at(std::string_view name, double t);
