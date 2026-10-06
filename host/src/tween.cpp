#include "tween.hpp"

#include <cmath>

namespace {

double ease_out_bounce(double t) {
  constexpr double n1 = 7.5625;
  constexpr double d1 = 2.75;
  if (t < 1.0 / d1) return n1 * t * t;
  if (t < 2.0 / d1) {
    t -= 1.5 / d1;
    return n1 * t * t + 0.75;
  }
  if (t < 2.5 / d1) {
    t -= 2.25 / d1;
    return n1 * t * t + 0.9375;
  }
  t -= 2.625 / d1;
  return n1 * t * t + 0.984375;
}

double ease_out_elastic(double t) {
  constexpr double c4 = (2.0 * 3.14159265358979323846) / 3.0;
  if (t == 0.0 || t == 1.0) return t;
  return std::pow(2.0, -10.0 * t) * std::sin((t * 10.0 - 0.75) * c4) + 1.0;
}

double ease_in_elastic(double t) {
  constexpr double c4 = (2.0 * 3.14159265358979323846) / 3.0;
  if (t == 0.0 || t == 1.0) return t;
  return -std::pow(2.0, 10.0 * t - 10.0) * std::sin((t * 10.0 - 10.75) * c4);
}

}  // namespace

std::expected<double, std::string> tween_at(std::string_view name, double t) {
  if (name.empty() || name == "linear") return t;
  if (name == "easeInQuad") return t * t;
  if (name == "easeOutQuad") return t * (2.0 - t);
  if (name == "easeInOutQuad") return t < 0.5 ? 2.0 * t * t : -1.0 + (4.0 - 2.0 * t) * t;
  if (name == "easeInBounce") return 1.0 - ease_out_bounce(1.0 - t);
  if (name == "easeOutElastic") return ease_out_elastic(t);
  if (name == "easeInElastic") return ease_in_elastic(t);
  return std::unexpected(std::string("unknown tween"));
}
