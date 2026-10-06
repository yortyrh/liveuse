#include "http_app.hpp"

#include <httplib.h>

#include <nlohmann/json.hpp>

#include <filesystem>
#include <optional>

#include "control.hpp"
#include "png.hpp"
#include "pointer.hpp"

using json = nlohmann::json;

namespace {

constexpr const char *kHostVersion = "0.3.0";

struct Outcome {
  int status = 200;
  json body = json::object();
  std::optional<PngImage> png;
};

json lines_body(const std::vector<std::string> &lines) { return json{{"lines", lines}}; }

std::optional<int> optional_int(const json &body, const char *key, std::string *error) {
  if (!body.contains(key) || body.at(key).is_null()) return std::nullopt;
  if (!body.at(key).is_number_integer()) {
    *error = std::string(key) + " must be an integer";
    return std::nullopt;
  }
  return body.at(key).get<int>();
}

double optional_number(const json &body, const char *key, double fallback, std::string *error) {
  if (!body.contains(key) || body.at(key).is_null()) return fallback;
  if (!body.at(key).is_number()) {
    *error = std::string(key) + " must be a number";
    return fallback;
  }
  return body.at(key).get<double>();
}

std::string optional_string(const json &body, const char *key, const std::string &fallback) {
  if (!body.contains(key) || body.at(key).is_null()) return fallback;
  if (!body.at(key).is_string()) return fallback;
  return body.at(key).get<std::string>();
}

bool optional_bool(const json &body, const char *key, bool fallback) {
  if (!body.contains(key) || body.at(key).is_null()) return fallback;
  if (!body.at(key).is_boolean()) return fallback;
  return body.at(key).get<bool>();
}

std::optional<Region> optional_region(const json &body, std::string *error) {
  if (!body.contains("region") || body.at("region").is_null()) return std::nullopt;
  const json &region = body.at("region");
  if (!region.is_array() || region.size() != 4) {
    *error = "region out of range";
    return std::nullopt;
  }
  for (const json &value : region) {
    if (!value.is_number_integer()) {
      *error = "region out of range";
      return std::nullopt;
    }
  }
  return Region{region[0].get<int>(), region[1].get<int>(), region[2].get<int>(), region[3].get<int>()};
}

Outcome from_error(const ControlError &error) { return Outcome{error.status, json{{"error", error.message}}, std::nullopt}; }

std::optional<std::vector<ObjectSpot>> object_spots(const json &args, std::string *error) {
  if (!args.contains("objects") || !args.at("objects").is_array() || args.at("objects").empty()) {
    *error = "objects is required";
    return std::nullopt;
  }
  std::vector<ObjectSpot> spots;
  for (const json &item : args.at("objects")) {
    if (!item.is_object()) {
      *error = "object must be an object";
      return std::nullopt;
    }
    auto number = [&](const char *key) -> std::optional<double> {
      if (!item.contains(key) || !item.at(key).is_number()) return std::nullopt;
      return item.at(key).get<double>();
    };
    const bool box = item.contains("left") || item.contains("top") || item.contains("width") || item.contains("height");
    if (box) {
      auto left = number("left");
      auto top = number("top");
      auto width = number("width");
      auto height = number("height");
      if (!left || !top || !width || !height) {
        *error = "object box needs left top width height";
        return std::nullopt;
      }
      if (*width <= 0 || *height <= 0) {
        *error = "object size out of range";
        return std::nullopt;
      }
      spots.push_back(ObjectSpot{*left, *top, *width, *height, true});
    } else {
      auto x = number("x");
      auto y = number("y");
      if (!x || !y) {
        *error = "object needs x y";
        return std::nullopt;
      }
      spots.push_back(ObjectSpot{*x, *y, 0, 0, false});
    }
  }
  return spots;
}

json object_moves_schema() {
  json spot = json{{"type", "object"},
                   {"description", "One target in screenshot pixels. Either a point (x,y) or a box (left,top,width,height)."},
                   {"properties", json{{"x", json{{"type", "number"}, {"description", "Screenshot pixel x. Pair with y. Do not also send a box."}}},
                                       {"y", json{{"type", "number"}, {"description", "Screenshot pixel y. Pair with x."}}},
                                       {"left", json{{"type", "number"}, {"description", "Box left edge in screenshot pixels. From locate."}}},
                                       {"top", json{{"type", "number"}, {"description", "Box top edge in screenshot pixels."}}},
                                       {"width", json{{"type", "number"}, {"description", "Box width in screenshot pixels. Center is the aim point."}}},
                                       {"height", json{{"type", "number"}, {"description", "Box height in screenshot pixels."}}}}}};
  json properties = json{{"objects", json{{"type", "array"},
                                          {"description", "Screenshot-pixel targets, in visit order. Not pointer points."},
                                          {"items", spot}}},
                         {"origin_x", json{{"type", "number"}, {"description", "Pointer x of this PNG's top-left. 0 for a full-screen screenshot. For a region capture, the region's left."}}},
                         {"origin_y", json{{"type", "number"}, {"description", "Pointer y of this PNG's top-left. 0 for a full-screen screenshot. For a region capture, the region's top."}}}};
  return json{{"type", "object"}, {"properties", properties}, {"required", json::array({"objects"})}};
}

json tool_schema(std::initializer_list<std::pair<const char *, const char *>> props, std::initializer_list<const char *> required) {
  json properties = json::object();
  for (const auto &prop : props) properties[prop.first] = json{{"type", prop.second}};
  json schema = json{{"type", "object"}, {"properties", properties}};
  if (required.size() != 0) {
    json names = json::array();
    for (const char *name : required) names.push_back(name);
    schema["required"] = names;
  }
  return schema;
}

json tool_list() {
  struct Item {
    const char *name;
    const char *description;
    json schema;
  };
  const Item items[] = {
      {"list_serial_ports", "List local serial ports", tool_schema({}, {})},
      {"connect_device", "Open the only USB device", tool_schema({}, {})},
      {"disconnect_device", "Close the CDC port", tool_schema({}, {})},
      {"device_status", "Connection and firmware status", tool_schema({}, {})},
      {"mouse_size", "Pointer grid: width and height are the only coordinates mouse_move_to accepts. pixel_width and pixel_height are the screenshot grid, often 2x. Divide screenshot pixels by pixel_width/width, or call mouse_moves_for_objects.", tool_schema({}, {})},
      {"mouse_position", "Where the pointer is now, in pointer coordinates (same grid as mouse_size width/height). Not a screenshot pixel.", tool_schema({}, {})},
      {"mouse_on_screen", "True when x,y is inside the pointer grid (mouse_size width/height), not the screenshot grid.", tool_schema({{"x", "integer"}, {"y", "integer"}}, {"x", "y"})},
      {"mouse_move_to", "Glide the pointer to x,y on the POINTER grid (mouse_position, mouse_size width/height). Never pass a screenshot, pixel, or locate coordinate here: those are backing-store pixels and will miss. Call mouse_moves_for_objects first and use moves[].x and moves[].y. Omit duration.", tool_schema({{"x", "integer"}, {"y", "integer"}, {"duration", "number"}, {"tween", "string"}}, {"x", "y"})},
      {"mouse_move", "Glide by dx,dy in pointer points (same grid as mouse_position). Omit duration. Not for converting a screenshot coordinate; use mouse_moves_for_objects.", tool_schema({{"dx", "integer"}, {"dy", "integer"}, {"duration", "number"}, {"tween", "string"}}, {"dx", "dy"})},
      {"mouse_moves_for_objects", "Required step between seeing an object and moving the pointer. Screenshot, pixel, and locate use backing-store pixels. mouse_move_to uses pointer points (often half). This divides by that scale. Pass objects in screenshot pixels: a point {\"x\":400,\"y\":200} or a locate box {\"left\":400,\"top\":200,\"width\":20,\"height\":10} (aims at the center). origin_x and origin_y are the PNG top-left in pointer points: 0 and 0 for a full-screen screenshot; for screenshot region [left,top,width,height] pass that left and top. Then call mouse_move_to with moves[].x and moves[].y. moves[].dx/dy is the step from the current pointer, then from the previous target.", object_moves_schema()},
      {"mouse_drag_to", "Glide a drag to x,y on the POINTER grid. Convert a screenshot or locate box with mouse_moves_for_objects first. Omit duration.", tool_schema({{"x", "integer"}, {"y", "integer"}, {"duration", "number"}, {"tween", "string"}, {"button", "string"}}, {"x", "y"})},
      {"mouse_drag", "Glide a drag by dx,dy in pointer points. Omit duration.", tool_schema({{"dx", "integer"}, {"dy", "integer"}, {"duration", "number"}, {"tween", "string"}, {"button", "string"}}, {"dx", "dy"})},
      {"mouse_click", "Click. Optional x,y are pointer coordinates. If the target came from a screenshot or locate, run mouse_moves_for_objects first and pass moves[].x and moves[].y.", tool_schema({{"x", "integer"}, {"y", "integer"}, {"button", "string"}, {"clicks", "integer"}, {"interval", "number"}}, {})},
      {"mouse_down", "Hold a mouse button", tool_schema({{"x", "integer"}, {"y", "integer"}, {"button", "string"}}, {})},
      {"mouse_up", "Release a mouse button", tool_schema({{"x", "integer"}, {"y", "integer"}, {"button", "string"}}, {})},
      {"mouse_scroll", "Vertical wheel. Positive is up", tool_schema({{"clicks", "integer"}, {"x", "integer"}, {"y", "integer"}}, {"clicks"})},
      {"mouse_hscroll", "Horizontal wheel. Positive is right", tool_schema({{"clicks", "integer"}, {"x", "integer"}, {"y", "integer"}}, {"clicks"})},
      {"keyboard_write", "Type printable ASCII", tool_schema({{"text", "string"}, {"interval", "number"}}, {"text"})},
      {"keyboard_press", "Tap a key", tool_schema({{"key", "string"}, {"presses", "integer"}, {"interval", "number"}}, {"key"})},
      {"keyboard_key_down", "Hold a key down", tool_schema({{"key", "string"}}, {"key"})},
      {"keyboard_key_up", "Release a key", tool_schema({{"key", "string"}}, {"key"})},
      {"keyboard_hotkey", "Press a chord, then release it in reverse", tool_schema({{"keys", "array"}}, {"keys"})},
      {"serial_echo", "Echo one CDC line", tool_schema({{"text", "string"}}, {"text"})},
      {"firmware_help", "Read the firmware help text", tool_schema({}, {})},
      {"send_line", "Send one raw CDC line", tool_schema({{"line", "string"}}, {"line"})},
      {"screenshot", "PNG, origin top-left, Y down. Pixel coordinates in this image are NOT mouse_move_to coordinates. Read mouse_size: width/height is the pointer grid, pixel_width/pixel_height is this image. To put the pointer on something you see, pass that box to mouse_moves_for_objects, then mouse_move_to moves[].x/y. Optional region is [left,top,width,height] in POINTER points; also pass that left,top as origin_x,origin_y.", tool_schema({{"region", "array"}}, {})},
      {"pixel", "RGB at screenshot pixel x,y (the image grid, not the pointer grid). To move the pointer there, call mouse_moves_for_objects with that x,y first.", tool_schema({{"x", "integer"}, {"y", "integer"}}, {"x", "y"})},
      {"pixel_matches_color", "Compare one pixel to an RGB color", tool_schema({{"x", "integer"}, {"y", "integer"}, {"red", "integer"}, {"green", "integer"}, {"blue", "integer"}, {"tolerance", "integer"}}, {"x", "y", "red", "green", "blue"})},
      {"locate_on_screen", "Find a PNG needle. The box left,top,width,height is in screenshot pixels, not pointer points. Pass that box to mouse_moves_for_objects before mouse_move_to or mouse_click.", tool_schema({{"needle", "string"}, {"grayscale", "boolean"}, {"confidence", "number"}, {"region", "array"}}, {"needle"})},
      {"locate_center_on_screen", "Center of a match as x,y in screenshot pixels, not pointer points. Pass {x,y} to mouse_moves_for_objects before moving or clicking.", tool_schema({{"needle", "string"}, {"grayscale", "boolean"}, {"confidence", "number"}, {"region", "array"}}, {"needle"})},
      {"locate_all_on_screen", "Every match. Each box is in screenshot pixels. Pass each box to mouse_moves_for_objects before moving.", tool_schema({{"needle", "string"}, {"grayscale", "boolean"}, {"confidence", "number"}, {"region", "array"}}, {"needle"})},
      {"locate", "Find a PNG needle inside a PNG haystack", tool_schema({{"needle", "string"}, {"haystack", "string"}, {"grayscale", "boolean"}, {"confidence", "number"}}, {"needle", "haystack"})},
      {"locate_all", "Every match of a PNG needle inside a PNG haystack", tool_schema({{"needle", "string"}, {"haystack", "string"}, {"grayscale", "boolean"}, {"confidence", "number"}}, {"needle", "haystack"})},
  };
  json tools = json::array();
  for (const Item &item : items) {
    tools.push_back(json{{"name", item.name}, {"description", item.description}, {"inputSchema", item.schema}});
  }
  return tools;
}

std::string cookie_value(const std::string &header) {
  const std::string key = "liveuse_session=";
  size_t at = header.find(key);
  if (at == std::string::npos) return {};
  at += key.size();
  size_t end = header.find(';', at);
  return header.substr(at, end == std::string::npos ? std::string::npos : end - at);
}

}  // namespace

HttpApp::HttpApp(Config config, CdcLink &link, ScreenCapture &screen, std::filesystem::path config_file)
    : config_(std::move(config)), link_(link), screen_(screen),
      config_file_(config_file.empty() ? config_path() : std::move(config_file)) {}

void HttpApp::reload(Config config) {
  std::lock_guard lock(mu_);
  config_ = std::move(config);
  sessions_.clear();
}

void HttpApp::set_public_base(std::string base) { public_base_ = std::move(base); }

void HttpApp::attach(httplib::Server &server, const std::string &web_root) {
  auto outcome_for = [this](const std::string &name, const json &args) -> Outcome {
    Control control(link_, screen_, nullptr);
    std::string error;
    auto need_int = [&](const char *key) -> std::optional<int> {
      auto value = optional_int(args, key, &error);
      if (!error.empty()) return std::nullopt;
      if (!value) error = std::string(key) + " is required";
      return value;
    };
    auto pair_xy = [&]() -> std::pair<std::optional<int>, std::optional<int>> {
      auto x = optional_int(args, "x", &error);
      auto y = optional_int(args, "y", &error);
      return {x, y};
    };
    if (name == "list_serial_ports") {
      json ports = json::array();
      for (const PortInfo &port : link_.list_ports()) ports.push_back(json{{"name", port.name}, {"kind", port.kind}});
      return {200, json{{"ports", ports}}, std::nullopt};
    }
    if (name == "connect_device") {
      auto found = only_device_port(link_.list_ports());
      if (!found) return {found.error().status, json{{"error", found.error().message}}, std::nullopt};
      auto banner = link_.connect(*found);
      if (!banner) return {banner.error().status, json{{"error", banner.error().message}}, std::nullopt};
      return {200, json{{"port", link_.port()}, {"banner", *banner}}, std::nullopt};
    }
    if (name == "disconnect_device") {
      auto previous = link_.disconnect();
      if (!previous) return {previous.error().status, json{{"error", previous.error().message}}, std::nullopt};
      return {200, json{{"disconnected", true}, {"port", *previous}}, std::nullopt};
    }
    if (name == "device_status") {
      json firmware = nullptr;
      if (link_.connected()) {
        auto status = control.send_line("STATUS");
        if (status) {
          for (const std::string &line : *status) {
            if (line.rfind("OK ", 0) == 0) firmware = line;
          }
        }
      }
      return {200, json{{"connected", link_.connected()}, {"port", link_.port()}, {"firmware", firmware}}, std::nullopt};
    }
    if (name == "mouse_size") {
      auto grid = control.pointer_grid();
      if (!grid) return from_error(grid.error());
      return {200,
              json{{"width", grid->width},
                   {"height", grid->height},
                   {"pixel_width", grid->pixel_width},
                   {"pixel_height", grid->pixel_height}},
              std::nullopt};
    }
    if (name == "mouse_position") {
      auto got = control.mouse_position();
      if (!got) return from_error(got.error());
      return {200, json{{"x", got->first}, {"y", got->second}}, std::nullopt};
    }
    if (name == "mouse_on_screen") {
      auto x = need_int("x");
      auto y = need_int("y");
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      auto got = control.mouse_on_screen(*x, *y);
      if (!got) return from_error(got.error());
      return {200, json{{"onScreen", *got}}, std::nullopt};
    }
    auto move_args = [&](bool absolute) -> Outcome {
      const double duration = optional_number(args, "duration", 0, &error);
      const std::string tween = optional_string(args, "tween", "linear");
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      if (absolute) {
        auto x = need_int("x");
        auto y = need_int("y");
        if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
        auto got = control.mouse_move_to(*x, *y, duration, tween);
        if (!got) return from_error(got.error());
        return {200, lines_body(*got), std::nullopt};
      }
      auto dx = need_int("dx");
      auto dy = need_int("dy");
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      auto got = control.mouse_move(*dx, *dy, duration, tween);
      if (!got) return from_error(got.error());
      return {200, lines_body(*got), std::nullopt};
    };
    if (name == "mouse_move_to") return move_args(true);
    if (name == "mouse_move") return move_args(false);
    if (name == "mouse_moves_for_objects") {
      const double origin_x = optional_number(args, "origin_x", 0, &error);
      const double origin_y = optional_number(args, "origin_y", 0, &error);
      auto spots = object_spots(args, &error);
      if (!error.empty() || !spots) return {400, json{{"error", error.empty() ? "objects is required" : error}}, std::nullopt};
      auto grid = control.pointer_grid();
      if (!grid) return from_error(grid.error());
      auto pos = control.mouse_position();
      if (!pos) return from_error(pos.error());
      auto moves = moves_for_objects(*grid, pos->first, pos->second, origin_x, origin_y, *spots);
      if (!moves) return {400, json{{"error", moves.error()}}, std::nullopt};
      json list = json::array();
      for (const PointerMove &move : *moves) {
        const bool on = move.x >= 0 && move.y >= 0 && move.x < grid->width && move.y < grid->height;
        list.push_back(json{{"x", move.x}, {"y", move.y}, {"dx", move.dx}, {"dy", move.dy}, {"on_screen", on}});
      }
      return {200,
              json{{"width", grid->width},
                   {"height", grid->height},
                   {"pixel_width", grid->pixel_width},
                   {"pixel_height", grid->pixel_height},
                   {"from", json{{"x", pos->first}, {"y", pos->second}}},
                   {"moves", list}},
              std::nullopt};
    }
    if (name == "mouse_drag_to" || name == "mouse_drag") {
      const double duration = optional_number(args, "duration", 0, &error);
      const std::string tween = optional_string(args, "tween", "linear");
      const std::string button = optional_string(args, "button", "left");
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      if (name == "mouse_drag_to") {
        auto x = need_int("x");
        auto y = need_int("y");
        if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
        auto got = control.mouse_drag_to(*x, *y, duration, tween, button);
        if (!got) return from_error(got.error());
        return {200, lines_body(*got), std::nullopt};
      }
      auto dx = need_int("dx");
      auto dy = need_int("dy");
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      auto got = control.mouse_drag(*dx, *dy, duration, tween, button);
      if (!got) return from_error(got.error());
      return {200, lines_body(*got), std::nullopt};
    }
    if (name == "mouse_click" || name == "mouse_down" || name == "mouse_up") {
      auto [x, y] = pair_xy();
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      const std::string button = optional_string(args, "button", "left");
      if (name == "mouse_down") {
        auto got = control.mouse_down(x, y, button);
        if (!got) return from_error(got.error());
        return {200, lines_body(*got), std::nullopt};
      }
      if (name == "mouse_up") {
        auto got = control.mouse_up(x, y, button);
        if (!got) return from_error(got.error());
        return {200, lines_body(*got), std::nullopt};
      }
      const int clicks = args.contains("clicks") ? args.value("clicks", 1) : 1;
      const double interval = optional_number(args, "interval", 0, &error);
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      auto got = control.mouse_click(x, y, button, clicks, interval);
      if (!got) return from_error(got.error());
      return {200, lines_body(*got), std::nullopt};
    }
    if (name == "mouse_scroll" || name == "mouse_hscroll") {
      auto clicks = need_int("clicks");
      auto [x, y] = pair_xy();
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      auto got = name == "mouse_scroll" ? control.mouse_scroll(*clicks, x, y) : control.mouse_hscroll(*clicks, x, y);
      if (!got) return from_error(got.error());
      return {200, lines_body(*got), std::nullopt};
    }
    if (name == "keyboard_write") {
      if (!args.contains("text") || !args.at("text").is_string()) return {400, json{{"error", "text is required"}}, std::nullopt};
      const double interval = optional_number(args, "interval", 0, &error);
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      auto got = control.keyboard_write(args.at("text").get<std::string>(), interval);
      if (!got) return from_error(got.error());
      return {200, json{{"lines", got->first}, {"chunks", got->second}}, std::nullopt};
    }
    if (name == "keyboard_press") {
      if (!args.contains("key") || !args.at("key").is_string()) return {400, json{{"error", "key is required"}}, std::nullopt};
      const int presses = args.value("presses", 1);
      const double interval = optional_number(args, "interval", 0, &error);
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      auto got = control.keyboard_press(args.at("key").get<std::string>(), presses, interval);
      if (!got) return from_error(got.error());
      return {200, lines_body(*got), std::nullopt};
    }
    if (name == "keyboard_key_down" || name == "keyboard_key_up") {
      if (!args.contains("key") || !args.at("key").is_string()) return {400, json{{"error", "key is required"}}, std::nullopt};
      const std::string key = args.at("key").get<std::string>();
      auto got = name == "keyboard_key_down" ? control.keyboard_down(key) : control.keyboard_up(key);
      if (!got) return from_error(got.error());
      return {200, lines_body(*got), std::nullopt};
    }
    if (name == "keyboard_hotkey") {
      if (!args.contains("keys") || !args.at("keys").is_array()) return {400, json{{"error", "keys is required"}}, std::nullopt};
      std::vector<std::string> keys;
      for (const json &key : args.at("keys")) {
        if (!key.is_string()) return {400, json{{"error", "unknown key"}}, std::nullopt};
        keys.push_back(key.get<std::string>());
      }
      auto got = control.keyboard_hotkey(keys);
      if (!got) return from_error(got.error());
      return {200, lines_body(*got), std::nullopt};
    }
    if (name == "serial_echo") {
      if (!args.contains("text") || !args.at("text").is_string()) return {400, json{{"error", "text is required"}}, std::nullopt};
      auto got = control.echo(args.at("text").get<std::string>());
      if (!got) return from_error(got.error());
      return {200, lines_body(*got), std::nullopt};
    }
    if (name == "firmware_help") {
      auto got = control.help();
      if (!got) return from_error(got.error());
      return {200, lines_body(*got), std::nullopt};
    }
    if (name == "send_line") {
      if (!args.contains("line") || !args.at("line").is_string()) return {400, json{{"error", "line is required"}}, std::nullopt};
      auto got = control.send_line(args.at("line").get<std::string>());
      if (!got) return from_error(got.error());
      return {200, lines_body(*got), std::nullopt};
    }
    if (name == "screenshot") {
      auto region = optional_region(args, &error);
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      auto got = control.screenshot(region ? &*region : nullptr);
      if (!got) return from_error(got.error());
      return {200, json::object(), std::optional<PngImage>{std::move(*got)}};
    }
    if (name == "pixel") {
      auto x = need_int("x");
      auto y = need_int("y");
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      auto got = control.pixel(*x, *y);
      if (!got) return from_error(got.error());
      return {200, json{{"red", got->red}, {"green", got->green}, {"blue", got->blue}}, std::nullopt};
    }
    if (name == "pixel_matches_color") {
      auto x = need_int("x");
      auto y = need_int("y");
      auto red = need_int("red");
      auto green = need_int("green");
      auto blue = need_int("blue");
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      int tolerance = 0;
      if (args.contains("tolerance") && !args.at("tolerance").is_null()) {
        if (!args.at("tolerance").is_number_integer()) return {400, json{{"error", "tolerance must be an integer"}}, std::nullopt};
        tolerance = args.at("tolerance").get<int>();
      }
      auto got = control.pixel_matches(*x, *y, *red, *green, *blue, tolerance);
      if (!got) return from_error(got.error());
      return {200, json{{"match", *got}}, std::nullopt};
    }
    auto locate_call = [&](bool on_screen, bool all, bool center) -> Outcome {
      if (!args.contains("needle") || !args.at("needle").is_string()) return {400, json{{"error", "needle is required"}}, std::nullopt};
      auto needle_bytes = base64_decode(args.at("needle").get<std::string>());
      if (!needle_bytes) return {400, json{{"error", needle_bytes.error()}}, std::nullopt};
      auto needle = decode_png(needle_bytes->data(), needle_bytes->size());
      if (!needle) return {400, json{{"error", needle.error()}}, std::nullopt};
      std::optional<PixelBuffer> haystack;
      if (!on_screen) {
        if (!args.contains("haystack") || !args.at("haystack").is_string() || args.at("haystack").get<std::string>().empty()) {
          return {400, json{{"error", "haystack is required"}}, std::nullopt};
        }
        auto hay_bytes = base64_decode(args.at("haystack").get<std::string>());
        if (!hay_bytes) return {400, json{{"error", hay_bytes.error()}}, std::nullopt};
        auto decoded = decode_png(hay_bytes->data(), hay_bytes->size());
        if (!decoded) return {400, json{{"error", decoded.error()}}, std::nullopt};
        haystack = std::move(*decoded);
      }
      const bool grayscale = optional_bool(args, "grayscale", false);
      const double confidence = optional_number(args, "confidence", 1, &error);
      auto region = optional_region(args, &error);
      if (!error.empty()) return {400, json{{"error", error}}, std::nullopt};
      auto found = control.locate(haystack ? &*haystack : nullptr, *needle, grayscale, confidence, region ? &*region : nullptr);
      if (!found) return from_error(found.error());
      if (center) {
        const Box &box = found->front();
        return {200, json{{"x", box.left + box.width / 2}, {"y", box.top + box.height / 2}}, std::nullopt};
      }
      if (all) {
        json boxes = json::array();
        for (const Box &box : *found) boxes.push_back(json{{"left", box.left}, {"top", box.top}, {"width", box.width}, {"height", box.height}});
        return {200, json{{"boxes", boxes}}, std::nullopt};
      }
      const Box &box = found->front();
      return {200, json{{"left", box.left}, {"top", box.top}, {"width", box.width}, {"height", box.height}}, std::nullopt};
    };
    if (name == "locate_on_screen") return locate_call(true, false, false);
    if (name == "locate_center_on_screen") return locate_call(true, false, true);
    if (name == "locate_all_on_screen") return locate_call(true, true, false);
    if (name == "locate") return locate_call(false, false, false);
    if (name == "locate_all") return locate_call(false, true, false);
    return {400, json{{"error", "unknown tool"}}, std::nullopt};
  };

  auto bearer_ok = [this](const httplib::Request &req) {
    const std::string header = req.get_header_value("Authorization");
    const std::string prefix = "Bearer ";
    if (header.rfind(prefix, 0) != 0) return false;
    std::lock_guard lock(mu_);
    return token_equal(header.substr(prefix.size()), config_.token);
  };
  auto session_user = [this](const httplib::Request &req) -> std::optional<std::string> {
    const std::string token = cookie_value(req.get_header_value("Cookie"));
    if (token.empty()) return std::nullopt;
    std::lock_guard lock(mu_);
    auto found = sessions_.find(token);
    if (found == sessions_.end()) return std::nullopt;
    if (found->second.expires < std::chrono::steady_clock::now()) {
      sessions_.erase(found);
      return std::nullopt;
    }
    return found->second.username;
  };
  auto require_cookie = [session_user](const httplib::Request &req, httplib::Response &res) {
    if (session_user(req)) return true;
    res.status = 401;
    res.set_content(json{{"error", "credenciales invalidas"}}.dump(), "application/json");
    return false;
  };

  server.Post("/api/login", [this](const httplib::Request &req, httplib::Response &res) {
    json body;
    try {
      body = json::parse(req.body);
    } catch (const json::exception &) {
      res.status = 401;
      res.set_content(json{{"error", "credenciales invalidas"}}.dump(), "application/json");
      return;
    }
    const std::string username = body.value("username", "");
    const std::string password = body.value("password", "");
    std::lock_guard lock(mu_);
    const bool password_ok = verify_password(password, config_.password_hash);
    if (!password_ok || username != config_.username) {
      res.status = 401;
      res.set_content(json{{"error", "credenciales invalidas"}}.dump(), "application/json");
      return;
    }
    const std::string token = make_token();
    sessions_[token] = Session{username, std::chrono::steady_clock::now() + std::chrono::hours(12)};
    res.set_header("Set-Cookie", "liveuse_session=" + token + "; Path=/; HttpOnly; SameSite=Lax; Max-Age=43200");
    res.set_content(json{{"username", username}, {"name", config_.name}}.dump(), "application/json");
  });

  server.Post("/api/logout", [this, session_user](const httplib::Request &req, httplib::Response &res) {
    const std::string token = cookie_value(req.get_header_value("Cookie"));
    {
      std::lock_guard lock(mu_);
      sessions_.erase(token);
    }
    res.set_header("Set-Cookie", "liveuse_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    res.set_content(json{{"ok", true}}.dump(), "application/json");
    (void)session_user;
  });

  server.Get("/api/session", [this, session_user](const httplib::Request &req, httplib::Response &res) {
    auto user = session_user(req);
    if (!user) {
      res.status = 401;
      res.set_content(json{{"error", "credenciales invalidas"}}.dump(), "application/json");
      return;
    }
    std::lock_guard lock(mu_);
    res.set_content(json{{"username", *user}, {"name", config_.name}}.dump(), "application/json");
  });

  server.Get("/api/config", [this, require_cookie](const httplib::Request &req, httplib::Response &res) {
    if (!require_cookie(req, res)) return;
    std::lock_guard lock(mu_);
    res.set_content(json{{"name", config_.name}, {"mcpUrl", public_base_ + "/mcp"}, {"token", config_.token}}.dump(), "application/json");
  });

  server.Put("/api/config", [this, require_cookie](const httplib::Request &req, httplib::Response &res) {
    if (!require_cookie(req, res)) return;
    json body;
    try {
      body = json::parse(req.body);
    } catch (const json::exception &) {
      res.status = 400;
      res.set_content(json{{"error", "name is required"}}.dump(), "application/json");
      return;
    }
    if (!body.contains("name") || !body.at("name").is_string() || body.at("name").get<std::string>().empty()) {
      res.status = 400;
      res.set_content(json{{"error", "name is required"}}.dump(), "application/json");
      return;
    }
    std::lock_guard lock(mu_);
    config_.name = body.at("name").get<std::string>();
    try {
      save_config(config_file_, config_);
    } catch (const std::exception &ex) {
      res.status = 500;
      res.set_content(json{{"error", ex.what()}}.dump(), "application/json");
      return;
    }
    res.set_content(json{{"name", config_.name}, {"mcpUrl", public_base_ + "/mcp"}, {"token", config_.token}}.dump(), "application/json");
  });

  auto post_json = [&](const char *path, const std::string &tool) {
    server.Post(path, [this, require_cookie, outcome_for, tool](const httplib::Request &req, httplib::Response &res) {
      if (!require_cookie(req, res)) return;
      json body = json::object();
      if (!req.body.empty()) {
        try {
          body = json::parse(req.body);
        } catch (const json::exception &) {
          res.status = 400;
          res.set_content(json{{"error", "invalid json"}}.dump(), "application/json");
          return;
        }
      }
      std::lock_guard lock(mu_);
      Outcome outcome = outcome_for(tool, body);
      if (outcome.png) {
        res.status = 200;
        res.set_header("X-Image-Width", std::to_string(outcome.png->width));
        res.set_header("X-Image-Height", std::to_string(outcome.png->height));
        res.set_content(std::string(outcome.png->bytes.begin(), outcome.png->bytes.end()), "image/png");
        return;
      }
      res.status = outcome.status;
      res.set_content(outcome.body.dump(), "application/json");
    });
  };
  auto get_query = [&](const char *path, const std::string &tool) {
    server.Get(path, [this, require_cookie, outcome_for, tool](const httplib::Request &req, httplib::Response &res) {
      if (!require_cookie(req, res)) return;
      json body = json::object();
      for (const auto &param : req.params) {
        try {
          body[param.first] = std::stoi(param.second);
        } catch (const std::exception &) {
          body[param.first] = param.second;
        }
      }
      std::lock_guard lock(mu_);
      Outcome outcome = outcome_for(tool, body);
      res.status = outcome.status;
      res.set_content(outcome.body.dump(), "application/json");
    });
  };

  server.Get("/api/ports", [this, require_cookie, outcome_for](const httplib::Request &req, httplib::Response &res) {
    if (!require_cookie(req, res)) return;
    std::lock_guard lock(mu_);
    Outcome outcome = outcome_for("list_serial_ports", json::object());
    res.status = outcome.status;
    res.set_content(outcome.body.dump(), "application/json");
  });
  post_json("/api/connect", "connect_device");
  post_json("/api/disconnect", "disconnect_device");
  get_query("/api/status", "device_status");
  get_query("/api/mouse/size", "mouse_size");
  get_query("/api/mouse/position", "mouse_position");
  get_query("/api/mouse/on-screen", "mouse_on_screen");
  post_json("/api/mouse/move-to", "mouse_move_to");
  post_json("/api/mouse/move", "mouse_move");
  post_json("/api/mouse/moves-for-objects", "mouse_moves_for_objects");
  post_json("/api/mouse/drag-to", "mouse_drag_to");
  post_json("/api/mouse/drag", "mouse_drag");
  post_json("/api/mouse/click", "mouse_click");
  post_json("/api/mouse/down", "mouse_down");
  post_json("/api/mouse/up", "mouse_up");
  post_json("/api/mouse/scroll", "mouse_scroll");
  post_json("/api/mouse/hscroll", "mouse_hscroll");
  post_json("/api/echo", "serial_echo");
  post_json("/api/help", "firmware_help");
  post_json("/api/keyboard/write", "keyboard_write");
  post_json("/api/keyboard/press", "keyboard_press");
  post_json("/api/keyboard/down", "keyboard_key_down");
  post_json("/api/keyboard/up", "keyboard_key_up");
  post_json("/api/keyboard/hotkey", "keyboard_hotkey");
  post_json("/api/screenshot", "screenshot");
  get_query("/api/pixel", "pixel");
  get_query("/api/pixel-matches", "pixel_matches_color");
  server.Post("/api/locate", [this, require_cookie, outcome_for](const httplib::Request &req, httplib::Response &res) {
    if (!require_cookie(req, res)) return;
    json body;
    try {
      body = json::parse(req.body);
    } catch (const json::exception &) {
      res.status = 400;
      res.set_content(json{{"error", "invalid json"}}.dump(), "application/json");
      return;
    }
    const std::string which = body.value("which", "box");
    std::string tool = "locate_on_screen";
    if (!body.contains("haystack") || body.at("haystack").is_null() || (body.at("haystack").is_string() && body.at("haystack").get<std::string>().empty())) {
      if (which == "center") tool = "locate_center_on_screen";
      else if (which == "all") tool = "locate_all_on_screen";
    } else {
      tool = which == "all" ? "locate_all" : "locate";
    }
    std::lock_guard lock(mu_);
    Outcome outcome = outcome_for(tool, body);
    res.status = outcome.status;
    res.set_content(outcome.body.dump(), "application/json");
  });
  post_json("/api/command", "send_line");

  server.Post("/mcp", [this, bearer_ok, outcome_for](const httplib::Request &req, httplib::Response &res) {
    if (!bearer_ok(req)) {
      res.status = 401;
      res.set_content("unauthorized", "text/plain");
      return;
    }
    json body;
    try {
      body = json::parse(req.body);
    } catch (const json::exception &) {
      res.status = 400;
      res.set_content(json{{"jsonrpc", "2.0"}, {"id", nullptr}, {"error", json{{"code", -32700}, {"message", "parse error"}}}}.dump(), "application/json");
      return;
    }
    const std::string method = body.value("method", "");
    json id = body.contains("id") ? body.at("id") : json(nullptr);
    json result;
    bool rpc_error = false;
    std::string rpc_message;
    if (method == "initialize") {
      std::string name;
      {
        std::lock_guard lock(mu_);
        name = config_.name;
      }
      result = json{{"protocolVersion", "2024-11-05"},
                    {"capabilities", json{{"tools", json::object()}}},
                    {"serverInfo", json{{"name", "liveuse"}, {"version", kHostVersion}}},
                    {"instructions", "Device " + name + ". Two coordinate grids. Pointer tools (mouse_position, mouse_size width/height, mouse_move_to, mouse_move, mouse_drag_to, mouse_click x/y) use pointer points. screenshot, pixel, and locate use backing-store pixels (mouse_size pixel_width/pixel_height, often 2x). Never pass a screenshot or locate number to mouse_move_to. Workflow: find the object, call mouse_moves_for_objects with that pixel point or box, then mouse_move_to(moves[].x, moves[].y). Omit duration. The pointer glides; do not add pauses. A missing device returns device not found. Size, position, screenshot, pixel, and locate do not use the serial port."}};
    } else if (method == "ping") {
      result = json::object();
    } else if (method == "tools/list") {
      result = json{{"tools", tool_list()}};
    } else if (method == "tools/call") {
      const json params = body.value("params", json::object());
      const std::string tool = params.value("name", "");
      const json arguments = params.value("arguments", json::object());
      std::lock_guard lock(mu_);
      Outcome outcome = outcome_for(tool, arguments);
      json content = json::array();
      if (outcome.png) {
        content.push_back(json{{"type", "image"},
                               {"data", base64_encode(outcome.png->bytes.data(), outcome.png->bytes.size())},
                               {"mimeType", "image/png"}});
      } else if (outcome.status == 200) {
        content.push_back(json{{"type", "text"}, {"text", outcome.body.dump()}});
      } else {
        content.push_back(json{{"type", "text"}, {"text", outcome.body.value("error", "error")}});
      }
      result = json{{"content", content}, {"isError", outcome.status != 200}};
    } else {
      rpc_error = true;
      rpc_message = "method not found";
    }
    json response = json{{"jsonrpc", "2.0"}, {"id", id}};
    if (rpc_error) response["error"] = json{{"code", -32601}, {"message", rpc_message}};
    else response["result"] = result;
    res.set_content(response.dump(), "application/json");
  });

  if (!web_root.empty() && std::filesystem::exists(web_root)) {
    server.set_mount_point("/", web_root);
  } else {
    server.Get("/", [web_root](const httplib::Request &, httplib::Response &res) {
      res.set_content("missing web root: " + web_root, "text/plain");
    });
  }
}
