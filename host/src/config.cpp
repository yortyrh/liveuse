#include "config.hpp"

#include <CommonCrypto/CommonCryptor.h>
#include <CommonCrypto/CommonKeyDerivation.h>
#include <Security/SecRandom.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include <array>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <nlohmann/json.hpp>
#include <stdexcept>
#include <system_error>
#include <vector>

namespace {

constexpr uint32_t kIterations = 100000;
constexpr size_t kSaltLen = 16;
constexpr size_t kHashLen = 32;
constexpr std::string_view kPrefix = "pbkdf2-sha256";

constexpr char kB64[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

std::string b64_encode(const uint8_t *data, size_t n) {
  std::string out;
  out.reserve((n + 2) / 3 * 4);
  for (size_t i = 0; i < n; i += 3) {
    uint32_t v = static_cast<uint32_t>(data[i]) << 16;
    if (i + 1 < n) v |= static_cast<uint32_t>(data[i + 1]) << 8;
    if (i + 2 < n) v |= data[i + 2];
    out.push_back(kB64[(v >> 18) & 63]);
    out.push_back(kB64[(v >> 12) & 63]);
    out.push_back(i + 1 < n ? kB64[(v >> 6) & 63] : '=');
    out.push_back(i + 2 < n ? kB64[v & 63] : '=');
  }
  return out;
}

bool b64_decode(std::string_view s, std::vector<uint8_t> &out) {
  out.clear();
  if (s.size() % 4 != 0) return false;
  auto val = [](char c) -> int {
    if (c >= 'A' && c <= 'Z') return c - 'A';
    if (c >= 'a' && c <= 'z') return c - 'a' + 26;
    if (c >= '0' && c <= '9') return c - '0' + 52;
    if (c == '+') return 62;
    if (c == '/') return 63;
    return -1;
  };
  for (size_t i = 0; i < s.size(); i += 4) {
    int pad = 0;
    uint32_t v = 0;
    for (size_t j = 0; j < 4; ++j) {
      char c = s[i + j];
      if (c == '=') {
        if (i + 4 != s.size() || j < 2) return false;
        ++pad;
        v <<= 6;
      } else {
        if (pad) return false;
        int d = val(c);
        if (d < 0) return false;
        v = (v << 6) | static_cast<uint32_t>(d);
      }
    }
    out.push_back(static_cast<uint8_t>(v >> 16));
    if (pad < 2) out.push_back(static_cast<uint8_t>(v >> 8));
    if (pad < 1) out.push_back(static_cast<uint8_t>(v));
  }
  return true;
}

bool derive(std::string_view password, const uint8_t *salt, size_t salt_len, uint32_t iterations,
            uint8_t *out) {
  return CCKeyDerivationPBKDF(kCCPBKDF2, password.data(), password.size(), salt, salt_len,
                              kCCPRFHmacAlgSHA256, iterations, out, kHashLen) == kCCSuccess;
}

void random_bytes(uint8_t *buf, size_t n) {
  if (SecRandomCopyBytes(kSecRandomDefault, n, buf) != errSecSuccess)
    throw std::runtime_error("SecRandomCopyBytes failed");
}

}  // namespace

std::filesystem::path runtime_dir() {
  const char *home = std::getenv("HOME");
  return std::filesystem::path(home ? home : "") / "Library" / "Application Support" / "liveuse";
}

std::filesystem::path config_path() { return runtime_dir() / "config.json"; }

std::filesystem::path socket_path() { return runtime_dir() / "liveuse.sock"; }

std::expected<Config, std::string> load_config(const std::filesystem::path &path) {
  std::ifstream in(path);
  if (!in) return std::unexpected("cannot open " + path.string());
  nlohmann::json j = nlohmann::json::parse(in, nullptr, false);
  if (j.is_discarded() || !j.is_object()) return std::unexpected("invalid config json");
  Config c;
  std::string *fields[] = {&c.name, &c.username, &c.password_hash, &c.token};
  const char *keys[] = {"name", "username", "password_hash", "token"};
  for (int i = 0; i < 4; ++i) {
    auto it = j.find(keys[i]);
    if (it == j.end() || !it->is_string())
      return std::unexpected(std::string("missing field: ") + keys[i]);
    *fields[i] = it->get<std::string>();
  }
  return c;
}

void save_config(const std::filesystem::path &path, const Config &cfg) {
  namespace fs = std::filesystem;
  fs::path parent = path.parent_path();
  if (!parent.empty() && !fs::exists(parent)) {
    std::error_code ec;
    fs::create_directories(parent, ec);
    if (ec) throw std::runtime_error("cannot create " + parent.string() + ": " + ec.message());
    if (::chmod(parent.c_str(), 0700) != 0)
      throw std::runtime_error("cannot set directory mode");
  }
  nlohmann::json j = {{"name", cfg.name},
                      {"username", cfg.username},
                      {"password_hash", cfg.password_hash},
                      {"token", cfg.token}};
  std::string body = j.dump(2) + "\n";
  int fd = ::open(path.c_str(), O_WRONLY | O_CREAT | O_TRUNC, 0600);
  if (fd < 0) throw std::runtime_error("cannot open " + path.string());
  if (::fchmod(fd, 0600) != 0) {
    ::close(fd);
    throw std::runtime_error("cannot set file mode");
  }
  size_t off = 0;
  while (off < body.size()) {
    ssize_t w = ::write(fd, body.data() + off, body.size() - off);
    if (w <= 0) {
      ::close(fd);
      throw std::runtime_error("cannot write " + path.string());
    }
    off += static_cast<size_t>(w);
  }
  ::close(fd);
}

std::string hash_password(std::string_view password) {
  std::array<uint8_t, kSaltLen> salt;
  random_bytes(salt.data(), salt.size());
  std::array<uint8_t, kHashLen> out;
  if (!derive(password, salt.data(), salt.size(), kIterations, out.data()))
    throw std::runtime_error("pbkdf2 failed");
  return std::string(kPrefix) + "$" + std::to_string(kIterations) + "$" +
         b64_encode(salt.data(), salt.size()) + "$" + b64_encode(out.data(), out.size());
}

bool verify_password(std::string_view password, std::string_view stored) {
  std::array<std::string_view, 4> parts;
  size_t start = 0;
  for (size_t i = 0; i < 4; ++i) {
    size_t end = stored.find('$', start);
    if (i < 3) {
      if (end == std::string_view::npos) return false;
      parts[i] = stored.substr(start, end - start);
      start = end + 1;
    } else {
      if (end != std::string_view::npos) return false;
      parts[i] = stored.substr(start);
    }
  }
  if (parts[0] != kPrefix) return false;
  uint32_t iters = 0;
  if (parts[1].empty() || parts[1].size() > 9) return false;
  for (char c : parts[1]) {
    if (c < '0' || c > '9') return false;
    iters = iters * 10 + static_cast<uint32_t>(c - '0');
  }
  if (iters == 0) return false;
  std::vector<uint8_t> salt, expected;
  if (!b64_decode(parts[2], salt) || salt.empty()) return false;
  if (!b64_decode(parts[3], expected) || expected.size() != kHashLen) return false;
  std::array<uint8_t, kHashLen> got;
  if (!derive(password, salt.data(), salt.size(), iters, got.data())) return false;
  uint8_t acc = 0;
  for (size_t i = 0; i < kHashLen; ++i) acc |= static_cast<uint8_t>(got[i] ^ expected[i]);
  return acc == 0;
}

bool token_equal(std::string_view given, std::string_view expected) {
  uint8_t acc = given.size() != expected.size() ? 1 : 0;
  for (size_t i = 0; i < expected.size(); ++i) {
    uint8_t g = i < given.size() ? static_cast<uint8_t>(given[i]) : 0;
    acc |= static_cast<uint8_t>(g ^ static_cast<uint8_t>(expected[i]));
  }
  return acc == 0;
}

std::string make_token() {
  std::array<uint8_t, 32> buf;
  random_bytes(buf.data(), buf.size());
  static constexpr char hex[] = "0123456789abcdef";
  std::string out;
  out.reserve(64);
  for (uint8_t b : buf) {
    out.push_back(hex[b >> 4]);
    out.push_back(hex[b & 15]);
  }
  return out;
}

Config make_config(std::string username, std::string password) {
  Config c;
  c.name = username;
  c.username = std::move(username);
  c.password_hash = hash_password(password);
  c.token = make_token();
  return c;
}
