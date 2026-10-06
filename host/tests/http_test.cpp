#include <httplib.h>

#include <iostream>
#include <thread>

#include <nlohmann/json.hpp>

#include "http_app.hpp"

namespace {
int failures = 0;
void check(bool ok, const char *name) {
  if (!ok) {
    std::cerr << "FAIL " << name << "\n";
    ++failures;
  }
}

class FakeLink final : public CdcLink {
 public:
  std::vector<PortInfo> list_ports() override { return ports; }
  std::vector<PortInfo> ports{{"/dev/cu.usbmodem1", "usb"}, {"/dev/cu.Bluetooth-Incoming-Port", "other"}};
  std::expected<std::vector<std::string>, CdcError> connect(const std::string &port) override {
    open_ = true;
    port_ = port;
    return std::vector<std::string>{"banner"};
  }
  std::expected<std::string, CdcError> disconnect() override {
    open_ = false;
    const std::string previous = port_;
    port_.clear();
    return previous;
  }
  bool connected() const override { return open_; }
  std::string port() const override { return port_; }
  std::expected<std::vector<std::string>, CdcError> write_lines(const std::vector<std::string> &lines, bool) override {
    if (!open_) return std::unexpected(CdcError{409, "serial closed"});
    std::vector<std::string> out;
    for (const std::string &line : lines) {
      written.push_back(line);
      if (line.rfind("MOVE ", 0) == 0) out.push_back("MOVE done");
      else if (line.rfind("STATUS", 0) == 0) out.push_back("OK uptime_ms=1 fw=0.4.0");
      else if (line.rfind("TYPE ", 0) == 0) continue;
      else out.push_back(line + " done");
    }
    return out;
  }
  std::vector<std::string> written;

 private:
  bool open_ = false;
  std::string port_;
};

class FakeScreen final : public ScreenCapture {
 public:
  std::expected<std::pair<int, int>, std::string> size() override { return std::pair<int, int>{100, 50}; }
  std::expected<std::pair<int, int>, std::string> cursor() override { return std::pair<int, int>{1, 2}; }
  std::expected<PixelBuffer, std::string> capture(const Region *region) override {
    PixelBuffer buffer;
    buffer.width = region ? region->width : 4;
    buffer.height = region ? region->height : 4;
    buffer.bgra.assign(static_cast<size_t>(buffer.width * buffer.height * 4), 0);
    if (!buffer.bgra.empty()) {
      buffer.bgra[0] = 1;
      buffer.bgra[1] = 2;
      buffer.bgra[2] = 3;
      buffer.bgra[3] = 255;
    }
    return buffer;
  }
};
}  // namespace

int main() {
  FakeLink link;
  FakeScreen screen;
  HttpApp app(make_config("ada", "secret"), link, screen, "/tmp/liveuse-http-test-config.json");
  httplib::Server server;
  app.attach(server, "");
  const int port = server.bind_to_any_port("127.0.0.1");
  check(port > 0, "bind");
  std::thread worker([&server] { server.listen_after_bind(); });
  httplib::Client client("127.0.0.1", port);
  auto missing = client.Post("/mcp", "{}", "application/json");
  check(missing && missing->status == 401, "mcp_401");
  auto bad = client.Post("/mcp", {{"Authorization", "Bearer nope"}}, "{}", "application/json");
  check(bad && bad->status == 401, "mcp_bad_bearer");
  auto no_cookie = client.Get("/api/mouse/size");
  check(no_cookie && no_cookie->status == 401, "api_401");
  auto login = client.Post("/api/login", nlohmann::json{{"username", "ada"}, {"password", "secret"}}.dump(), "application/json");
  check(login && login->status == 200, "login");
  const std::string set_cookie = login ? login->get_header_value("Set-Cookie") : "";
  const std::string cookie = set_cookie.substr(0, set_cookie.find(';'));
  client.set_default_headers({{"Cookie", cookie}});
  auto bad_login = client.Post("/api/login", nlohmann::json{{"username", "ada"}, {"password", "nope"}}.dump(), "application/json");
  check(bad_login && bad_login->status == 401 && bad_login->body.find("credenciales invalidas") != std::string::npos, "bad_login");

  auto cfg_res = client.Get("/api/config");
  check(cfg_res && cfg_res->status == 200, "config");
  const std::string token = cfg_res ? nlohmann::json::parse(cfg_res->body).at("token").get<std::string>() : "";
  auto listed = client.Post("/mcp", {{"Authorization", "Bearer " + token}},
                            nlohmann::json{{"jsonrpc", "2.0"}, {"id", 1}, {"method", "tools/list"}}.dump(), "application/json");
  check(listed && listed->status == 200, "tools_http");
  if (listed) {
    for (const char *name : {"mouse_move_to", "mouse_drag", "keyboard_hotkey", "keyboard_key_down", "screenshot", "locate_on_screen", "mouse_moves_for_objects"}) {
      check(listed->body.find(name) != std::string::npos, name);
    }
  }
  auto size = client.Get("/api/mouse/size");
  check(size && size->status == 200 && size->body.find("\"width\":100") != std::string::npos &&
            size->body.find("\"pixel_width\":100") != std::string::npos,
        "size");
  auto aimed = client.Post("/api/mouse/moves-for-objects",
                           nlohmann::json{{"objects", nlohmann::json::array({nlohmann::json{{"x", 10}, {"y", 20}}})}}.dump(),
                           "application/json");
  check(aimed && aimed->status == 200 && aimed->body.find("\"x\":10") != std::string::npos &&
            aimed->body.find("\"dx\":9") != std::string::npos && aimed->body.find("\"dy\":18") != std::string::npos,
        "moves_for_objects");
  auto slide = client.Post("/api/mouse/move-to", nlohmann::json{{"x", 4}, {"y", 6}}.dump(), "application/json");
  check(slide && slide->status == 200, "move_to");
  check(link.port() == "/dev/cu.usbmodem1", "auto_port");
  check(!link.written.empty() && link.written[0].rfind("MOVE ", 0) == 0, "move_line");
  check(client.Post("/api/disconnect", "{}", "application/json")->status == 200, "disconnect");
  link.ports = {{"/dev/cu.Bluetooth-Incoming-Port", "other"}};
  auto missing_device = client.Post("/api/mouse/move-to", nlohmann::json{{"x", 1}, {"y", 1}}.dump(), "application/json");
  check(missing_device && missing_device->status == 404 && missing_device->body.find("device not found") != std::string::npos,
        "device_not_found");
  auto closed = client.Post("/api/keyboard/hotkey", nlohmann::json{{"keys", nlohmann::json::array({"volumeup"})}}.dump(), "application/json");
  check(closed && closed->status == 400, "no_hid_key");

  server.stop();
  worker.join();
  return failures == 0 ? 0 : 1;
}
