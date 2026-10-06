#include "http_app.hpp"

#include <httplib.h>

#include <arpa/inet.h>
#include <ifaddrs.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <termios.h>
#include <unistd.h>

#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <iostream>
#include <mach-o/dyld.h>
#include <thread>

namespace {

std::string read_secret() {
  termios saved{};
  const bool tty = tcgetattr(STDIN_FILENO, &saved) == 0;
  if (tty) {
    termios hidden = saved;
    hidden.c_lflag &= ~static_cast<unsigned long>(ECHO);
    tcsetattr(STDIN_FILENO, TCSANOW, &hidden);
  }
  std::string password;
  if (!std::getline(std::cin, password)) password.clear();
  if (tty) {
    tcsetattr(STDIN_FILENO, TCSANOW, &saved);
    std::cout << "\n";
  }
  return password;
}

void ask_reload() {
  const int fd = ::socket(AF_UNIX, SOCK_STREAM, 0);
  if (fd < 0) return;
  sockaddr_un addr{};
  addr.sun_family = AF_UNIX;
  const std::string path = socket_path().string();
  if (path.size() >= sizeof(addr.sun_path)) {
    ::close(fd);
    return;
  }
  std::strncpy(addr.sun_path, path.c_str(), sizeof(addr.sun_path) - 1);
  if (::connect(fd, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) == 0) {
    const char message[] = "reload\n";
    if (::write(fd, message, sizeof(message) - 1) > 0) {
      char buf[16] = {};
      ::read(fd, buf, sizeof(buf));
    }
  }
  ::close(fd);
}

bool write_config(const Config &config) {
  try {
    save_config(config_path(), config);
  } catch (const std::exception &ex) {
    std::cerr << ex.what() << "\n";
    return false;
  }
  ask_reload();
  return true;
}

std::string lan_ip() {
  ifaddrs *list = nullptr;
  if (getifaddrs(&list) != 0) return "127.0.0.1";
  std::string found = "127.0.0.1";
  for (ifaddrs *item = list; item != nullptr; item = item->ifa_next) {
    if (item->ifa_addr == nullptr || item->ifa_addr->sa_family != AF_INET) continue;
    auto *addr = reinterpret_cast<sockaddr_in *>(item->ifa_addr);
    char text[INET_ADDRSTRLEN] = {};
    if (inet_ntop(AF_INET, &addr->sin_addr, text, sizeof(text)) == nullptr) continue;
    if (std::strcmp(text, "127.0.0.1") == 0) continue;
    found = text;
    break;
  }
  freeifaddrs(list);
  return found;
}

std::string web_root() {
  if (const char *env = std::getenv("LIVEUSE_WEB_ROOT")) return env;
  char buf[4096];
  uint32_t size = sizeof(buf);
  if (_NSGetExecutablePath(buf, &size) != 0) return "web";
  return (std::filesystem::path(buf).parent_path() / "web").string();
}

void serve_reload(HttpApp &app) {
  const std::filesystem::path path = socket_path();
  std::error_code ignore;
  std::filesystem::create_directories(path.parent_path(), ignore);
  ::unlink(path.c_str());
  const int fd = ::socket(AF_UNIX, SOCK_STREAM, 0);
  if (fd < 0) return;
  sockaddr_un addr{};
  addr.sun_family = AF_UNIX;
  const std::string text = path.string();
  if (text.size() >= sizeof(addr.sun_path)) {
    ::close(fd);
    return;
  }
  std::strncpy(addr.sun_path, text.c_str(), sizeof(addr.sun_path) - 1);
  if (::bind(fd, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) != 0) {
    ::close(fd);
    return;
  }
  ::listen(fd, 4);
  while (true) {
    const int client = ::accept(fd, nullptr, nullptr);
    if (client < 0) continue;
    char buf[32] = {};
    const ssize_t n = ::read(client, buf, sizeof(buf) - 1);
    if (n > 0 && std::string(buf, buf + n).rfind("reload", 0) == 0) {
      auto loaded = load_config(config_path());
      if (loaded) app.reload(*loaded);
      const char ok[] = "ok\n";
      ::write(client, ok, sizeof(ok) - 1);
    }
    ::close(client);
  }
}

int serve() {
  auto loaded = load_config(config_path());
  if (!loaded) {
    std::cerr << "run liveuse init\n";
    return 1;
  }
  auto link = make_serial_link();
  auto screen = make_screen_capture();
  auto device = only_device_port(link->list_ports());
  if (!device) {
    std::cerr << device.error().message << "\n";
  } else if (auto opened = link->connect(*device); !opened) {
    std::cerr << opened.error().message << "\n";
  } else {
    std::cerr << "device " << link->port() << "\n";
  }
  HttpApp app(*loaded, *link, *screen);
  httplib::Server server;
  app.attach(server, web_root());
  if (!server.bind_to_port("0.0.0.0", 7395)) {
    std::cerr << "port 7395 is in use\n";
    return 1;
  }
  app.set_public_base("http://" + lan_ip() + ":7395");
  std::thread reload(serve_reload, std::ref(app));
  reload.detach();
  if (!server.listen_after_bind()) {
    std::cerr << "could not listen\n";
    return 1;
  }
  return 0;
}

}  // namespace

int main(int argc, char **argv) {
  if (argc < 2) return serve();
  const std::string cmd = argv[1];
  if (cmd == "init") {
    std::string user;
    bool force = false;
    for (int i = 2; i < argc; ++i) {
      const std::string arg = argv[i];
      if (arg == "--force") force = true;
      else if (arg == "--user" && i + 1 < argc) user = argv[++i];
      else {
        std::cerr << "usage: liveuse init --user <name> [--force]\n";
        return 1;
      }
    }
    if (user.empty()) {
      std::cerr << "usage: liveuse init --user <name> [--force]\n";
      return 1;
    }
    if (std::filesystem::exists(config_path()) && !force) {
      std::cerr << "config already exists\n";
      return 1;
    }
    std::cout << "password: " << std::flush;
    const std::string password = read_secret();
    if (password.empty()) {
      std::cerr << "empty password\n";
      return 1;
    }
    return write_config(make_config(user, password)) ? 0 : 1;
  }
  auto loaded = load_config(config_path());
  if (!loaded) {
    std::cerr << "run liveuse init\n";
    return 1;
  }
  if (cmd == "token" && argc == 3 && std::strcmp(argv[2], "rotate") == 0) {
    loaded->token = make_token();
    return write_config(*loaded) ? 0 : 1;
  }
  if (cmd == "user" && argc == 4 && std::strcmp(argv[2], "set") == 0) {
    loaded->username = argv[3];
    return write_config(*loaded) ? 0 : 1;
  }
  if (cmd == "password" && argc == 3 && std::strcmp(argv[2], "set") == 0) {
    std::cout << "password: " << std::flush;
    const std::string password = read_secret();
    if (password.empty()) {
      std::cerr << "empty password\n";
      return 1;
    }
    loaded->password_hash = hash_password(password);
    return write_config(*loaded) ? 0 : 1;
  }
  std::cerr << "usage: liveuse [init|token rotate|user set|password set]\n";
  return 1;
}
