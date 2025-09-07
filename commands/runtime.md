# MSO Runtime Command

## Overview
The `runtime` command manages MSO virtual machine execution and runtime operations.

## Usage
```bash
helix runtime <subcommand> [options]
```

## Subcommands

### `execute`
Execute a compiled MSO binary.

```bash
helix runtime execute <binary> [options]
```

**Options:**
- `--debug`: Enable debug mode with breakpoints
- `--step`: Step through execution one instruction at a time
- `--breakpoint <address>`: Set breakpoint at specific address
- `--memory-limit <size>`: Set memory limit (default: 64KB)
- `--stack-limit <size>`: Set stack limit (default: 1KB)

**Example:**
```bash
helix runtime execute config.hlxb --debug --breakpoint 42
```

### `debug`
Start interactive debugger for MSO binary.

```bash
helix runtime debug <binary> [options]
```

**Options:**
- `--port <port>`: Debug server port (default: 9229)
- `--host <host>`: Debug server host (default: localhost)

**Example:**
```bash
helix runtime debug config.hlxb --port 9230
```

### `profile`
Profile MSO binary execution performance.

```bash
helix runtime profile <binary> [options]
```

**Options:**
- `--iterations <count>`: Number of iterations to run (default: 1000)
- `--output <file>`: Output profile data to file
- `--format <format>`: Output format (json, csv, text) (default: text)

**Example:**
```bash
helix runtime profile config.hlxb --iterations 5000 --output profile.json
```

### `validate`
Validate MSO binary for runtime execution.

```bash
helix runtime validate <binary> [options]
```

**Options:**
- `--strict`: Enable strict validation mode
- `--check-memory`: Validate memory access patterns
- `--check-stack`: Validate stack operations

**Example:**
```bash
helix runtime validate config.hlxb --strict --check-memory
```

## Features

### Virtual Machine
- **Stack-based Execution**: Efficient stack-based instruction execution
- **Memory Management**: Configurable memory limits and garbage collection
- **Register System**: Program counter, stack pointer, frame pointer management
- **Call Stack**: Function call tracking and return address management
- **Breakpoints**: Debug breakpoint support with hit counting

### Debugging Support
- **Interactive Debugger**: Step-through debugging with breakpoints
- **Memory Inspection**: View and modify VM memory state
- **Stack Inspection**: Examine call stack and local variables
- **Instruction Tracing**: Trace instruction execution
- **Performance Profiling**: Detailed execution performance analysis

### Runtime Configuration
- **Memory Limits**: Configurable maximum memory usage
- **Stack Limits**: Configurable maximum stack size
- **Call Depth Limits**: Maximum function call depth
- **Garbage Collection**: Optional garbage collection for memory management
- **Execution Timeouts**: Configurable execution time limits

## Examples

### Basic Execution
```bash
# Execute binary
helix runtime execute config.hlxb

# Execute with debug mode
helix runtime execute config.hlxb --debug
```

### Advanced Debugging
```bash
# Set breakpoints and step through
helix runtime execute config.hlxb \
  --debug \
  --breakpoint 10 \
  --breakpoint 25 \
  --step

# Start interactive debugger
helix runtime debug config.hlxb --port 9230
```

### Performance Analysis
```bash
# Profile execution
helix runtime profile config.hlxb \
  --iterations 10000 \
  --output performance.json

# Validate runtime safety
helix runtime validate config.hlxb \
  --strict \
  --check-memory \
  --check-stack
```

## VM Architecture

### Execution State
```rust
pub enum ExecutionState {
    Ready,
    Running,
    Paused,
    Halted,
    Error(String),
}
```

### VM Registers
```rust
pub struct VMRegisters {
    pub program_counter: usize,
    pub stack_pointer: usize,
    pub frame_pointer: usize,
    pub return_address: usize,
    pub flags: VMFlags,
}
```

### Memory Management
- **Stack**: LIFO stack for temporary values and function calls
- **Memory**: HashMap-based memory for persistent data
- **Registers**: CPU-like registers for execution state
- **Call Frames**: Function call context and local variables

## Debug Commands

When using the interactive debugger:

### Basic Commands
- `step` or `s`: Execute one instruction
- `continue` or `c`: Continue execution
- `break <address>`: Set breakpoint at address
- `delete <address>`: Remove breakpoint
- `list`: Show current instruction and context

### Memory Commands
- `memory <address>`: Show memory at address
- `stack`: Show current stack state
- `registers`: Show register values
- `variables`: Show local variables

### Execution Commands
- `run`: Start execution
- `stop`: Stop execution
- `restart`: Restart from beginning
- `quit` or `q`: Exit debugger

## Performance Profiling

### Metrics Collected
- **Instructions Executed**: Total instruction count
- **Memory Usage**: Peak and average memory usage
- **Stack Depth**: Maximum stack depth reached
- **Call Depth**: Maximum function call depth
- **Execution Time**: Total and per-instruction timing
- **Cache Performance**: Instruction and data cache statistics

### Output Formats

#### JSON Format
```json
{
  "execution_time_ms": 1234,
  "instructions_executed": 5678,
  "memory_usage": {
    "peak": 1024,
    "average": 512
  },
  "stack_depth": {
    "max": 64,
    "average": 32
  }
}
```

#### CSV Format
```csv
iteration,time_ms,instructions,memory_peak,stack_max
1,1.2,100,512,32
2,1.1,98,510,31
...
```

## Error Handling

### Runtime Errors
- **Stack Overflow**: Stack size exceeded
- **Stack Underflow**: Pop from empty stack
- **Memory Access Violation**: Invalid memory access
- **Invalid Instruction**: Unknown or malformed instruction
- **Execution Timeout**: Execution time limit exceeded

### Debug Errors
- **Breakpoint Not Found**: Invalid breakpoint address
- **Debug Server Error**: Debug server connection issues
- **Memory Inspection Error**: Invalid memory address

## Best Practices

1. **Use Debug Mode**: Enable debug mode for development and testing
2. **Set Appropriate Limits**: Configure memory and stack limits for your use case
3. **Profile Performance**: Regular performance profiling for optimization
4. **Validate Binaries**: Always validate binaries before production use
5. **Monitor Resources**: Watch memory and stack usage during execution
6. **Use Breakpoints**: Strategic breakpoints for debugging complex logic
