---
title: Rust Core C ABI
description: Canonical contract for generated native-shell bindings to the Rust core.
---

# Rust core C ABI

This document is the repository-owned single source of truth for the Rust core/native shell boundary. Both shells consume bindings generated from the same generated C header; neither shell may maintain hand-copied declarations.

## Boundary and data representation

- Export a stable C ABI from the Rust core. Use Rust `#[repr(C)]` types for C-visible structs and enums.
- Pass complex inputs and results as UTF-8 JSON strings. Keep function arguments and return values C-compatible; do not expose Rust-owned collections, trait objects, or language-specific types.
- Provider refresh uses `core_refresh_provider(id)` and returns a JSON string containing the result or structured error. The exact JSON schema must be versioned and specified with the provider API before implementation.
- Generate `core.h` from the Rust declarations with `cbindgen`. Generate Swift and C# shell bindings from that header using platform-appropriate generators. Never hand-copy or independently redefine ABI declarations.

## String ownership

- Every allocated string returned by the core is owned by the caller after return and remains valid until released.
- The caller must pass each returned string to `core_free_string` exactly once, including error-result strings, after copying or decoding its contents.
- `core_free_string` accepts only a non-null pointer returned by the core. It must not be used on shell-allocated memory, and a pointer must not be used after it is freed.
- The shell must not free an input string unless the corresponding function's contract explicitly transfers ownership; inputs are borrowed for the duration of the call by default.

## ABI version check

- Export `core_abi_version()` to report the ABI version supported by the loaded core library.
- Each shell checks `core_abi_version()` during startup before making any other core calls.
- A shell must refuse to call an incompatible core and present an actionable startup error. Compatibility/versioning policy must be maintained with the generated header and release packaging.

## Required public functions

The initial boundary includes at least:

```c
uint32_t core_abi_version(void);
char *core_refresh_provider(const char *id);
void core_free_string(char *value);
```

These signatures describe the C-facing contract. `core_refresh_provider` returns a JSON-encoded result or error in a core-owned string. Header declarations and shell bindings are generated, not copied from this example. Any additional exported function must follow the same representation, ownership, and version-check rules.
