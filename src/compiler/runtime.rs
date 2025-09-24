use std::collections::{HashMap, VecDeque};
use std::path::Path;
use crate::compiler::binary::{HelixBinary, Value};
use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::types::HelixConfig;
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
#[derive(Debug, Default)]
pub struct VMRegisters {
    pub program_counter: usize,
    pub stack_pointer: usize,
    pub frame_pointer: usize,
    pub return_address: usize,
    pub flags: VMFlags,
}
#[derive(Debug, Default)]
pub struct VMFlags {
    pub zero: bool,
    pub overflow: bool,
    pub error: bool,
    pub halted: bool,
}
#[derive(Debug)]
pub struct CallFrame {
    pub return_address: usize,
    pub frame_pointer: usize,
    pub local_vars: HashMap<u32, Value>,
}
#[derive(Debug, PartialEq)]
pub enum ExecutionState {
    Ready,
    Running,
    Paused,
    Halted,
    Error(String),
}
#[derive(Debug)]
pub struct Breakpoint {
    pub active: bool,
    pub condition: Option<String>,
    pub hit_count: usize,
}
pub type VMResult<T> = Result<T, RuntimeError>;
impl HelixVM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            memory: HashMap::new(),
            registers: VMRegisters::default(),
            config: HelixConfig::default(),
            call_stack: VecDeque::new(),
            execution_state: ExecutionState::Ready,
            debug_mode: false,
            breakpoints: HashMap::new(),
        }
    }
    pub fn with_debug(mut self) -> Self {
        self.debug_mode = true;
        self
    }
    pub fn execute_binary(&mut self, binary: &HelixBinary) -> VMResult<HelixConfig> {
        let serializer = crate::compiler::serializer::BinarySerializer::new(false);
        let ir = serializer
            .deserialize_to_ir(binary)
            .map_err(|e| RuntimeError {
                kind: RuntimeErrorKind::InvalidInstruction,
                message: format!("Failed to deserialize binary: {}", e),
                stack_trace: vec![],
            })?;
        self.execution_state = ExecutionState::Running;
        self.registers.program_counter = 0;
        while self.registers.program_counter < ir.instructions.len()
            && self.execution_state == ExecutionState::Running
        {
            if self.debug_mode {
                if let Some(bp) = self
                    .breakpoints
                    .get_mut(&self.registers.program_counter)
                {
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
        Ok(self.config.clone())
    }
    fn execute_instruction(
        &mut self,
        instruction: &crate::codegen::Instruction,
    ) -> VMResult<()> {
        use crate::codegen::Instruction as IR;
        match instruction {
            IR::DeclareAgent(id) => {
                self.declare_agent(*id)?;
            }
            IR::DeclareWorkflow(id) => {
                self.declare_workflow(*id)?;
            }
            IR::DeclareContext(id) => {
                self.declare_context(*id)?;
            }
            IR::DeclareCrew(id) => {
                self.declare_crew(*id)?;
            }
            IR::SetProperty { target, key, value } => {
                self.set_property(*target, *key, value)?;
            }
            IR::SetCapability { agent, capability } => {
                self.set_capability(*agent, *capability)?;
            }
            IR::SetSecret { context, key, secret } => {
                self.set_secret(*context, *key, secret)?;
            }
            IR::DefineStep { workflow, step } => {
                self.define_step(*workflow, step)?;
            }
            IR::DefinePipeline { workflow, nodes } => {
                self.define_pipeline(*workflow, nodes)?;
            }
            IR::ResolveReference { ref_type, index } => {
                self.resolve_reference(ref_type, *index)?;
            }
            IR::SetMetadata { key, value } => {
                self.set_metadata(*key, *value)?;
            }
        }
        self.registers.program_counter += 1;
        Ok(())
    }
    fn declare_agent(&mut self, _id: u32) -> VMResult<()> {
        Ok(())
    }
    fn declare_workflow(&mut self, _id: u32) -> VMResult<()> {
        Ok(())
    }
    fn declare_context(&mut self, _id: u32) -> VMResult<()> {
        Ok(())
    }
    fn declare_crew(&mut self, _id: u32) -> VMResult<()> {
        Ok(())
    }
    fn set_property(
        &mut self,
        _target: u32,
        _key: u32,
        _value: &crate::codegen::ConstantValue,
    ) -> VMResult<()> {
        Ok(())
    }
    fn set_capability(&mut self, _agent: u32, _capability: u32) -> VMResult<()> {
        Ok(())
    }
    fn set_secret(
        &mut self,
        _context: u32,
        _key: u32,
        _secret: &crate::codegen::SecretType,
    ) -> VMResult<()> {
        Ok(())
    }
    fn define_step(
        &mut self,
        _workflow: u32,
        _step: &crate::codegen::StepDefinition,
    ) -> VMResult<()> {
        Ok(())
    }
    fn define_pipeline(
        &mut self,
        _workflow: u32,
        _nodes: &[crate::codegen::PipelineNodeIR],
    ) -> VMResult<()> {
        Ok(())
    }
    fn resolve_reference(
        &mut self,
        _ref_type: &crate::codegen::ReferenceType,
        _index: u32,
    ) -> VMResult<()> {
        Ok(())
    }
    fn set_metadata(&mut self, _key: u32, _value: u32) -> VMResult<()> {
        Ok(())
    }
    pub fn push(&mut self, value: Value) -> VMResult<()> {
        if self.stack.len() >= 1024 {
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
    pub fn load_memory(&self, address: u32) -> VMResult<&Value> {
        self.memory
            .get(&address)
            .ok_or_else(|| RuntimeError {
                kind: RuntimeErrorKind::MemoryAccessViolation,
                message: format!("Invalid memory access at address {}", address),
                stack_trace: self.get_stack_trace(),
            })
    }
    pub fn store_memory(&mut self, address: u32, value: Value) -> VMResult<()> {
        self.memory.insert(address, value);
        Ok(())
    }
    pub fn set_breakpoint(&mut self, address: usize) {
        self.breakpoints
            .insert(
                address,
                Breakpoint {
                    active: true,
                    condition: None,
                    hit_count: 0,
                },
            );
    }
    pub fn remove_breakpoint(&mut self, address: usize) {
        self.breakpoints.remove(&address);
    }
    pub fn continue_execution(&mut self) {
        if self.execution_state == ExecutionState::Paused {
            self.execution_state = ExecutionState::Running;
        }
    }
    pub fn step(&mut self) {
        if self.execution_state == ExecutionState::Paused {
            self.execution_state = ExecutionState::Running;
        }
    }
    pub fn state(&self) -> &ExecutionState {
        &self.execution_state
    }
    fn get_stack_trace(&self) -> Vec<String> {
        let mut trace = Vec::new();
        trace.push(format!("PC: {}", self.registers.program_counter));
        for (i, frame) in self.call_stack.iter().enumerate() {
            trace.push(format!("Frame {}: return address {}", i, frame.return_address));
        }
        trace
    }
    pub fn stats(&self) -> VMStats {
        VMStats {
            instructions_executed: self.registers.program_counter,
            stack_size: self.stack.len(),
            memory_usage: self.memory.len(),
            call_depth: self.call_stack.len(),
        }
    }
}
#[derive(Debug)]
pub struct VMStats {
    pub instructions_executed: usize,
    pub stack_size: usize,
    pub memory_usage: usize,
    pub call_depth: usize,
}
impl Default for HelixVM {
    fn default() -> Self {
        Self::new()
    }
}
pub struct VMExecutor {
    vm: HelixVM,
}
impl VMExecutor {
    pub fn new() -> Self {
        Self { vm: HelixVM::new() }
    }
    pub fn execute_file<P: AsRef<Path>>(&mut self, path: P) -> VMResult<HelixConfig> {
        let loader = crate::compiler::loader::BinaryLoader::new();
        let binary = loader
            .load_file(path)
            .map_err(|e| RuntimeError {
                kind: RuntimeErrorKind::ResourceNotFound,
                message: format!("Failed to load binary: {}", e),
                stack_trace: vec![],
            })?;
        self.vm.execute_binary(&binary)
    }
    pub fn execute_with_debug<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> VMResult<HelixConfig> {
        self.vm = HelixVM::new().with_debug();
        self.execute_file(path)
    }
    pub fn vm(&mut self) -> &mut HelixVM {
        &mut self.vm
    }
}
impl Default for VMExecutor {
    fn default() -> Self {
        Self::new()
    }
}
pub struct VMConfig {
    pub max_stack_size: usize,
    pub max_memory: usize,
    pub max_call_depth: usize,
    pub enable_gc: bool,
    pub gc_threshold: usize,
}
impl Default for VMConfig {
    fn default() -> Self {
        Self {
            max_stack_size: 1024,
            max_memory: 65536,
            max_call_depth: 256,
            enable_gc: false,
            gc_threshold: 1000,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_vm_creation() {
        let vm = HelixVM::new();
        assert_eq!(vm.execution_state, ExecutionState::Ready);
        assert!(vm.stack.is_empty());
        assert!(vm.memory.is_empty());
    }
    #[test]
    fn test_stack_operations() {
        let mut vm = HelixVM::new();
        vm.push(Value::Int(42)).unwrap();
        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.registers.stack_pointer, 1);
        let value = vm.pop().unwrap();
        match value {
            Value::Int(42) => {}
            _ => panic!("Expected Int(42)"),
        }
        assert!(vm.stack.is_empty());
        assert_eq!(vm.registers.stack_pointer, 0);
    }
    #[test]
    fn test_memory_operations() {
        let mut vm = HelixVM::new();
        vm.store_memory(100, Value::Bool(true)).unwrap();
        let value = vm.load_memory(100).unwrap();
        match value {
            Value::Bool(true) => {}
            _ => panic!("Expected Bool(true)"),
        }
    }
    #[test]
    fn test_stack_overflow() {
        let mut vm = HelixVM::new();
        for _ in 0..1024 {
            vm.push(Value::Int(1)).unwrap();
        }
        let result = vm.push(Value::Int(2));
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.kind, RuntimeErrorKind::StackOverflow);
        }
    }
    #[test]
    fn test_stack_underflow() {
        let mut vm = HelixVM::new();
        let result = vm.pop();
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.kind, RuntimeErrorKind::StackUnderflow);
        }
    }
    #[test]
    fn test_breakpoints() {
        let mut vm = HelixVM::new().with_debug();
        vm.set_breakpoint(10);
        assert!(vm.breakpoints.contains_key(& 10));
        vm.remove_breakpoint(10);
        assert!(! vm.breakpoints.contains_key(& 10));
    }
    #[test]
    fn test_vm_stats() {
        let vm = HelixVM::new();
        let stats = vm.stats();
        assert_eq!(stats.instructions_executed, 0);
        assert_eq!(stats.stack_size, 0);
        assert_eq!(stats.memory_usage, 0);
        assert_eq!(stats.call_depth, 0);
    }
}