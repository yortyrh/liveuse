#pragma once

#include <expected>
#include <memory>
#include <string>
#include <vector>

#include "cdc.hpp"

struct PortInfo {
  std::string name;
  std::string kind;
};

inline bool usb_device_name(const std::string &name) {
  std::string lower = name;
  for (char &c : lower) {
    if (c >= 'A' && c <= 'Z') c = static_cast<char>(c - 'A' + 'a');
  }
  return lower.find("usbmodem") != std::string::npos || lower.find("usbserial") != std::string::npos;
}

inline std::expected<std::string, CdcError> only_device_port(const std::vector<PortInfo> &ports) {
  std::vector<std::string> found;
  for (const PortInfo &port : ports) {
    if (usb_device_name(port.name)) found.push_back(port.name);
  }
  if (found.empty()) return std::unexpected(CdcError{404, "device not found"});
  if (found.size() != 1) return std::unexpected(CdcError{409, "more than one device"});
  return found[0];
}

class CdcLink {
 public:
  virtual ~CdcLink() = default;
  virtual std::vector<PortInfo> list_ports() = 0;
  virtual std::expected<std::vector<std::string>, CdcError> connect(const std::string &port) = 0;
  virtual std::expected<std::string, CdcError> disconnect() = 0;
  virtual bool connected() const = 0;
  virtual std::string port() const = 0;
  virtual std::expected<std::vector<std::string>, CdcError> write_lines(const std::vector<std::string> &lines,
                                                                        bool wait_done) = 0;
};

std::unique_ptr<CdcLink> make_serial_link();
