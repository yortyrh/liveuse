#pragma once

#include <expected>
#include <filesystem>
#include <string>
#include <string_view>

struct Config {
  std::string name;
  std::string username;
  std::string password_hash;
  std::string token;
};

std::filesystem::path runtime_dir();  // ~/Library/Application Support/liveuse
std::filesystem::path config_path();  // runtime_dir()/config.json
std::filesystem::path socket_path();  // runtime_dir()/liveuse.sock
std::expected<Config, std::string> load_config(const std::filesystem::path &path);
void save_config(const std::filesystem::path &path, const Config &cfg);
std::string hash_password(std::string_view password);
bool verify_password(std::string_view password, std::string_view stored);
bool token_equal(std::string_view given, std::string_view expected);
std::string make_token();  // 64 hex chars
Config make_config(std::string username, std::string password);
