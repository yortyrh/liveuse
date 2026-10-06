#include "hid_reports.h"

// Adafruit keeps one global HID and returns that report descriptor for every
// instance. Both interfaces then enumerate as keyboards and mouse reports
// never move the cursor. The link wraps tud_hid_descriptor_report_cb.
static uint8_t const *g_desc[2];

void hid_report_set(uint8_t instance, uint8_t const *desc) {
  if (instance < 2) {
    g_desc[instance] = desc;
  }
}

extern "C" uint8_t const *__wrap_tud_hid_descriptor_report_cb(uint8_t instance) {
  if (instance < 2 && g_desc[instance] != nullptr) {
    return g_desc[instance];
  }
  return nullptr;
}
