# Live Use Host Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One C++23 process, `liveuse`, serves MCP on `/mcp`, the REST API on `/api`, and later the SPA files, and it owns the CDC port and the screenshot.

The control surface is the PyAutoGUI contract in `docs/superpowers/specs/2026-10-05-lan-hid-control-design.md` (mouse, keyboard, screenshot). Where this plan names an older tool (`mouse_move` only, `keyboard_type`, `capture_screen`), the spec wins.

**Architecture:** `firmware/src/keymap.cpp` is the only chord and US ANSI table. `Device` talks to a `SerialLink` and a `ScreenCapture` behind a mutex, one CDC transaction at a time. `cpp-httplib` owns port 7395. Bearer tokens open `/mcp`. A cookie opens `/api`. The CLI is the same binary and reloads config through `liveuse.sock`.

**Tech Stack:** C++23, CMake, cpp-httplib 0.18.3, nlohmann/json 3.11.3, stb_image_write, CommonCrypto PBKDF2, CoreGraphics, termios, Unix sockets. No MCP SDK.

## Global Constraints

- `CMAKE_CXX_STANDARD` is 23 and `CMAKE_CXX_STANDARD_REQUIRED` is ON.
- Binary name is `liveuse`. `liveuse` with no arguments listens. `init`, `token`, `user`, and `password` do not listen.
- Bind `0.0.0.0:7395`. If the port is taken, exit. No second port. No CORS headers. No TLS.
- Runtime directory is `~/Library/Application Support/liveuse/`. Socket is `liveuse.sock` in that directory. Config file is `config.json` mode `0600`.
- Config fields are `name`, `username`, `password_hash`, `token`. Hash text is `pbkdf2-sha256$100000$<salt_b64>$<hash_b64>` (PBKDF2-HMAC-SHA256, 100000 iterations, 16-byte salt, 32-byte output). Token is 32 random bytes, 64 hex chars.
- No config file: do not listen; tell the user to run `init`. No default user. `init` does not overwrite unless `--force`. `init` sets `name` equal to the username. Password comes from stdin, never from an argument.
- Cookie name `liveuse_session`, `Path=/`, `HttpOnly`, `SameSite=Lax`, not `Secure`, 12-hour memory lifetime. Login failure body is `{"error":"credenciales invalidas"}` for a bad user and a bad password. `reload` swaps config and drops every cookie. It does not close the serial port.
- `/mcp` requires `Authorization: Bearer <token>` compared in constant time. Wrong or missing token is HTTP 401 and is not a JSON-RPC body. A device failure after a good token is HTTP 200 with `isError: true`.
- Client `Accept` may list `application/json` and `text/event-stream`. The response is one JSON object, `Content-Type: application/json`. No SSE and no `Mcp-Session-Id`.
- `serverInfo.name` is `liveuse`. Instructions include the device name from config, say to connect before mouse or keyboard, and say `capture_screen` does not use serial.
- Mouse and keyboard with the port closed return HTTP 409 and write nothing to CDC. Bad arguments return 400. Firmware `ERR TYPE bad` or `ERR KEY bad` returns 502. `TYPE` has no success line and no `completed` field. Screenshot failure returns 503 and no PNG body.
- `keyboard_type` is printable ASCII, no CR/LF, length 1–2000. The host splits it into `TYPE ` lines of at most 75 characters. `MOVE` and `SCROLL` are integers in ±2000.
- REST success bodies use `"lines"` as a string array, except screenshot, which is `image/png` with `X-Image-Width` and `X-Image-Height`.
- Screenshot: `ScreenCapture::capture_primary()` returns BGRA. `encode_png` is shared. Non-Apple builds link `capture_stub.cpp` and return exactly `screen capture is not implemented on this OS`. This plan implements the macOS backend only. Primary display only.
- Static files come from `web/` next to the binary, or `LIVEUSE_WEB_ROOT`. If that directory is missing, `GET /` returns a short text naming the path. `/api` and `/mcp` still work.
- The advertised MCP URL uses the first non-loopback IPv4, else `127.0.0.1`: `http://<ip>:7395/mcp`.
- Do not log the token, the password, or the hash.
- Do not rename the repo directory. Do not change the USB product string.

## File map

| File | Responsibility |
| --- | --- |
| `host/CMakeLists.txt` | C++23 binary `liveuse` and CTest targets |
| `host/src/cdc.hpp`, `host/src/cdc.cpp` | Chunk `TYPE`, format `KEY` and the mouse lines |
| `host/src/config.hpp`, `host/src/config.cpp` | Load, save, hash, token compare, paths |
| `host/src/device.hpp`, `host/src/device.cpp` | Serial mutex, mouse, keyboard, screenshot |
| `host/src/capture.hpp` | `ScreenCapture`, `PixelBuffer` |
| `host/src/png.cpp` | `encode_png` |
| `host/src/capture_macos.mm` | CoreGraphics primary display |
| `host/src/capture_stub.cpp` | Exact not-implemented error |
| `host/src/serial_macos.cpp` | termios CDC and `/dev/cu.*` |
| `host/src/http.cpp` | Routes for `/mcp`, `/api`, `/` |
| `host/src/main.cpp` | CLI and `liveuse.sock` reload |
| `host/tests/*.cpp` | CTest drivers |

`SerialLink` and `ScreenCapture` are abstract. Tests pass fakes. `main.cpp` passes the macOS types.

---

### Task 1: CDC line formatting

**Files:**
- Create: `host/CMakeLists.txt`
- Create: `host/src/cdc.hpp`
- Create: `host/src/cdc.cpp`
- Test: `host/tests/cdc_test.cpp`

**Interfaces:**
- Consumes: `parse_chord` and `tap_ascii` from `firmware/include/keymap.h`
- Produces:

```cpp
struct CdcError { int status; std::string message; };
std::expected<std::vector<std::string>, CdcError> format_type(std::string_view text);
std::expected<std::string, CdcError> format_key(std::string_view chord);
std::expected<std::string, CdcError> format_move(int dx, int dy);
std::expected<std::string, CdcError> format_click(std::string_view button);
std::expected<std::string, CdcError> format_scroll(int steps);
std::expected<std::string, CdcError> format_demo(std::string_view pattern);
std::expected<std::string, CdcError> format_echo(std::string_view text);
std::expected<std::string, CdcError> format_raw(std::string_view line);
```

`format_type` returns lines `TYPE <chunk>` with at most 75 payload characters. `format_key` returns `KEY <canonical>` using `parse_chord`. Range errors use `status` 400.

- [ ] **Step 1: Write the failing test**

`host/tests/cdc_test.cpp` asserts all of the following, prints `FAIL <name>` on stderr, and returns 1 if any fail:

- `format_type("hi")` equals `{"TYPE hi"}`.
- `format_type("")` is an error with status 400.
- `format_type("a\nb")` is status 400.
- `format_type` of a 76-character string of `a` returns two lines, lengths 80 and 6 (`TYPE ` + 75, then `TYPE ` + 1).
- `format_type` of 2001 `a`s is status 400.
- `format_type("@")` is `{"TYPE @"}` and `tap_ascii('@')` has modifier `0x02` and usage `0x1F`.
- `format_key("shift+cmd+4")` equals `KEY cmd+shift+4`.
- `format_key("A")` equals `KEY a`.
- `format_key("cmd")` is status 400.
- `format_move(10, -4)` equals `MOVE 10 -4`. `format_move(2001, 0)` is status 400.
- `format_click("Right")` equals `CLICK right`. `format_click("middle")` is status 400.
- `format_scroll(-3)` equals `SCROLL -3`.
- `format_demo("Circle")` equals `DEMO circle`.
- `format_echo("a\nb")` is status 400.
- `format_raw("HELP")` equals `HELP`. `format_raw` of an 81-character line is status 400.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cmake -S host -B host/build && cmake --build host/build && ctest --test-dir host/build --output-on-failure -R cdc`

Expected: cmake or link fails because `cdc.cpp` does not exist yet. After the CMake file exists and the test target is the only source, the test binary fails to link `format_type`.

- [ ] **Step 3: Implement formatting and the CMake root**

`host/CMakeLists.txt` sets C++23 required, `project(liveuse VERSION 0.3.0 LANGUAGES CXX)`, FetchContent for cpp-httplib `v0.18.3` and nlohmann/json `v3.11.3`, and FetchContent for `https://github.com/nothings/stb.git` at tag `f7f20f39fe4f206c6f19e26ebf2b5ca4a65ab763`. It builds `keymap` from `../firmware/src/keymap.cpp` with include `../firmware/include`. Test executable `cdc_test` links `cdc.cpp` and `keymap`. `enable_testing()` and `add_test(NAME cdc COMMAND cdc_test)`.

`format_type` rejects empty, size above 2000, CR, LF, and any byte outside `0x20`–`0x7E` by calling `tap_ascii` per character. It slices the accepted string every 75 characters.

`format_key` copies `parse_chord(...).canonical` and prefixes `KEY `.

`format_move`, `format_scroll` reject values outside ±2000. `format_click` lowercases and accepts `left`, `right`, `double`. `format_demo` lowercases and accepts `circle`, `zigzag`, `both`. `format_echo` and `format_raw` reject CR, LF, empty, and size above 80. `format_echo` prefixes `ECHO `.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure -R cdc`

Expected: `100% tests passed`.

- [ ] **Step 5: Commit**

```bash
git add host/CMakeLists.txt host/src/cdc.hpp host/src/cdc.cpp host/tests/cdc_test.cpp
git commit -m "$(cat <<'EOF'
feat: format TYPE and KEY lines for the CDC port

Chunks stay inside the 80-byte firmware line and the chord text is canonical.

EOF
)"
```

---

### Task 2: Config file, password hash, token

**Files:**
- Create: `host/src/config.hpp`
- Create: `host/src/config.cpp`
- Test: `host/tests/config_test.cpp`
- Modify: `host/CMakeLists.txt` (add `config_test`, link CoreFoundation and Security)

**Interfaces:**
- Consumes: nothing from Task 1
- Produces:

```cpp
struct Config {
  std::string name;
  std::string username;
  std::string password_hash;
  std::string token;
};
std::filesystem::path runtime_dir();      // ~/Library/Application Support/liveuse
std::filesystem::path config_path();      // runtime_dir()/config.json
std::filesystem::path socket_path();      // runtime_dir()/liveuse.sock
std::expected<Config, std::string> load_config(const std::filesystem::path& path);
void save_config(const std::filesystem::path& path, const Config& cfg);
std::string hash_password(std::string_view password);
bool verify_password(std::string_view password, std::string_view stored);
bool token_equal(std::string_view given, std::string_view expected);
std::string make_token();                 // 64 hex chars
Config make_config(std::string username, std::string password);
```

- [ ] **Step 1: Write the failing test**

`config_test` uses a temp directory and checks:

- `verify_password("secret", hash_password("secret"))` is true.
- `verify_password("other", hash_password("secret"))` is false.
- `token_equal` is true for equal 64-hex strings and false when the last character differs. It is false when the sizes differ.
- `make_token().size()` is 64 and every character is hex.
- `make_config("ada", "secret")` has `name == "ada"` and `username == "ada"`.
- `save_config` then `load_config` round-trips the four fields.
- After save, `std::filesystem::status(path).permissions()` contains owner read and write and does not contain group read or others read.
- `load_config` of a missing path returns an error.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure -R config`

Expected: link error for `hash_password`.

- [ ] **Step 3: Implement config**

`hash_password` draws 16 salt bytes with `SecRandomCopyBytes`, calls `CCKeyDerivationPBKDF` with `kCCPBKDF2`, `kCCPRFHmacAlgSHA256`, 100000 iterations, 32-byte output, and writes the `pbkdf2-sha256$100000$...` string. Base64 is the standard alphabet with padding.

`verify_password` parses that shape, decodes salt and hash, derives again, and compares the 32 bytes with XOR accumulation so the loop always runs 32 times.

`token_equal` walks `expected.size()` bytes, using 0 when `given` is shorter, and also folds in the size mismatch. It returns true only when the accumulator is 0.

`save_config` writes JSON with nlohmann (`name`, `username`, `password_hash`, `token`) via `open(path, O_WRONLY|O_CREAT|O_TRUNC, 0600)`. `load_config` reads those four strings and errors if any is missing.

`runtime_dir` joins `HOME` with `Library/Application Support/liveuse`. `save_config` creates that directory only when the path's parent is missing, with mode `0700`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure -R config`

Expected: `100% tests passed`.

- [ ] **Step 5: Commit**

```bash
git add host/src/config.hpp host/src/config.cpp host/tests/config_test.cpp host/CMakeLists.txt
git commit -m "$(cat <<'EOF'
feat: store liveuse credentials as a private config file

The password is a PBKDF2 hash and the token compare does not stop at the first mismatch.

EOF
)"
```

---

### Task 3: Device operations on a fake serial port

**Files:**
- Create: `host/src/capture.hpp`
- Create: `host/src/device.hpp`
- Create: `host/src/device.cpp`
- Test: `host/tests/device_test.cpp`
- Modify: `host/CMakeLists.txt`

**Interfaces:**
- Consumes: `format_type`, `format_key`, `format_move`, `format_click`, `format_scroll`, `format_demo`, `format_echo`, `format_raw`
- Produces:

```cpp
struct PortInfo { std::string name; std::string kind; };
struct PixelBuffer {
  bool ok = false;
  std::string error;
  std::vector<uint8_t> bgra;
  uint32_t width = 0;
  uint32_t height = 0;
  uint32_t stride = 0;
};
class ScreenCapture {
 public:
  virtual ~ScreenCapture() = default;
  virtual PixelBuffer capture_primary() = 0;
};
class SerialLink {
 public:
  virtual ~SerialLink() = default;
  virtual std::vector<PortInfo> list() = 0;
  virtual void open_port(const std::string& path) = 0;
  virtual void close_port() = 0;
  virtual bool is_open() const = 0;
  virtual std::string path() const = 0;
  virtual std::vector<std::string> transact(const std::string& line) = 0;
  virtual std::vector<std::string> read_banner() = 0;
};
struct OpResult {
  int status = 200;
  std::string error;
  std::vector<std::string> lines;
  int chunks = 0;
  bool connected = false;
  std::string port;
  std::string firmware;
  std::vector<PortInfo> ports;
  std::vector<uint8_t> png;
  uint32_t width = 0;
  uint32_t height = 0;
};
class Device {
 public:
  Device(std::unique_ptr<SerialLink> serial, std::unique_ptr<ScreenCapture> capture);
  OpResult list_ports();
  OpResult connect(std::string port);
  OpResult disconnect();
  OpResult status();
  OpResult move(int dx, int dy);
  OpResult click(std::string_view button);
  OpResult scroll(int steps);
  OpResult demo(std::string_view pattern);
  OpResult echo(std::string_view text);
  OpResult help();
  OpResult type_text(std::string_view text);
  OpResult key_chord(std::string_view chord);
  OpResult command(std::string_view line);
  OpResult screenshot();
};
```

- [ ] **Step 1: Write the failing test**

`device_test` builds a `FakeSerial` that records lines. A `TYPE` line produces no success reply. The test can arm the second `TYPE` to return `ERR TYPE bad`. `FakeCapture` returns a 1×1 BGRA pixel `{1,2,3,255}` until told to fail with `no display`.

Assert:

- `move(1, 2)` while closed has status 409 and the fake recorded nothing.
- After `connect("/dev/cu.fake")`, `move(1, 2)` sends `MOVE 1 2` and status 200.
- `type_text` of 80 `b`s sends two `TYPE` lines and `chunks == 2`.
- When the second `TYPE` returns `ERR TYPE bad`, status is 502 and `error` contains `ERR TYPE bad`. There is no `completed` count.
- `key_chord("cmd")` is status 400 and records nothing.
- `screenshot()` status is not 409 when the port is closed.
- A capture error yields status 503 and an empty `png`.

`screenshot()` in this task may leave `png` empty until Task 4. The test that expects PNG bytes is in Task 4. This task only asserts 503 on capture failure and that a closed port does not block the call.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure -R device`

Expected: link error for `Device`.

- [ ] **Step 3: Implement Device**

Every public method takes `std::mutex`. `require_open()` returns 409 `{"serial closed"}` without calling `transact`.

`transact` appends the firmware lines. If any line starts with `ERR `, status becomes 502 and `error` is that line.

`type_text` formats every chunk first (400 on format error, nothing sent). Then it writes every chunk without waiting for a success line between them. `TYPE` success is silence. If a reply starts with `ERR `, status is 502 and `error` is that line. Do not look for `TYPE done`.

`connect` closes any previous port, calls `open_port`, `read_banner`, and stores the path. `status` sends `STATUS` when open and copies the `OK ` line into `firmware`.

`screenshot` calls `capture_primary`. On `ok == false`, status 503 and `error` is `PixelBuffer::error`. It does not look at the serial port.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure -R device`

Expected: `100% tests passed`.

- [ ] **Step 5: Commit**

```bash
git add host/src/capture.hpp host/src/device.hpp host/src/device.cpp host/tests/device_test.cpp host/CMakeLists.txt
git commit -m "$(cat <<'EOF'
feat: drive mouse and keyboard through one serial transaction

A closed port and a bad chord never reach the firmware.

EOF
)"
```

---

### Task 4: PNG encoder and capture backends

**Files:**
- Create: `host/src/png.cpp`
- Create: `host/src/capture_stub.cpp`
- Create: `host/src/capture_macos.mm`
- Test: `host/tests/png_test.cpp`
- Modify: `host/src/device.cpp` (fill `OpResult::png` from `encode_png`)
- Modify: `host/CMakeLists.txt` (Apple compiles `capture_macos.mm` and links CoreGraphics; otherwise `capture_stub.cpp`)

**Interfaces:**
- Consumes: `PixelBuffer`
- Produces: `std::vector<uint8_t> encode_png(const PixelBuffer& buffer)`. `make_screen_capture() -> std::unique_ptr<ScreenCapture>`.

- [ ] **Step 1: Write the failing test**

`png_test` builds a 2×1 BGRA buffer, stride 8, pixels `{0,0,255,255}` and `{0,255,0,255}`. `encode_png` must start with the eight bytes `89 50 4E 47 0D 0A 1A 0A`. Decode is not required. A second test constructs the stub type only when `LIVEUSE_TEST_STUB` is defined, calls `capture_primary`, and requires `error == "screen capture is not implemented on this OS"`. On Apple the CMake test target defines nothing and only runs the PNG assertion. A second test executable `stub_test` is built from `capture_stub.cpp` on every OS, including Apple, so the string is checked everywhere.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure -R 'png|stub'`

Expected: link error for `encode_png`.

- [ ] **Step 3: Implement PNG and the backends**

`encode_png` calls `stbi_write_png_to_func` with 4 components and the given stride. Define `STB_IMAGE_WRITE_IMPLEMENTATION` in `png.cpp` only.

`capture_stub.cpp` returns `ok = false` and the exact error string. It ignores the display.

`capture_macos.mm` calls `CGDisplayCreateImage(CGMainDisplayID())`. A null image returns error `screen recording permission denied or no display`. Draw the image into a BGRA8888 `CGBitmapContext` with `kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little`, then copy `width * height * 4` bytes into `PixelBuffer` with `stride = width * 4`. Release the image and the context.

`Device::screenshot` on success sets `png = encode_png(buffer)`, `width`, and `height`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure -R 'png|stub'`

Expected: `100% tests passed`.

- [ ] **Step 5: Commit**

```bash
git add host/src/png.cpp host/src/capture_stub.cpp host/src/capture_macos.mm host/src/device.cpp host/tests/png_test.cpp host/CMakeLists.txt
git commit -m "$(cat <<'EOF'
feat: encode the primary display as PNG behind a per-OS capture

Other systems link a stub with a fixed error until a backend exists.

EOF
)"
```

---

### Task 5: HTTP auth, MCP, and REST

**Files:**
- Create: `host/src/http.hpp`
- Create: `host/src/http.cpp`
- Test: `host/tests/http_test.cpp`
- Modify: `host/CMakeLists.txt`

**Interfaces:**
- Consumes: `Device`, `Config`, `token_equal`, `verify_password`
- Produces:

```cpp
class SessionStore {
 public:
  std::string issue();
  bool valid(std::string_view value) const;
  void clear();
};
struct HttpApp {
  Device& device;
  SessionStore& sessions;
  std::function<Config()> config;
  std::string mcp_url;
  std::filesystem::path web_root;
};
void install_routes(httplib::Server& server, HttpApp& app);
```

- [ ] **Step 1: Write the failing test**

`http_test` listens on `127.0.0.1` and an ephemeral port (`bind` port 0, read the port, close, then `listen`). Config token is 64 `a`s. User `ada`, password hash of `secret`. `FakeSerial` and `FakeCapture` from Task 3.

Assert:

- `POST /mcp` with no Authorization returns 401 and the body is not JSON-RPC (`jsonrpc` key absent).
- `POST /mcp` with `Bearer ` plus 64 `b`s returns 401.
- `POST /mcp` `initialize` with the good bearer returns 200, `serverInfo.name == "liveuse"`, and `instructions` contains the config name.
- `POST /mcp` `tools/list` names include `mouse_move`, `keyboard_type`, `keyboard_key`, `capture_screen`.
- `POST /mcp` `tools/call` `keyboard_key` with `chord` `cmd` returns 200 and `result.isError == true`.
- `POST /api/status` with no cookie returns 401.
- `POST /api/login` with a wrong password returns 401 and body `{"error":"credenciales invalidas"}`.
- `POST /api/login` with `ada` / `secret` returns 200 and `Set-Cookie` contains `liveuse_session=` and `HttpOnly` and `SameSite=Lax`.
- With that cookie, `POST /api/keyboard/type` `{"text":"hi"}` after connect returns 200 and `"lines"`.
- `PUT /api/config` `{"name":"mini","token":"nope"}` changes the name the next `GET /api/config` returns, and the token in that GET is still 64 `a`s.
- `GET /` when `web_root` is a missing path returns 200 text containing that path.
- `SessionStore::clear()` makes the cookie return 401.

Send `Accept: application/json, text/event-stream` on the MCP calls. Response `Content-Type` contains `application/json`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure -R http`

Expected: link error for `install_routes`.

- [ ] **Step 3: Implement the routes**

Bearer check: header must start with `Bearer `, remainder compared with `token_equal`. Otherwise 401 and an empty body.

MCP methods: `initialize`, `ping`, `tools/list`, `tools/call`. Other methods return a JSON-RPC error object with HTTP 200. `tools/call` maps `OpResult::status >= 400` to `isError: true` and a text content block with `error`. `capture_screen` adds an image content block, base64 of `png`, mime `image/png`, only when status is 200.

Tool argument names match the spec: `port`, `dx`, `dy`, `button`, `steps`, `pattern`, `text`, `line`, `chord`.

Line-producing routes return `{ "lines": string[] }`. `POST /api/keyboard/type` also returns `"chunks"`. Errors return `{ "error": string }` and never `"completed"`. `GET /api/session` returns `{ "username", "name" }`. `GET /api/status` returns `{ "connected", "port", "firmware" }`.

Register these routes and no others under `/api`: `POST /api/login`, `POST /api/logout`, `GET /api/session`, `GET /api/config`, `PUT /api/config`, `GET /api/ports`, `POST /api/connect`, `POST /api/disconnect`, `GET /api/status`, `POST /api/mouse/move`, `POST /api/mouse/click`, `POST /api/mouse/scroll`, `POST /api/mouse/demo`, `POST /api/echo`, `POST /api/help`, `POST /api/keyboard/type`, `POST /api/keyboard/key`, `POST /api/screenshot`, `POST /api/command`. Login sets the cookie. `PUT /api/config` reads only `name`, calls `HttpApp::rename`. It does not write `token` from the body.

`tools/list` publishes exactly these names: `list_serial_ports`, `connect_device`, `disconnect_device`, `device_status`, `mouse_move`, `mouse_click`, `mouse_scroll`, `mouse_demo`, `serial_echo`, `firmware_help`, `send_line`, `keyboard_type`, `keyboard_key`, `capture_screen`.

`GET /api/config` returns `name`, `mcpUrl` (`HttpApp::mcp_url`), and `token`.

Screenshot route sets `Content-Type: image/png`, `X-Image-Width`, and `X-Image-Height`.

Do not call httplib CORS helpers.

`SessionStore::issue` stores 32 random bytes as 64 hex with `steady_clock` expiry 12 hours. `valid` rejects expired entries.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure -R http`

Expected: `100% tests passed`.

- [ ] **Step 5: Commit**

```bash
git add host/src/http.hpp host/src/http.cpp host/tests/http_test.cpp host/CMakeLists.txt
git commit -m "$(cat <<'EOF'
feat: serve MCP and the REST API on one HTTP port

The bearer token and the session cookie open different doors.

EOF
)"
```

---

### Task 6: macOS serial, CLI, and reload socket

**Files:**
- Create: `host/src/serial_macos.cpp`
- Create: `host/src/main.cpp`
- Modify: `host/CMakeLists.txt` (target `liveuse`)
- Test: `host/tests/cli_test.cpp`

**Interfaces:**
- Consumes: `install_routes`, `save_config`, `load_config`, `make_config`, `socket_path`, `Device`, `make_screen_capture`
- Produces: `int liveuse_main(int argc, char** argv)` so tests can call it. `make_serial() -> std::unique_ptr<SerialLink>`.

- [ ] **Step 1: Write the failing test**

`cli_test` sets `HOME` to a temp directory and checks:

- `liveuse_main` with `init`, `--user`, `ada`, and stdin `secret\n`, creates `Library/Application Support/liveuse/config.json`.
- A second `init` without `--force` returns exit code 1 and stderr contains `config exists`.
- `init` with `--force` returns 0.
- `token rotate` changes `token` and keeps `username`.
- `password set` with stdin `next\n` makes `verify_password("next", hash)` true.
- `user set bob` changes `username` only.

The server-listen path is not started in this test (`init` / `token` / `user` / `password` return without binding 7395).

- [ ] **Step 2: Run the test to verify it fails**

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure -R cli`

Expected: link error for `liveuse_main`.

- [ ] **Step 3: Implement serial, CLI, and the socket**

`serial_macos.cpp` lists `/dev/cu.*`. `kind` is `usb` when the name contains `usb`, otherwise `other`. `open_port` opens the path `O_RDWR | O_NOCTTY | O_NONBLOCK`, then `termios` 115200, 8N1, raw (`cfmakeraw`), `VMIN 0`, `VTIME 1`. `transact` writes the line plus `\n`. For `TYPE`, read only until `ERR ` or 80 ms of silence, whichever comes first. Silence is success. For every other command, read until a completion line or 20 seconds (`KEY ` ending in ` done`, `MOVE done`, `CLICK ` ending in ` done`, `SCROLL done`, `DEMO ` ending in ` done`, a line starting with `OK `, `ECHO `, or `ERR `). `read_banner` reads for 300 ms.

`liveuse_main`:

- `init --user <name> [--force]`: read one password line from stdin with echo off when stdin is a tty. Refuse an existing file without `--force`. `save_config(config_path(), make_config(user, password))`.
- `token rotate`: load, `make_token()`, save, `notify_reload()`.
- `user set <name>`: load, change username, save, `notify_reload()`.
- `password set`: read stdin, `hash_password`, save, `notify_reload()`.
- no arguments: `load_config`. On failure print `run liveuse init --user <name>` and return 1. Build `Device` with `make_serial()` and `make_screen_capture()`. Start a thread on `liveuse.sock` that accepts one line `reload` and replies `ok\n` after replacing the `shared_ptr<const Config>` and calling `SessionStore::clear()`. Compute the MCP URL from `getifaddrs` (first `AF_INET` that is not `127.0.0.1`). `install_routes`, then `listen("0.0.0.0", 7395)`. On listen failure print the bind error and return 1.

`notify_reload` connects to `socket_path()` and writes `reload\n`. If the connect fails, it returns without an error (the next process start reads the file).

`rename` from `PUT /api/config` updates the in-memory config and `save_config`.

Do not print the token or the password.

- [ ] **Step 4: Run the tests**

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure`

Expected: every test passes, including `cli`, `http`, `cdc`, `config`, `device`, `png`, and `stub`.

- [ ] **Step 5: Commit**

```bash
git add host/src/serial_macos.cpp host/src/main.cpp host/tests/cli_test.cpp host/CMakeLists.txt
git commit -m "$(cat <<'EOF'
feat: start liveuse from the CLI and own the CDC port

init writes the private config, and a running server reloads it without dropping the serial link.

EOF
)"
```
