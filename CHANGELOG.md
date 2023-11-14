## 0.14.0
- Update to drm-rs 0.11
- Use `BorrowedFd` instead of `RawFd` in API
- Don't require generated bindings for specific OS/architecture to build
- Fix build without default features

## 0.13.0

- Update to drm-rs 0.10
- Update wayland-server to 0.31

## 0.12.0

- Update to drm-rs 0.9

## 0.11.0

- Test for `-1` in fd and fd_for_plane

## 0.10.0

- Update `wayland-rs` to 0.30
- Use io-safe types over `RawFd`
- Update to drm-rs 0.8
- YANKED: No errors for fd-methods, use 0.11.0

## 0.9.0

- Update to drm-rs 0.7

## 0.8.0

- Update to drm-rs 0.6

## 0.7.0

- Update to drm-rs 0.5

## 0.6.0

- Update to drm-rs 0.4
- Update bindings, add new functionality
- Make Device clonable
- Use drm-fourcc for Formats
- Implement Send where applicable
- Switch to new std-Error trait

## 0.5.0

- Make `Surface::lock_front_buffer` unsafe as it may cause segfaults

## 0.4.0

- API overhaul to use ref-counting internally to:
  - Enable out-of-order destruction without causing leaks, crashes or double-frees
  - Remove lifetimes, which made this api a pain to work with and almost required hacks like the `rental` crate
- Remove `FromRaw` as it is not possible to create most structs anymore without a reference to the underlying `Device`
- Remove `Device` initializers other then `new_from_fd`. Lifetimes do not exist anymore and it is part of the contract to drop the `Device` before closing the file descriptor.
- Add `Device` initializer `new` that wraps any open drm device.
- Implement the [`drm-rs`](https://github.com/Smithay/drm-rs) `Device` traits for `Device` where applicable.

## 0.3.0

- Upgrade to bitflags 1.0 with associated consts

## 0.2.0

- drm-rs support

## 0.1.0

- Initial release
