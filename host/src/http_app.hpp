#pragma once

#include <chrono>
#include <filesystem>
#include <mutex>
#include <string>
#include <unordered_map>

#include "capture.hpp"
#include "config.hpp"
#include "link.hpp"

namespace httplib {
class Server;
}

class HttpApp {
 public:
  HttpApp(Config config, CdcLink &link, ScreenCapture &screen, std::filesystem::path config_file = {});
  void attach(httplib::Server &server, const std::string &web_root);
  void reload(Config config);
  void set_public_base(std::string base);

 private:
  Config config_;
  CdcLink &link_;
  ScreenCapture &screen_;
  std::string public_base_ = "http://127.0.0.1:7395";
  std::filesystem::path config_file_;
  std::mutex mu_;
  struct Session {
    std::string username;
    std::chrono::steady_clock::time_point expires;
  };
  std::unordered_map<std::string, Session> sessions_;
};
