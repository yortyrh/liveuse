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

enable_testing()

add_executable(cdc_test tests/cdc_test.cpp src/cdc.cpp)
target_include_directories(cdc_test PRIVATE src)
target_link_libraries(cdc_test PRIVATE keymap)
add_test(NAME cdc COMMAND cdc_test)
