#include <cctype>
#include <filesystem>
#include <iostream>
#include <string>

#include "config.hpp"

namespace {

int failures = 0;

void check(bool ok, const char *name) {
  if (!ok) {
    std::cerr << "FAIL " << name << "\n";
    ++failures;
  }
}

}  // namespace

int main() {
  namespace fs = std::filesystem;

  {
    std::string h = hash_password("secret");
    check(verify_password("secret", h), "verify_correct");
    check(!verify_password("other", h), "verify_wrong");
    check(h.rfind("pbkdf2-sha256$100000$", 0) == 0, "hash_format");
    check(!verify_password("secret", "garbage"), "verify_malformed");
  }

  {
    std::string a(64, 'a');
    std::string b = a;
    b.back() = 'b';
    check(token_equal(a, a), "token_equal_same");
    check(!token_equal(a, b), "token_equal_last_differs");
    check(!token_equal(a.substr(0, 63), a), "token_equal_shorter");
    check(!token_equal(a + "a", a), "token_equal_longer");
  }

  {
    std::string t = make_token();
    bool hex = t.size() == 64;
    for (char c : t) hex = hex && std::isxdigit(static_cast<unsigned char>(c));
    check(hex, "make_token_hex64");
    check(make_token() != t, "make_token_random");
  }

  {
    Config c = make_config("ada", "secret");
    check(c.name == "ada" && c.username == "ada", "make_config_names");
    check(verify_password("secret", c.password_hash), "make_config_hash");
    check(c.token.size() == 64, "make_config_token");
  }

  {
    fs::path dir = fs::temp_directory_path() / ("liveuse_config_test_" + make_token().substr(0, 8));
    fs::path path = dir / "config.json";
    Config c{"n", "u", "h", "t"};
    save_config(path, c);
    auto r = load_config(path);
    check(r.has_value() && r->name == "n" && r->username == "u" && r->password_hash == "h" &&
              r->token == "t",
          "roundtrip");
    auto p = fs::status(path).permissions();
    check((p & fs::perms::owner_read) != fs::perms::none, "perm_owner_read");
    check((p & fs::perms::owner_write) != fs::perms::none, "perm_owner_write");
    check((p & fs::perms::group_read) == fs::perms::none, "perm_group_read");
    check((p & fs::perms::others_read) == fs::perms::none, "perm_others_read");
    fs::remove_all(dir);
    check(!load_config(path).has_value(), "load_missing");
  }

  if (failures) return 1;
  std::cout << "ok\n";
  return 0;
}
