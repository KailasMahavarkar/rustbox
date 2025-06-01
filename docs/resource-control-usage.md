# Mini-Isolate Resource Control Examples

## Using the New Resource Limit Flags

The mini-isolate project now supports runtime resource control via command-line flags. These flags allow you to override the resource limits configured during isolate initialization.

### Available Flags

- `--max-cpu SECONDS`: Override CPU time limit in seconds  
- `--max-memory MB`: Override memory limit in megabytes
- `--max-time SECONDS`: Override wall clock time limit in seconds

### Usage Examples

#### 1. Basic Usage with Run Command

```bash
# Initialize an isolate with default limits
mini-isolate init --box-id 0 --mem 128 --time 10

# Run a command with overridden resource limits
mini-isolate run --box-id 0 \
    --max-cpu 30 \
    --max-memory 256 \
    --max-time 60 \
    "/usr/bin/python3" -- script.py
```

#### 2. Execute Source Files with Custom Limits

```bash
# Execute a Python script with custom limits
mini-isolate execute --box-id 0 \
    --source heavy_computation.py \
    --max-cpu 60 \
    --max-memory 512 \
    --max-time 120 \
    --input data.txt \
    --output results.json
```

#### 3. Selective Override

```bash
# Override only memory limit (CPU and time use isolate defaults)
mini-isolate run --box-id 0 \
    --max-memory 1024 \
    "memory_intensive_program"

# Override only time limits
mini-isolate execute --box-id 0 \
    --source benchmark.cpp \
    --max-cpu 300 \
    --max-time 600
```

#### 4. Contest/Competition Usage

```bash
# Different limits for different problem types
# Quick problems - strict limits
mini-isolate run --box-id contest \
    --max-cpu 1 \
    --max-memory 64 \
    --max-time 2 \
    "./solution"

# Complex problems - relaxed limits  
mini-isolate run --box-id contest \
    --max-cpu 10 \
    --max-memory 512 \
    --max-time 20 \
    "./complex_solution"
```

#### 5. Testing Scenarios

```bash
# Test with increasingly strict limits
for cpu_limit in 1 2 5 10; do
    echo "Testing with ${cpu_limit}s CPU limit..."
    mini-isolate run --box-id test \
        --max-cpu $cpu_limit \
        --max-memory 128 \
        --output "result_${cpu_limit}s.json" \
        "./test_program"
done
```

### Implementation Details

The resource override flags work by:

1. Loading the existing isolate instance configuration
2. Creating a temporary copy of the configuration
3. Applying the override values to CPU, memory, and time limits  
4. Using the modified configuration for execution
5. Preserving the original isolate instance configuration

### Compatibility

- These flags are compatible with all existing mini-isolate functionality
- If no override flags are specified, the isolate instance's original limits apply
- Override flags only affect the single execution, not the persistent isolate configuration
- Flags can be used individually or in combination

### Performance Considerations

- Resource limit overrides have minimal performance impact
- The configuration override happens in memory before process execution
- Original isolate configuration remains unchanged for future executions

### Use Cases

1. **Contest Environments**: Different time/memory limits per problem
2. **Educational Platforms**: Adjusting limits based on assignment complexity  
3. **Testing Frameworks**: Systematic testing with varying resource constraints
4. **Development**: Quick testing with different resource profiles
5. **Production**: Fine-tuning limits based on workload characteristics