cmake_minimum_required(VERSION 3.24)
project(liveuse VERSION 0.3.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

include(FetchContent)

FetchContent_Declare(
  httplib
  GIT_REPOSITORY https://github.com/yhirose/cpp-httplib.git
  GIT_TAG v0.18.3
)
FetchContent_Declare(
  json
  GIT_REPOSITORY https://github.com/nlohmann/json.git
  GIT_TAG v3.11.3
)
FetchContent_Declare(
  stb
  GIT_REPOSITORY https://github.com/nothings/stb.git
  GIT_TAG 2c980bb59875b0d32144a71867fbdebb2f77cd20
)
FetchContent_MakeAvailable(httplib json stb)

add_library(keymap STATIC ../firmware/src/keymap.cpp)
target_include_directories(keymap PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/../firmware/include)

set(LIVEUSE_CORE
  src/cdc.cpp
  src/config.cpp
  src/control.cpp
  src/http_app.cpp
  src/locate.cpp
  src/png.cpp
  src/pointer.cpp
  src/tween.cpp
)
add_library(liveuse_core STATIC ${LIVEUSE_CORE})
target_include_directories(liveuse_core PUBLIC src ${stb_SOURCE_DIR})
target_link_libraries(liveuse_core PUBLIC keymap httplib::httplib nlohmann_json::nlohmann_json
  "-framework CoreFoundation" "-framework Security")

if(APPLE)
  set(LIVEUSE_CAPTURE src/capture_macos.mm)
  set(LIVEUSE_CAPTURE_LIBS "-framework CoreGraphics" "-framework CoreFoundation")
else()
  set(LIVEUSE_CAPTURE src/capture_stub.cpp)
  set(LIVEUSE_CAPTURE_LIBS "")
endif()

add_executable(liveuse src/main.cpp src/serial_macos.cpp ${LIVEUSE_CAPTURE})
target_link_libraries(liveuse PRIVATE liveuse_core ${LIVEUSE_CAPTURE_LIBS})

enable_testing()

function(liveuse_test name)
  add_executable(${name} ${ARGN})
  target_link_libraries(${name} PRIVATE liveuse_core)
  add_test(NAME ${name} COMMAND ${name})
endfunction()

liveuse_test(cdc_test tests/cdc_test.cpp)
target_link_libraries(cdc_test PRIVATE keymap)
liveuse_test(config_test tests/config_test.cpp)
liveuse_test(locate_test tests/locate_test.cpp)
liveuse_test(pointer_test tests/pointer_test.cpp)
liveuse_test(http_test tests/http_test.cpp)

add_executable(capture_stub_test tests/capture_stub_test.cpp src/capture_stub.cpp)
target_include_directories(capture_stub_test PRIVATE src)
add_test(NAME capture_stub_test COMMAND capture_stub_test)
