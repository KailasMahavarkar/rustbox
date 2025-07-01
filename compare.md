| Feature | `isolate-reference` (C) | `mini-isolate` (Rust) | Status |
| :--- | :--- | :--- | :--- |
| **Core Lifecycle** | `init`, `run`, `cleanup` commands | `init`, `run`, `cleanup` commands | ✅ In Both |
| **Instance Listing** | ❌ Not available | `list` command | 🟢 `mini-isolate` Only |
| **Direct Execution** | ❌ Not available | `execute` command for source files | 🟢 `mini-isolate` Only |
| **System Info** | `isolate-check-environment` script | `info` command | ✅ In Both (different approach) |
| **Control Groups** | cgroup v2 | cgroup v1 (Simplified) | 🟡 Partial / Different |
| **File Locking** | `flock` based, binary format | `flock` based, text format | ✅ In Both (Recently fixed) |
| **Configuration** | System-wide file (`/etc/isolate`) | Per-instance JSON in temp dir | 🟡 Partial / Different |
| **Memory Limit** | ✅ `--mem`, `--cg-mem` | ✅ `--mem` | ✅ In Both |
| **Time Limit** | ✅ `--time`, `--wall-time`, `--extra-time` | ✅ `--time`, `--wall-time` | 🟡 Partial (`--extra-time` missing) |
| **Process Limit** | ✅ `--processes` | ✅ `--processes` | ✅ In Both |
| **File Size Limit** | ✅ `--fsize` | ✅ `--fsize` | ✅ In Both |
| **Stack Size Limit** | ✅ `--stack` | ❌ Not implemented | 🔴 Reference Only |
| **Core Dump Limit** | ✅ `--core` | ❌ Not implemented | 🔴 Reference Only |
| **Disk Quota** | ✅ `--quota` (ext fs only) | ❌ Not implemented | 🔴 Reference Only |
| **Filesystem Isolation**| Advanced `--dir` rules, chroot | Basic `workdir`, no chroot or advanced rules | 🔴 Reference Only |
| **Environment Vars** | ✅ `--env`, `--full-env` | ✅ Basic support | ✅ In Both |
| **Networking** | ✅ `--share-net` | ✅ `enable_network` config flag | ✅ In Both |
| **User/Group Control** | ✅ `--as-uid`, `--as-gid` | ✅ `uid`/`gid` config, no CLI override | 🟡 Partial |
| **Metadata Output** | ✅ `--meta` file (key:value format) | ✅ `--output` file (JSON format) | ✅ In Both (different format) |
| **I/O Redirection** | ✅ `--stdin`, `--stdout`, `--stderr` | ✅ `--input` (stdin only) | 🟡 Partial |
| **Process Waiting** | ✅ `--wait` | ❌ Not implemented | 🔴 Reference Only |
| **TTY Support** | ✅ `--tty-hack` | ❌ Not implemented | 🔴 Reference Only |
