#include "link.hpp"

#include <cerrno>
#include <dirent.h>
#include <fcntl.h>
#include <poll.h>
#include <termios.h>
#include <unistd.h>

#include <chrono>
#include <cstring>
#include <string>

namespace {

bool terminal_line(const std::string &line) {
  if (line.rfind("ERR", 0) == 0) return true;
  if (line.size() >= 5 && line.ends_with(" done")) return true;
  if (line.rfind("OK ", 0) == 0) return true;
  if (line.rfind("ECHO", 0) == 0) return true;
  return false;
}

class MacLink final : public CdcLink {
 public:
  ~MacLink() override {
    if (fd_ >= 0) ::close(fd_);
  }

  std::vector<PortInfo> list_ports() override {
    std::vector<PortInfo> ports;
    DIR *dir = opendir("/dev");
    if (dir == nullptr) return ports;
    while (dirent *entry = readdir(dir)) {
      const std::string name = entry->d_name;
      if (name.rfind("cu.", 0) != 0) continue;
      std::string lower = name;
      for (char &c : lower) {
        if (c >= 'A' && c <= 'Z') c = static_cast<char>(c - 'A' + 'a');
      }
      ports.push_back(PortInfo{"/dev/" + name, lower.find("usb") != std::string::npos ? "usb" : "other"});
    }
    closedir(dir);
    return ports;
  }

  std::expected<std::vector<std::string>, CdcError> connect(const std::string &port) override {
    if (fd_ >= 0) ::close(fd_);
    fd_ = -1;
    port_.clear();
    pending_.clear();
    fd_ = ::open(port.c_str(), O_RDWR | O_NOCTTY | O_NONBLOCK);
    if (fd_ < 0) return std::unexpected(CdcError{409, "could not open port"});
    termios tio{};
    if (tcgetattr(fd_, &tio) != 0) {
      ::close(fd_);
      fd_ = -1;
      return std::unexpected(CdcError{409, "could not configure port"});
    }
    cfmakeraw(&tio);
    cfsetspeed(&tio, B115200);
    tio.c_cflag |= CLOCAL | CREAD | CS8;
    tio.c_cflag &= static_cast<unsigned long>(~(PARENB | CSTOPB));
    tio.c_cc[VMIN] = 0;
    tio.c_cc[VTIME] = 0;
    if (tcsetattr(fd_, TCSANOW, &tio) != 0) {
      ::close(fd_);
      fd_ = -1;
      return std::unexpected(CdcError{409, "could not configure port"});
    }
    port_ = port;
    return read_for(300, false);
  }

  std::expected<std::string, CdcError> disconnect() override {
    const std::string previous = port_;
    if (fd_ >= 0) ::close(fd_);
    fd_ = -1;
    port_.clear();
    pending_.clear();
    return previous;
  }

  bool connected() const override { return fd_ >= 0; }
  std::string port() const override { return port_; }

  std::expected<std::vector<std::string>, CdcError> write_lines(const std::vector<std::string> &lines,
                                                                bool wait_done) override {
    if (fd_ < 0) return std::unexpected(CdcError{409, "serial closed"});
    std::vector<std::string> all;
    if (!wait_done) {
      for (const std::string &line : lines) {
        if (auto err = write_line(line); !err) return std::unexpected(err.error());
      }
      auto got = read_for(80, false);
      if (!got) return std::unexpected(got.error());
      return *got;
    }
    for (const std::string &line : lines) {
      if (auto err = write_line(line); !err) return std::unexpected(err.error());
      const bool help = line == "HELP" || line == "?";
      auto got = help ? read_for(5000, false) : read_for(20000, true);
      if (!got) return std::unexpected(got.error());
      all.insert(all.end(), got->begin(), got->end());
    }
    return all;
  }

 private:
  std::expected<void, CdcError> write_line(const std::string &line) {
    const std::string data = line + "\n";
    size_t sent = 0;
    while (sent < data.size()) {
      const ssize_t n = ::write(fd_, data.data() + sent, data.size() - sent);
      if (n < 0) {
        if (errno == EAGAIN) continue;
        return std::unexpected(CdcError{502, "serial write failed"});
      }
      sent += static_cast<size_t>(n);
    }
    return {};
  }

  std::expected<std::vector<std::string>, CdcError> read_for(int timeout_ms, bool until_terminal) {
    std::vector<std::string> lines;
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(timeout_ms);
    auto last_data = std::chrono::steady_clock::now();
    while (std::chrono::steady_clock::now() < deadline) {
      while (true) {
        const auto pos = pending_.find('\n');
        if (pos == std::string::npos) break;
        std::string line = pending_.substr(0, pos);
        pending_.erase(0, pos + 1);
        if (!line.empty() && line.back() == '\r') line.pop_back();
        lines.push_back(line);
        last_data = std::chrono::steady_clock::now();
        if (until_terminal && terminal_line(line)) return lines;
      }
      if (!until_terminal && !lines.empty()) {
        const auto idle = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - last_data);
        if (idle.count() >= (timeout_ms > 200 ? 200 : timeout_ms)) return lines;
      }
      const auto left = std::chrono::duration_cast<std::chrono::milliseconds>(deadline - std::chrono::steady_clock::now());
      if (left.count() <= 0) break;
      pollfd pfd{.fd = fd_, .events = POLLIN, .revents = 0};
      const int wait = static_cast<int>(std::min<std::chrono::milliseconds::rep>(left.count(), 50));
      const int rc = ::poll(&pfd, 1, wait);
      if (rc < 0) return std::unexpected(CdcError{502, "serial read failed"});
      if (rc == 0) continue;
      char buf[256];
      const ssize_t n = ::read(fd_, buf, sizeof(buf));
      if (n > 0) {
        pending_.append(buf, buf + n);
        last_data = std::chrono::steady_clock::now();
      }
    }
    if (until_terminal) return std::unexpected(CdcError{502, "serial timeout"});
    return lines;
  }

  int fd_ = -1;
  std::string port_;
  std::string pending_;
};

}  // namespace

std::unique_ptr<CdcLink> make_serial_link() { return std::make_unique<MacLink>(); }
