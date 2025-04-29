mod parsers;

use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};
use anyhow::{Result, Context, bail};
use regex::Regex;

use crate::{Circuit, TableGate, MAX_GATES, OUT_BIAS};

pub use self::parsers::*;

pub fn parse_netlist<P: AsRef<Path>>(path: P, circuit: &mut Circuit) -> Result<()> {
    let file = File::open(path.as_ref())
        .context(format!("Failed to open file: {:?}", path.as_ref()))?;
    
    let reader = BufReader::new(file);
    let mut temp_var = 1;
    
    // Extract benchmark name from path
    let file_name = path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    
    // Remove extension
    let bench_name = if let Some(pos) = file_name.rfind('.') {
        &file_name[..pos]
    } else {
        file_name
    };
    
    circuit.bench_name = bench_name.to_string();
    
    // Initialize the input count to 0
    circuit.num_inputs = 0;
    
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        
        if line.starts_with('.') {
            break;
        }
        
        let var_ids = extract_variables(&line)?;
        
        match var_ids.len() {
            2 => {
                // NOT gate
                let mut gate = TableGate::default();
                gate.fanin = 1;
                gate.out = if var_ids[0] >= OUT_BIAS as i32 { var_ids[0] - OUT_BIAS as i32 } else { var_ids[0] };
                gate.inputs[0] = if var_ids[1] >= OUT_BIAS as i32 { var_ids[1] - OUT_BIAS as i32 } else { var_ids[1] };
                
                if var_ids[0] >= OUT_BIAS as i32 {
                    gate.is_output = true;
                    circuit.num_outputs += 1;
                }
                
                circuit.gates.push(gate);
                circuit.num_gates += 1;
            },
            3 => {
                // 2-input NOR
                let mut gate = TableGate::default();
                gate.fanin = 2;
                gate.out = if var_ids[0] >= OUT_BIAS as i32 { var_ids[0] - OUT_BIAS as i32 } else { var_ids[0] };
                gate.inputs[0] = if var_ids[1] >= OUT_BIAS as i32 { var_ids[1] - OUT_BIAS as i32 } else { var_ids[1] };
                gate.inputs[1] = if var_ids[2] >= OUT_BIAS as i32 { var_ids[2] - OUT_BIAS as i32 } else { var_ids[2] };
                
                if var_ids[0] >= OUT_BIAS as i32 {
                    gate.is_output = true;
                    circuit.num_outputs += 1;
                }
                
                circuit.gates.push(gate);
                circuit.num_gates += 1;
            },
            4 => {
                // Two 2-input NOR gates in cascade
                let mut gate1 = TableGate::default();
                gate1.fanin = 2;
                gate1.out = -(temp_var);
                gate1.inputs[0] = var_ids[2];
                gate1.inputs[1] = var_ids[3];
                
                circuit.gates.push(gate1);
                circuit.num_gates += 1;
                
                let mut gate2 = TableGate::default();
                gate2.fanin = 2;
                gate2.out = var_ids[0];
                gate2.inputs[0] = var_ids[1];
                gate2.inputs[1] = -(temp_var);
                
                circuit.gates.push(gate2);
                circuit.num_gates += 1;
                
                temp_var += 1;
            },
            5 => {
                // Three 2-input NOR gates in two levels
                let mut gate1 = TableGate::default();
                gate1.fanin = 2;
                gate1.out = -(temp_var);
                gate1.inputs[0] = var_ids[1];
                gate1.inputs[1] = var_ids[2];
                
                circuit.gates.push(gate1);
                circuit.num_gates += 1;
                
                let mut gate2 = TableGate::default();
                gate2.fanin = 2;
                gate2.out = -(temp_var + 1);
                gate2.inputs[0] = var_ids[3];
                gate2.inputs[1] = var_ids[4];
                
                circuit.gates.push(gate2);
                circuit.num_gates += 1;
                
                let mut gate3 = TableGate::default();
                gate3.fanin = 2;
                gate3.out = var_ids[0];
                gate3.inputs[0] = -(temp_var);
                gate3.inputs[1] = -(temp_var + 1);
                
                circuit.gates.push(gate3);
                circuit.num_gates += 1;
                
                temp_var += 2;
            },
            _ => {
                bail!("Invalid number of variables in line: {}", line);
            }
        }
    }
    
    // Initialize gate levels
    for gate in &mut circuit.gates {
        gate.asap_level = -1;
        gate.alap_level = -1;
        gate.list_level = -1;
    }
    
    Ok(())
}

fn extract_variables(line: &str) -> Result<Vec<i32>> {
    let mut var_ids = Vec::new();
    
    // Count variables first
    let var_count = count_variables(line);
    
    if var_count == 0 {
        return Ok(var_ids);
    }
    
    // Create improved regexes to extract variables
    let x_regex = Regex::new(r"x(\d+)").unwrap();
    let n_regex = Regex::new(r"n(\d+)").unwrap();
    
    // Split the line into left and right sides
    if let Some(equals_pos) = line.find('=') {
        let left_side = &line[0..equals_pos].trim();
        let right_side = &line[equals_pos+1..].trim();
        
        // Process the left-hand side (output variable)
        let mut left_var_ids = Vec::new();
        for cap in n_regex.captures_iter(left_side) {
            if let Some(id_match) = cap.get(1) {
                if let Ok(id) = id_match.as_str().parse::<i32>() {
                    left_var_ids.push(id);
                }
            }
        }
        
        if !left_var_ids.is_empty() {
            var_ids.push(left_var_ids[0]);
        }
        
        // Process the right-hand side (input variables)
        for cap in x_regex.captures_iter(right_side) {
            if let Some(id_match) = cap.get(1) {
                if let Ok(id) = id_match.as_str().parse::<i32>() {
                    var_ids.push(MAX_GATES as i32 + id);
                }
            }
        }
        
        // Process internal n variables on the right side
        for cap in n_regex.captures_iter(right_side) {
            if let Some(id_match) = cap.get(1) {
                if let Ok(id) = id_match.as_str().parse::<i32>() {
                    var_ids.push(id);
                }
            }
        }
    }
    
    Ok(var_ids)
}

fn count_variables(line: &str) -> usize {
    let mut count = 0;
    
    for c in line.chars() {
        if c == 'n' || c == 'x' {
            count += 1;
        }
    }
    
    count
}

pub fn find_primary_inputs(circuit: &mut Circuit) {
    // Reset primary inputs
    circuit.num_inputs = 0;
    
    // First, find the highest input number to determine the total count
    let mut max_input_num = 0;
    
    for k in 0..circuit.num_gates {
        for j in 0..circuit.gates[k].fanin {
            if circuit.gates[k].inputs[j] >= MAX_GATES as i32 {
                let input_num = circuit.gates[k].inputs[j] - MAX_GATES as i32;
                if input_num > max_input_num {
                    max_input_num = input_num;
                }
            }
        }
    }
    
    // Set the number of inputs
    circuit.num_inputs = (max_input_num + 1) as usize;
    
    // Now create a mapping of all found inputs
    for k in 0..circuit.num_gates {
        for j in 0..circuit.gates[k].fanin {
            if circuit.gates[k].inputs[j] >= MAX_GATES as i32 {
                let idx = (circuit.gates[k].inputs[j] - MAX_GATES as i32) as usize;
                circuit.primary_inputs[idx] = circuit.gates[k].inputs[j];
            }
        }
    }
}

pub fn find_gate_outputs(circuit: &mut Circuit) {
    for i in 0..circuit.num_gates {
        for k in 0..circuit.num_gates {
            if k == i {
                continue;
            }
            
            for j in 0..circuit.gates[k].fanin {
                let current_input = circuit.gates[k].inputs[j];
                
                if circuit.gates[i].out == current_input {
                    let output_count = circuit.gates[i].output_gates.iter()
                        .position(|&x| x == 0)
                        .unwrap_or(0);
                    
                    circuit.gates[i].output_gates[output_count] = circuit.gates[k].out;
                }
            }
        }
    }
}