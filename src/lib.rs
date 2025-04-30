//lib.rs
pub mod parser;
pub mod scheduler;
pub mod mapper;
pub mod generator;
// Stub module to keep compatibility
pub mod parallel {
    use crate::Circuit;
    use anyhow::Result;
    use log::warn;
    
    // Stub functions that should never be called
    pub fn find_primary_inputs_parallel(_circuit: &mut Circuit) -> Result<()> {
        warn!("Parallel module function called but not implemented");
        Ok(())
    }

    pub fn compute_asap_schedule_parallel(_circuit: &mut Circuit) -> Result<()> {
        warn!("Parallel module function called but not implemented");
        Ok(())
    }
    
    pub fn compute_alap_schedule_parallel(_circuit: &mut Circuit) -> Result<()> {
        warn!("Parallel module function called but not implemented");
        Ok(())
    }
    
    pub fn compute_list_schedule_parallel(_circuit: &mut Circuit) -> Result<()> {
        warn!("Parallel module function called but not implemented");
        Ok(())
    }

    pub fn create_naive_mapping_parallel(_circuit: &mut Circuit) -> Result<crate::CrossbarMapping> {
        warn!("Parallel module function called but not implemented");
        Ok(crate::CrossbarMapping::new())
    }
    
    pub fn create_compact_mapping_parallel(_circuit: &mut Circuit) -> Result<crate::CrossbarMapping> {
        warn!("Parallel module function called but not implemented");
        Ok(crate::CrossbarMapping::new())
    }
}

// use std::collections::HashMap;
// use std::sync::Arc;
// use parking_lot::{RwLock, Mutex};

pub const MAX_GATES: usize = 8000;     // Good for circuits up to 8000 gates
pub const MAX_FANIN: usize = 5;       // Maximum fanin of gates
pub const MAX_LEVEL: usize = 500;     // Maximum level in schedule
pub const MAX_PI: usize = 1000;       // Maximum primary inputs
pub const MAX_GATES_LEVEL: usize = 1000; // Maximum gates per level
pub const MAX_LEVELS: usize = 500;    // Maximum levels in schedule
pub const MAX_ROW: usize = 500;       // Maximum rows in crossbar
pub const MAX_COL: usize = 1000;      // Maximum columns in crossbar
pub const MAX_CPY: usize = 100;       // Maximum copies
pub const OUT_BIAS: usize = 10000;    // Output bias

// Determine optimal chunk size for parallel processing based on problem size
pub fn calculate_chunk_size(total_items: usize) -> usize {
    let num_threads = num_cpus::get();
    let base_chunk = total_items / num_threads;
    if base_chunk < 8 {
        // For small problems, avoid excessive threading overhead
        return total_items;
    }
    // Aim for at least 8 items per chunk, max 1000 items
    base_chunk.max(8).min(1000)
}

#[derive(Debug, Clone)]
pub struct MemristiveGate {
    pub fanin: usize,
    pub inputs: Vec<Option<Box<MemristiveGate>>>,
    pub value: i32,
    pub idx: i32,
    pub jdx: i32,
    pub state: i32,
    pub asap_level: i32,
    pub list_time: i32,
    pub is_copy: bool,
}

impl Default for MemristiveGate {
    fn default() -> Self {
        Self {
            fanin: 0,
            inputs: vec![None; MAX_FANIN],
            value: -1,
            idx: -1,
            jdx: -1,
            state: -1,
            asap_level: -1,
            list_time: -1,
            is_copy: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableGate {
    pub fanin: usize,
    pub inputs: Vec<i32>,
    pub out: i32,
    pub asap_level: i32,
    pub alap_level: i32,
    pub list_level: i32,
    pub list_time: i32,
    pub mobility: i32,
    pub slack: i32,
    pub output_gates: Vec<i32>,
    pub is_output: bool,
    pub gate_map: Option<Box<MemristiveGate>>,
}

impl Default for TableGate {
    fn default() -> Self {
        Self {
            fanin: 0,
            inputs: vec![-1; MAX_FANIN],
            out: -1,
            asap_level: -1,
            alap_level: -1,
            list_level: -1,
            list_time: -1,
            mobility: 0,
            slack: 0,
            output_gates: vec![0; MAX_GATES],
            is_output: false,
            gate_map: None,
        }
    }
}

#[derive(Default, Debug)]
pub struct Circuit {
    pub gates: Vec<TableGate>,
    pub num_gates: usize,
    pub primary_inputs: Vec<i32>,
    pub num_inputs: usize,
    pub num_outputs: usize,
    pub max_asap: i32,
    pub max_alap: i32,
    pub max_list: i32,
    pub max_resources: i32,
    pub bench_name: String,
}

impl Circuit {
    pub fn new() -> Self {
        Self {
            gates: Vec::with_capacity(MAX_GATES),
            num_gates: 0,
            primary_inputs: vec![0; MAX_PI],
            num_inputs: 0,
            num_outputs: 0,
            max_asap: 0,
            max_alap: 0,
            max_list: 0,
            max_resources: 0,
            bench_name: String::new(),
        }
    }
}

#[derive(Default, Debug)]
pub struct CrossbarMapping {
    pub crossbar: Vec<Vec<MemristiveGate>>,
    pub max_idx: i32,
    pub max_jdx: i32,
}

impl CrossbarMapping {
    pub fn new() -> Self {
        let mut crossbar = Vec::with_capacity(MAX_ROW);
        for _ in 0..MAX_ROW {
            let row = vec![MemristiveGate::default(); MAX_COL];
            crossbar.push(row);
        }
        
        Self {
            crossbar,
            max_idx: 0,
            max_jdx: 0,
        }
    }
}