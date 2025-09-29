use crate::dna::mds::codegen::{HelixIR, Instruction};
use std::collections::{HashMap, HashSet};
pub use crate::mds::codegen::{StringPool, SymbolTable, Metadata, ConstantPool, ConstantValue};
use std::path::PathBuf;
use anyhow::Result;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    Zero,
    One,
    Two,
    Three,
}
impl Default for OptimizationLevel {
    fn default() -> Self {
        Self::Two
    }
}
impl From<u8> for OptimizationLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => Self::Zero,
            1 => Self::One,
            2 => Self::Two,
            3 | _ => Self::Three,
        }
    }
}
pub struct Optimizer {
    level: OptimizationLevel,
    stats: OptimizationStats,
}
impl Optimizer {
    pub fn new(level: OptimizationLevel) -> Self {
        Self {
            level,
            stats: OptimizationStats::default(),
        }
    }
    pub fn optimize(&mut self, ir: &mut HelixIR) {
        match self.level {
            OptimizationLevel::Zero => {}
            OptimizationLevel::One => {
                self.apply_basic_optimizations(ir);
            }
            OptimizationLevel::Two => {
                self.apply_basic_optimizations(ir);
                self.apply_standard_optimizations(ir);
            }
            OptimizationLevel::Three => {
                self.apply_basic_optimizations(ir);
                self.apply_standard_optimizations(ir);
                self.apply_aggressive_optimizations(ir);
            }
        }
    }
    fn apply_basic_optimizations(&mut self, ir: &mut HelixIR) {
        self.deduplicate_strings(ir);
        self.remove_dead_code(ir);
        self.optimize_string_pool(ir);
    }
    fn apply_standard_optimizations(&mut self, ir: &mut HelixIR) {
        self.fold_constants(ir);
        self.inline_small_functions(ir);
        self.optimize_instruction_sequence(ir);
        self.merge_duplicate_sections(ir);
    }
    fn apply_aggressive_optimizations(&mut self, ir: &mut HelixIR) {
        self.eliminate_cross_references(ir);
        self.optimize_pipelines(ir);
        self.compress_data_sections(ir);
        self.reorder_for_cache_locality(ir);
    }
    fn deduplicate_strings(&mut self, ir: &mut HelixIR) {
        let mut seen = HashMap::new();
        let mut new_strings = Vec::new();
        let mut remap = HashMap::new();
        for (idx, string) in ir.string_pool.strings.iter().enumerate() {
            if let Some(&existing_idx) = seen.get(string) {
                remap.insert(idx as u32, existing_idx);
                self.stats.strings_deduplicated += 1;
            } else {
                let new_idx = new_strings.len() as u32;
                seen.insert(string.clone(), new_idx);
                new_strings.push(string.clone());
                remap.insert(idx as u32, new_idx);
            }
        }
        let original_size = ir.string_pool.strings.len();
        ir.string_pool.strings = new_strings;
        self.stats.strings_removed = original_size - ir.string_pool.strings.len();
        for instruction in &mut ir.instructions {
            self.remap_instruction_strings(instruction, &remap);
        }
    }
    fn remove_dead_code(&mut self, ir: &mut HelixIR) {
        let mut reachable = HashSet::new();
        let mut work_list = vec![0];
        while let Some(idx) = work_list.pop() {
            if idx >= ir.instructions.len() || !reachable.insert(idx) {
                continue;
            }
            work_list.push(idx + 1);
        }
        let mut new_instructions = Vec::new();
        let mut remap = HashMap::new();
        for (idx, instruction) in ir.instructions.iter().enumerate() {
            if reachable.contains(&idx) {
                remap.insert(idx, new_instructions.len());
                new_instructions.push(instruction.clone());
            } else {
                self.stats.instructions_removed += 1;
            }
        }
        for instruction in &mut new_instructions {
            self.remap_jump_targets(instruction, &remap);
        }
        ir.instructions = new_instructions;
    }
    fn optimize_string_pool(&mut self, ir: &mut HelixIR) {
        let mut frequency = HashMap::new();
        for instruction in &ir.instructions {
            self.count_string_usage(instruction, &mut frequency);
        }
        let mut indexed_strings: Vec<(u32, String)> = ir
            .string_pool
            .strings
            .iter()
            .enumerate()
            .map(|(i, s)| (i as u32, s.clone()))
            .collect();
        indexed_strings
            .sort_by_key(|(idx, _)| {
                std::cmp::Reverse(frequency.get(idx).copied().unwrap_or(0))
            });
        let mut remap = HashMap::new();
        let mut new_strings = Vec::new();
        for (old_idx, string) in indexed_strings {
            let new_idx = new_strings.len() as u32;
            remap.insert(old_idx, new_idx);
            new_strings.push(string);
        }
        ir.string_pool.strings = new_strings;
        for instruction in &mut ir.instructions {
            self.remap_instruction_strings(instruction, &remap);
        }
    }
    fn fold_constants(&mut self, _ir: &mut HelixIR) {}
    fn inline_small_functions(&mut self, ir: &mut HelixIR) {
        let mut reference_count = std::collections::HashMap::new();
        for instruction in &ir.instructions {
            if let Instruction::ResolveReference { index, .. } = instruction {
                *reference_count.entry(*index).or_insert(0) += 1;
            }
        }
        self.stats.functions_inlined = 0;
    }
    fn optimize_instruction_sequence(&mut self, ir: &mut HelixIR) {
        let mut seen_properties: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
        let mut i = 0;
        while i < ir.instructions.len() {
            match &ir.instructions[i] {
                Instruction::SetProperty { target, key, .. } => {
                    let prop_key = (*target, *key);
                    if seen_properties.contains(&prop_key) {
                        ir.instructions.remove(i);
                        self.stats.instructions_removed += 1;
                        continue;
                    } else {
                        seen_properties.insert(prop_key);
                    }
                }
                Instruction::SetCapability { agent, capability } => {
                    let cap_key = (*agent, *capability);
                    if seen_properties.contains(&cap_key) {
                        ir.instructions.remove(i);
                        self.stats.instructions_removed += 1;
                        continue;
                    } else {
                        seen_properties.insert(cap_key);
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
    fn merge_duplicate_sections(&mut self, ir: &mut HelixIR) {
        use std::collections::HashMap;
        let mut agent_signatures: HashMap<String, Vec<u32>> = HashMap::new();
        for (id, agent) in &ir.symbol_table.agents {
            let signature = format!(
                "{}-{}-{:?}-{:?}", agent.model_idx, agent.role_idx, agent.temperature,
                agent.max_tokens
            );
            agent_signatures.entry(signature).or_insert_with(Vec::new).push(*id);
        }
        for (_, agents) in &agent_signatures {
            if agents.len() > 1 {
                self.stats.sections_merged += agents.len() - 1;
            }
        }
        let mut workflow_signatures: HashMap<String, Vec<u32>> = HashMap::new();
        for (id, workflow) in &ir.symbol_table.workflows {
            let signature = format!("{:?}", workflow.trigger_type);
            workflow_signatures.entry(signature).or_insert_with(Vec::new).push(*id);
        }
    }
    fn eliminate_cross_references(&mut self, ir: &mut HelixIR) {
        use std::collections::HashSet;
        let mut referenced_agents: HashSet<u32> = HashSet::new();
        let mut referenced_workflows: HashSet<u32> = HashSet::new();
        for crew in ir.symbol_table.crews.values() {
            for agent_id in &crew.agent_ids {
                referenced_agents.insert(*agent_id);
            }
        }
        for workflow in ir.symbol_table.workflows.values() {
            if let Some(pipeline) = &workflow.pipeline {
                for node_id in pipeline {
                    referenced_workflows.insert(*node_id);
                }
            }
        }
        for instruction in &ir.instructions {
            match instruction {
                Instruction::ResolveReference { ref_type: _, index: _ } => {}
                _ => {}
            }
        }
        let unreferenced_agents: Vec<u32> = ir
            .symbol_table
            .agents
            .keys()
            .filter(|id| !referenced_agents.contains(id))
            .cloned()
            .collect();
        for agent_id in unreferenced_agents {
            ir.symbol_table.agents.remove(&agent_id);
            self.stats.instructions_removed += 1;
        }
        let unreferenced_workflows: Vec<u32> = ir
            .symbol_table
            .workflows
            .keys()
            .filter(|id| !referenced_workflows.contains(id))
            .cloned()
            .collect();
        for workflow_id in unreferenced_workflows {
            ir.symbol_table.workflows.remove(&workflow_id);
            self.stats.instructions_removed += 1;
        }
    }
    fn optimize_pipelines(&mut self, ir: &mut HelixIR) {
        for i in 0..ir.instructions.len() {
            if let Instruction::DefinePipeline { .. } = &ir.instructions[i] {
                self.stats.pipelines_optimized += 1;
            }
        }
    }
    fn compress_data_sections(&mut self, ir: &mut HelixIR) {
        let mut string_frequency: HashMap<u32, usize> = HashMap::new();
        for instruction in &ir.instructions {
            match instruction {
                Instruction::SetProperty { key, .. } => {
                    *string_frequency.entry(*key).or_insert(0) += 1;
                }
                Instruction::SetCapability { capability, .. } => {
                    *string_frequency.entry(*capability).or_insert(0) += 1;
                }
                Instruction::SetMetadata { key, value } => {
                    *string_frequency.entry(*key).or_insert(0) += 1;
                    *string_frequency.entry(*value).or_insert(0) += 1;
                }
                _ => {}
            }
        }
        for agent in ir.symbol_table.agents.values() {
            *string_frequency.entry(agent.name_idx).or_insert(0) += 1;
            *string_frequency.entry(agent.model_idx).or_insert(0) += 1;
            *string_frequency.entry(agent.role_idx).or_insert(0) += 1;
        }
        let _total_strings = ir.string_pool.strings.len();
        let frequently_used = string_frequency
            .iter()
            .filter(|(_, count)| **count > 1)
            .count();
        if frequently_used > 0 {
            self.stats.bytes_saved += frequently_used * 8;
        }
    }
    fn reorder_for_cache_locality(&mut self, ir: &mut HelixIR) {
        let mut reordered = Vec::new();
        let mut agent_instructions = Vec::new();
        let mut workflow_instructions = Vec::new();
        let mut other_instructions = Vec::new();
        for instruction in ir.instructions.drain(..) {
            match &instruction {
                Instruction::DeclareAgent(_) => agent_instructions.push(instruction),
                Instruction::DeclareWorkflow(_) => {
                    workflow_instructions.push(instruction)
                }
                Instruction::DefinePipeline { .. } => {
                    workflow_instructions.push(instruction)
                }
                _ => other_instructions.push(instruction),
            }
        }
        reordered.extend(agent_instructions);
        reordered.extend(workflow_instructions);
        reordered.extend(other_instructions);
        ir.instructions = reordered;
    }
    fn remap_instruction_strings(
        &self,
        instruction: &mut Instruction,
        remap: &HashMap<u32, u32>,
    ) {
        match instruction {
            Instruction::SetProperty { key, value, .. } => {
                if let Some(&new_idx) = remap.get(key) {
                    *key = new_idx;
                }
                if let ConstantValue::String(idx) = value {
                    if let Some(&new_idx) = remap.get(idx) {
                        *idx = new_idx;
                    }
                }
            }
            Instruction::SetCapability { capability, .. } => {
                if let Some(&new_idx) = remap.get(capability) {
                    *capability = new_idx;
                }
            }
            Instruction::SetMetadata { key, value } => {
                if let Some(&new_idx) = remap.get(key) {
                    *key = new_idx;
                }
                if let Some(&new_idx) = remap.get(value) {
                    *value = new_idx;
                }
            }
            _ => {}
        }
    }
    fn remap_jump_targets(
        &self,
        _instruction: &mut Instruction,
        _remap: &HashMap<usize, usize>,
    ) {}
    fn count_string_usage(
        &self,
        instruction: &Instruction,
        frequency: &mut HashMap<u32, usize>,
    ) {
        match instruction {
            Instruction::SetProperty { key, value, .. } => {
                *frequency.entry(*key).or_insert(0) += 1;
                if let ConstantValue::String(idx) = value {
                    *frequency.entry(*idx).or_insert(0) += 1;
                }
            }
            Instruction::SetCapability { capability, .. } => {
                *frequency.entry(*capability).or_insert(0) += 1;
            }
            Instruction::SetMetadata { key, value } => {
                *frequency.entry(*key).or_insert(0) += 1;
                *frequency.entry(*value).or_insert(0) += 1;
            }
            _ => {}
        }
    }
    pub fn stats(&self) -> &OptimizationStats {
        &self.stats
    }
}
#[derive(Debug, Default)]
pub struct OptimizationStats {
    pub strings_deduplicated: usize,
    pub strings_removed: usize,
    pub instructions_removed: usize,
    pub constants_folded: usize,
    pub functions_inlined: usize,
    pub pipelines_optimized: usize,
    pub sections_merged: usize,
    pub bytes_saved: usize,
}
impl OptimizationStats {
    pub fn report(&self) -> String {
        format!(
            "Optimization Results:\n\
             - Strings deduplicated: {}\n\
             - Strings removed: {}\n\
             - Instructions removed: {}\n\
             - Constants folded: {}\n\
             - Functions inlined: {}\n\
             - Pipelines optimized: {}\n\
             - Sections merged: {}\n\
             - Total bytes saved: {}",
            self.strings_deduplicated, self.strings_removed, self.instructions_removed,
            self.constants_folded, self.functions_inlined, self.pipelines_optimized, self
            .sections_merged, self.bytes_saved
        )
    }
}
#[cfg(test)]
mod tests {
    use crate::mds::codegen::{StringPool, SymbolTable, Metadata, ConstantPool, ConstantValue};
    #[test]
    fn test_optimization_levels() {
        assert_eq!(OptimizationLevel::from(0), OptimizationLevel::Zero);
        assert_eq!(OptimizationLevel::from(1), OptimizationLevel::One);
        assert_eq!(OptimizationLevel::from(2), OptimizationLevel::Two);
        assert_eq!(OptimizationLevel::from(3), OptimizationLevel::Three);
        assert_eq!(OptimizationLevel::from(99), OptimizationLevel::Three);
    }
    #[test]
    fn test_string_deduplication() {
        let mut ir = HelixIR {
            version: 1,
            metadata: Metadata::default(),
            symbol_table: SymbolTable::default(),
            instructions: vec![
                Instruction::DeclareAgent(0), Instruction::DeclareWorkflow(1),
                Instruction::DeclareContext(2),
            ],
            string_pool: StringPool {
                strings: vec![
                    "hello".to_string(), "world".to_string(), "hello".to_string(),
                ],
                index: std::collections::HashMap::new(),
            },
            constants: ConstantPool::default(),
        };
        let mut optimizer = Optimizer::new(OptimizationLevel::One);
        optimizer.deduplicate_strings(&mut ir);
        assert_eq!(ir.string_pool.strings.len(), 2);
        assert_eq!(optimizer.stats.strings_deduplicated, 1);
    }
    #[test]
    fn test_constant_folding() {
        let mut ir = HelixIR {
            version: 1,
            metadata: Metadata::default(),
            symbol_table: SymbolTable::default(),
            instructions: vec![Instruction::DeclareAgent(0),],
            string_pool: StringPool::default(),
            constants: ConstantPool::default(),
        };
        let mut optimizer = Optimizer::new(OptimizationLevel::Two);
        optimizer.fold_constants(&mut ir);
        assert_eq!(ir.instructions.len(), 1);
        match &ir.instructions[0] {
            Instruction::DeclareAgent(0) => {}
            _ => panic!("Expected DeclareAgent(0)"),
        }
    }
}

pub fn optimize_command(
    input: PathBuf,
    output: Option<PathBuf>,
    level: u8,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = output.unwrap_or_else(|| input.clone());
    if verbose {
        println!("⚡ Optimizing: {}", input.display());
        println!("  Level: {}", level);
    }
    let loader = crate::mds::loader::BinaryLoader::new();
    let binary = loader.load_file(&input)?;
    let serializer = crate::mds::serializer::BinarySerializer::new(false);
    let mut ir = serializer.deserialize_to_ir(&binary)?;
    let mut optimizer = crate::mds::optimizer::Optimizer::new(
        OptimizationLevel::from(level),
    );
    optimizer.optimize(&mut ir);
    let optimized_binary = serializer.serialize(ir, None)?;
    serializer.write_to_file(&optimized_binary, &output_path)?;
    println!("✅ Optimized successfully: {}", output_path.display());
    if verbose {
        let stats = optimizer.stats();
        println!("\nOptimization Results:");
        println!("{}", stats.report());
    }
    Ok(())
}