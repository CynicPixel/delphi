//parser/mod.rs
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

    // Split line at '='
    let (left, right) = match line.find('=') {
        Some(eq) => (&line[..eq].trim(), &line[eq+1..].trim()),
        None => return Ok(var_ids),
    };

    // Output variable (left)
    let output_re = Regex::new(r"([nx])(\d+)").unwrap();
    if let Some(cap) = output_re.captures(left) {
        let prefix = &cap[1];
        let id: i32 = cap[2].parse().unwrap();
        let var_id = if prefix == "x" { MAX_GATES as i32 + id } else { id };
        var_ids.push(var_id);
    }

    // Input variables (right), preserve order!
    for cap in output_re.captures_iter(right) {
        let prefix = &cap[1];
        let id: i32 = cap[2].parse().unwrap();
        let var_id = if prefix == "x" { MAX_GATES as i32 + id } else { id };
        var_ids.push(var_id);
    }

    Ok(var_ids)
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