// libFuzzer's entry point for every DPE fuzz target. Each `cc_fuzz_test` in BUILD.bazel links
// exactly one shim from `shims/`, and every shim defines `dpe_fuzz_entry`, so the call below
// reaches that target's body. The declaration mirrors the Rust signature by hand: a mismatch is
// undefined behaviour the compiler cannot see.

// The C headers: this file is pure C ABI.
#include <stddef.h>
#include <stdint.h>

extern "C" int dpe_fuzz_entry(const uint8_t *data, size_t len);

extern "C" int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size) {
  return dpe_fuzz_entry(data, size);
}
