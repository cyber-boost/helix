# HELIX Compiler Runtime System - Technical Documentation

## Overview
The `compiler/runtime.rs` file implements a virtual machine for executing compiled MSO binaries. This system provides runtime execution capabilities with debugging support, memory management, and performance monitoring.

## Architecture

### Core Components

#### HelixVM
The main virtual machine that executes MSO binaries:

```rust
pub struct HelixVM {
    stack: Vec<Value>,
    memory: HashMap<u32, Value>,
    registers: VMRegisters,
    config: HelixConfig,
    call_stack: VecDeque<CallFrame>,
    execution_state: ExecutionState,
    debug_mode: bool,
    breakpoints: HashMap<usize, Breakpoint>,
}
```

**Key Components:**
- **Stack**: LIFO stack for temporary values and function calls
- **Memory**: HashMap-based memory for persistent data storage
- **Registers**: CPU-like registers for execution state
- **Call Stack**: Function call context and local variables
- **Execution State**: Current VM execution state
- **Debug Support**: Breakpoints and debugging capabilities

#### VMRegisters
CPU-like registers for execution state management:

```rust
pub struct VMRegisters {
    pub program_counter: usize,
    pub stack_pointer: usize,
    pub frame_pointer: usize,
    pub return_address: usize,
    pub flags: VMFlags,
}
```

**Register Functions:**
- **Program Counter**: Points to current instruction
- **Stack Pointer**: Points to top of stack
- **Frame Pointer**: Points to current call frame
- **Return Address**: Stores return address for function calls
- **Flags**: Execution flags (zero, overflow, error, halted)

#### VMFlags
Execution flags for conditional operations:

```rust
pub struct VMFlags {
    pub zero: bool,
    pub overflow: bool,
    pub error: bool,
    pub halted: bool,
}
```

**Flag Usage:**
- **Zero**: Set when operation result is zero
- **Overflow**: Set when arithmetic overflow occurs
- **Error**: Set when runtime error occurs
- **Halted**: Set when VM execution is halted

### Execution Model

#### Execution State
Tracks the current state of VM execution:

```rust
pub enum ExecutionState {
    Ready,
    Running,
    Paused,
    Halted,
    Error(String),
}
```

**State Transitions:**
- **Ready → Running**: Start execution
- **Running → Paused**: Hit breakpoint or step mode
- **Running → Halted**: Normal completion
- **Running → Error**: Runtime error occurred
- **Paused → Running**: Continue execution

#### Instruction Execution
The VM executes instructions in a loop:

```rust
while self.registers.program_counter < ir.instructions.len() && 
      self.execution_state == ExecutionState::Running {
    
    // Check for breakpoints in debug mode
    if self.debug_mode {
        if let Some(bp) = self.breakpoints.get_mut(&self.registers.program_counter) {
            if bp.active {
                bp.hit_count += 1;
                self.execution_state = ExecutionState::Paused;
                break;
            }
        }
    }
    
    let instruction = &ir.instructions[self.registers.program_counter];
    self.execute_instruction(instruction)?;
}
```

**Execution Process:**
1. **Breakpoint Check**: Check for debug breakpoints
2. **Instruction Fetch**: Get current instruction
3. **Instruction Execute**: Execute the instruction
4. **Program Counter Update**: Increment program counter
5. **State Check**: Check for halt or error conditions

### Instruction Set

#### Supported Instructions
The VM supports all MSO IR instructions:

```rust
match instruction {
    IR::DeclareAgent(id) => self.declare_agent(*id)?,
    IR::DeclareWorkflow(id) => self.declare_workflow(*id)?,
    IR::DeclareContext(id) => self.declare_context(*id)?,
    IR::DeclareCrew(id) => self.declare_crew(*id)?,
    IR::SetProperty { target, key, value } => self.set_property(*target, *key, value)?,
    IR::SetCapability { agent, capability } => self.set_capability(*agent, *capability)?,
    IR::SetSecret { context, key, secret } => self.set_secret(*context, *key, secret)?,
    IR::DefineStep { workflow, step } => self.define_step(*workflow, step)?,
    IR::DefinePipeline { workflow, nodes } => self.define_pipeline(*workflow, nodes)?,
    IR::ResolveReference { ref_type, index } => self.resolve_reference(ref_type, *index)?,
    IR::SetMetadata { key, value } => self.set_metadata(*key, *value)?,
}
```

**Instruction Categories:**
- **Declaration Instructions**: Create agents, workflows, contexts, crews
- **Property Instructions**: Set properties on entities
- **Capability Instructions**: Set agent capabilities
- **Secret Instructions**: Set context secrets
- **Step Instructions**: Define workflow steps
- **Pipeline Instructions**: Define workflow pipelines
- **Reference Instructions**: Resolve environment variables and memory
- **Metadata Instructions**: Set metadata values

### Memory Management

#### Stack Operations
LIFO stack for temporary values and function calls:

```rust
pub fn push(&mut self, value: Value) -> VMResult<()> {
    if self.stack.len() >= 1024 { // Stack size limit
        return Err(RuntimeError {
            kind: RuntimeErrorKind::StackOverflow,
            message: "Stack overflow".to_string(),
            stack_trace: self.get_stack_trace(),
        });
    }
    self.stack.push(value);
    self.registers.stack_pointer += 1;
    Ok(())
}

pub fn pop(&mut self) -> VMResult<Value> {
    if self.stack.is_empty() {
        return Err(RuntimeError {
            kind: RuntimeErrorKind::StackUnderflow,
            message: "Stack underflow".to_string(),
            stack_trace: self.get_stack_trace(),
        });
    }
    self.registers.stack_pointer -= 1;
    Ok(self.stack.pop().unwrap())
}
```

**Stack Features:**
- **Size Limits**: Configurable stack size limits
- **Overflow Protection**: Prevents stack overflow
- **Underflow Protection**: Prevents stack underflow
- **Pointer Tracking**: Maintains stack pointer register

#### Memory Operations
HashMap-based memory for persistent data:

```rust
pub fn load_memory(&self, address: u32) -> VMResult<&Value> {
    self.memory.get(&address).ok_or_else(|| RuntimeError {
        kind: RuntimeErrorKind::MemoryAccessViolation,
        message: format!("Invalid memory access at address {}", address),
        stack_trace: self.get_stack_trace(),
    })
}

pub fn store_memory(&mut self, address: u32, value: Value) -> VMResult<()> {
    self.memory.insert(address, value);
    Ok(())
}
```

**Memory Features:**
- **Address Validation**: Validates memory addresses
- **Access Violation Detection**: Detects invalid memory access
- **Persistent Storage**: Values persist across function calls
- **Efficient Lookup**: O(1) memory access using HashMap

### Debugging Support

#### Breakpoints
Support for execution breakpoints:

```rust
pub struct Breakpoint {
    pub active: bool,
    pub condition: Option<String>,
    pub hit_count: usize,
}
```

**Breakpoint Features:**
- **Address-based**: Set breakpoints at specific addresses
- **Conditional**: Support for conditional breakpoints
- **Hit Counting**: Track how many times breakpoint is hit
- **Active/Inactive**: Enable/disable breakpoints

#### Debug Commands
Interactive debugging capabilities:

```rust
pub fn set_breakpoint(&mut self, address: usize)
pub fn remove_breakpoint(&mut self, address: usize)
pub fn continue_execution(&mut self)
pub fn step(&mut self)
```

**Debug Operations:**
- **Breakpoint Management**: Add/remove breakpoints
- **Execution Control**: Continue, step, pause execution
- **State Inspection**: View registers, memory, stack
- **Stack Trace**: Get execution call stack

#### Stack Trace
Provides execution context for debugging:

```rust
fn get_stack_trace(&self) -> Vec<String> {
    let mut trace = Vec::new();
    trace.push(format!("PC: {}", self.registers.program_counter));
    
    for (i, frame) in self.call_stack.iter().enumerate() {
        trace.push(format!("Frame {}: return address {}", i, frame.return_address));
    }
    
    trace
}
```

### High-Level Interface

#### VMExecutor
High-level interface for VM execution:

```rust
pub struct VMExecutor {
    vm: HelixVM,
}
```

**Features:**
- **File Execution**: Execute binaries from files
- **Debug Mode**: Enable debugging for execution
- **VM Access**: Direct access to VM for debugging
- **Error Handling**: Comprehensive error handling

#### Execution Methods
```rust
pub fn execute_file<P: AsRef<Path>>(&mut self, path: P) -> VMResult<HelixConfig>
pub fn execute_with_debug<P: AsRef<Path>>(&mut self, path: P) -> VMResult<HelixConfig>
pub fn vm(&mut self) -> &mut HelixVM
```

### Configuration

#### VMConfig
Configurable VM parameters:

```rust
pub struct VMConfig {
    pub max_stack_size: usize,
    pub max_memory: usize,
    pub max_call_depth: usize,
    pub enable_gc: bool,
    pub gc_threshold: usize,
}
```

**Default Values:**
- **Max Stack Size**: 1024 elements
- **Max Memory**: 65536 entries
- **Max Call Depth**: 256 levels
- **Garbage Collection**: Disabled
- **GC Threshold**: 1000 entries

### Error Handling

#### Runtime Errors
Comprehensive error handling system:

```rust
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub message: String,
    pub stack_trace: Vec<String>,
}
```

**Error Types:**
- **StackOverflow**: Stack size exceeded
- **StackUnderflow**: Pop from empty stack
- **MemoryAccessViolation**: Invalid memory access
- **InvalidInstruction**: Unknown or malformed instruction
- **ResourceNotFound**: Cannot find required resource

#### Error Recovery
- **Graceful Degradation**: VM continues execution when possible
- **Detailed Messages**: Clear error messages with context
- **Stack Traces**: Full execution context for debugging
- **Error Classification**: Categorized error types for handling

### Performance Monitoring

#### VM Statistics
Track VM performance metrics:

```rust
pub struct VMStats {
    pub instructions_executed: usize,
    pub stack_size: usize,
    pub memory_usage: usize,
    pub call_depth: usize,
}
```

**Metrics:**
- **Instructions Executed**: Total instruction count
- **Stack Size**: Current stack usage
- **Memory Usage**: Current memory usage
- **Call Depth**: Current function call depth

#### Performance Optimization
- **Efficient Data Structures**: Uses HashMaps for O(1) operations
- **Minimal Allocations**: Reduces memory allocations
- **Register-based Operations**: Efficient register usage
- **Stack-based Execution**: Efficient stack operations

## Integration Points

### Compiler Integration
- **Binary Loading**: Loads compiled MSO binaries
- **IR Execution**: Executes MSO Intermediate Representation
- **Symbol Table**: Uses symbol table for entity resolution

### Runtime Integration
- **Configuration Loading**: Loads MSO configuration
- **Environment Variables**: Resolves environment variables
- **Memory Management**: Manages runtime memory

### Debugging Integration
- **Breakpoint System**: Integrates with debugger
- **State Inspection**: Provides VM state access
- **Execution Control**: Supports step-through debugging

## Testing Strategy

### Unit Tests
- **Stack Operations**: Test push/pop operations
- **Memory Operations**: Test load/store operations
- **Instruction Execution**: Test individual instructions
- **Error Handling**: Test error scenarios

### Integration Tests
- **Binary Execution**: Test complete binary execution
- **Debugging**: Test debugging functionality
- **Performance**: Test performance characteristics
- **Error Recovery**: Test error recovery scenarios

### Test Data
- **Sample Binaries**: Various MSO binary configurations
- **Error Cases**: Invalid binaries and runtime errors
- **Performance Data**: Large binaries for performance testing
- **Debug Scenarios**: Complex debugging situations

## Future Enhancements

### Planned Features
- **Garbage Collection**: Automatic memory management
- **Parallel Execution**: Multi-threaded instruction execution
- **JIT Compilation**: Just-in-time compilation for performance
- **Hot Reloading**: Runtime code updates
- **Profiling**: Advanced performance profiling

### Performance Improvements
- **Instruction Caching**: Cache frequently used instructions
- **Memory Pooling**: Reduce memory allocations
- **Register Optimization**: Optimize register usage
- **Stack Optimization**: Optimize stack operations
