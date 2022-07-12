# XMMAP

let me announcing the smallest feature-rich industrial-ready no-std cross-platform memory mapping api for rust!

## [**If you are windows user and you want to use large page, please read**](https://docs.microsoft.com/en/windows/security/threat-protection/security-policy-settings/lock-pages-in-memory)


## Motivation

`memmap` is dead!

and the API of memmap is not very user friendly.

# Features

## Common
- [x] 🚧 file memory maps
- [x] 🚧 anonymous memory maps
- [x] 🚧 access control
    - [x] 🚧 read
    - [x] 🚧 write
    - [x] 🚧 execute
- [x] 🚧 sync flush
- [x] 🚧 flush view
- [x] 🚧 async flush view
- [x] 🚧 wait all async flush done
## Common Huge Page
- [x] common huge page support
## Windows
- [x] first class windows support
- [ ] 🚧 copy on write
## Unix
- [ ] 🚧 Unix Flags
- [ ] 🚧 Unix advice
### Linux
- [ ] 🚧 Linux Flags
- [ ] 🚧 Linux advice
### BSD
- [ ] 🚧 BSD Flags
- [ ] 🚧 BSD advice

### MacOS: **Donate me a Mac if you'd like to.**
- [ ] 🚧 MacOS Flags
- [ ] 🚧 MacOS advice

# Targets
cpu architectures
- [x] x86_64
- [x] 🚧 i686
- [x] 🚧 aarch64

operating systems ci status
- [x] windows-msvc
- [x] windows-gnu
- [x] 🚧 linux
- [x] 🚧 linux-musl
- [x] 🚧 apple-darwin
- [ ] 🚧 apple-ios
- [ ] 🚧 linux-android
- [ ] 🚧 freebsd