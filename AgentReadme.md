# Agent Development Notes

## Project Structure Standards

This project follows standard Rust project conventions:

- `docs/` - All documentation files
- `tests/` - Test files and test utilities
- Root level files should be minimized

### Cgroup Permissions and Resource Isolation (Current Session)

**Important**: Mini-isolate requires sudo/root permissions for full cgroup-based resource isolation, but works gracefully without it.

#### Permission Requirements

**Without sudo:**
- ✅ **All functionality works** (process execution, timeouts, file operations)
- ⚠️ **Resource limits are not enforced** (memory, CPU limits are ineffective)  
- ⚠️ **Warning message displayed**: "Cannot create cgroup (permission denied). Resource limits will not be enforced."

**With sudo:**
- ✅ **Full cgroup support** with actual memory/CPU/process limits
- ✅ **Resource monitoring** (peak memory usage, CPU time tracking)
- ✅ **Enhanced isolation** and security

#### Technical Details

1. **Cgroup directory structure** is owned by root:
   ```
   drwxr-xr-x 10 root root   0 /sys/fs/cgroup/memory/
   ```

2. **Creating subdirectories** requires root privileges:
   - `/sys/fs/cgroup/memory/[instance_name]/`
   - `/sys/fs/cgroup/cpu/[instance_name]/`
   - `/sys/fs/cgroup/pids/[instance_name]/`

3. **Writing cgroup files** requires root access for limit enforcement

#### Design Philosophy

The library follows **graceful degradation**: continues operation without cgroups when permission is denied, logging appropriate warnings.

#### Usage Recommendations

1. **For development/testing**: Run without sudo - basic functionality works fine
2. **For production/sandboxing**: Use sudo for proper resource isolation  
3. **For judges/competitive programming**: Sudo recommended for memory/time enforcement

#### Alternative Solutions

- **Docker containers**: Run mini-isolate inside containers with cgroup delegation
- **User namespaces**: Configure user namespaces for unprivileged cgroup access
- **Systemd user slices**: Use systemd for resource management instead of direct cgroups

## Recent Changes

### Resource Control Enhancement (Session Date: Current)

Added command-line flags for runtime resource control:
- `--max-cpu SECONDS` - Override CPU time limit
- `--max-memory MB` - Override memory limit  
- `--max-time SECONDS` - Override wall clock time limit

**Files Modified:**
- `src/cli.rs` - Added new CLI arguments to Run and Execute commands
- `src/isolate.rs` - Added `execute_with_overrides()` and `execute_file_with_overrides()` methods

**Documentation:**
- `docs/resource-control-usage.md` - Usage examples and implementation details

**Tests:**
- `tests/resource_limits/` - Test suite for resource limit override functionality
  - `mod.rs` - Rust integration tests
  - `test_runner.py` - Python test automation script  
  - `cpu_test.py`, `memory_test.py`, `time_test.py` - Test programs

### Implementation Notes

1. **Backend Architecture**: Resource overrides work by cloning the isolate configuration and applying temporary modifications before execution
2. **Backwards Compatibility**: Original execution methods preserved - overrides only apply when flags are specified
3. **Type Safety**: All flags properly validated through clap argument parsing

### Future Development Considerations

- Resource override flags are extensible - additional limits can be added following the same pattern
- The override system preserves original isolate configuration for subsequent executions
- Performance impact is minimal as config cloning only occurs when override flags are used

## Development Workflow

1. Always place documentation in `docs/`
2. Place test files in `tests/` (take reference from exisiting file structure of /tests)
3. Update this AgentReadme.md for significant changes
4. Follow existing code patterns and naming conventions
5. Ensure backwards compatibility unless explicitly breaking changes are required