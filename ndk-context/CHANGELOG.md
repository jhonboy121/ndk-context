# Unreleased

- uplink patch: `initialize_android_context` keeps the first context and ignores later calls;
  `release_android_context` is a no-op; storage is a `OnceLock` instead of `static mut`.

# 0.1.1 (2022-04-19)

- Add `release_android_context()` function to remove `AndroidContext` when activity is destroyed. (#263)

# 0.1.0 (2022-02-14)

- Initial release! 🎉
