#pragma once

// Inserted before every translation unit. The library header sets
// CFG_TUD_HID to 1 and a second HID interface then stalls
// SET_CONFIGURATION, so macOS never configures CDC.
#include_next "tusb_config.h"

#undef CFG_TUD_HID
#define CFG_TUD_HID 2
