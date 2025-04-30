//mapper/mod.rs
use std::cmp::max;
use std::collections::HashMap;

use crate::{Circuit, CrossbarMapping, MemristiveGate, MAX_GATES, MAX_ROW};

pub fn create_naive_mapping(circuit: &mut Circuit) -> CrossbarMapping {
    let mut mapping = CrossbarMapping::new();
    
    // Reset crossbar array
    for i in 0..MAX_ROW {
        for j in 0..mapping.crossbar[i].len() {
            mapping.crossbar[i][j].value = -1;
            mapping.crossbar[i][j].idx = -1;
            mapping.crossbar[i][j].jdx = -1;
            mapping.crossbar[i][j].asap_level = -1;
            mapping.crossbar[i][j].fanin = 0;
        }
    }
    
    // Reset gate mappings
    for i in 0..circuit.num_gates {
        circuit.gates[i].gate_map = None;
    }
    
    // Sort gates by ASAP level
    circuit.gates.sort_by(|a, b| a.asap_level.cmp(&b.asap_level));
    
    // Create inverse mapping for gate lookup by output
    let mut inv_map = HashMap::new();
    for i in 0..circuit.num_gates {
        inv_map.insert(circuit.gates[i].out, i);
    }
    
    // Handle case where there are no inputs
    if circuit.num_inputs == 0 {
        return mapping;
    }
    
    // Map primary inputs to the first row of the crossbar
    for j in 0..circuit.num_inputs {
        // Map each primary input to the crossbar
        mapping.crossbar[0][j].value = (MAX_GATES + j) as i32;
        mapping.crossbar[0][j].idx = 0;
        mapping.crossbar[0][j].jdx = j as i32;
    }
    
    // Update max_jdx to reflect the number of inputs
    mapping.max_jdx = (circuit.num_inputs - 1) as i32;
    
    // Map gates
    for i in 0..circuit.num_gates {
        // Increment the column index for the next gate
        mapping.max_jdx += 1;
        
        let ip1 = circuit.gates[i].inputs[0];
        let ip2 = if circuit.gates[i].fanin > 1 {
            circuit.gates[i].inputs[1]
        } else {
            -1
        };
        
        // Place the gate in the crossbar
        mapping.crossbar[0][mapping.max_jdx as usize].fanin = circuit.gates[i].fanin;
        mapping.crossbar[0][mapping.max_jdx as usize].value = circuit.gates[i].out;
        mapping.crossbar[0][mapping.max_jdx as usize].jdx = mapping.max_jdx;
        mapping.crossbar[0][mapping.max_jdx as usize].idx = 0; // All gates in row 0 for naive mapping
        mapping.crossbar[0][mapping.max_jdx as usize].asap_level = circuit.gates[i].asap_level;
        
        // Create a boxed copy of the mapping for the gate
        let gate_map = Box::new(mapping.crossbar[0][mapping.max_jdx as usize].clone());
        circuit.gates[i].gate_map = Some(gate_map);
        
        // Connect the first input
        if ip1 >= MAX_GATES as i32 {
            // Input is a primary input
            let input_num = ip1 - MAX_GATES as i32;
            if input_num < circuit.num_inputs as i32 {
                let input_gate = mapping.crossbar[0][input_num as usize].clone();
                mapping.crossbar[0][mapping.max_jdx as usize].inputs[0] = Some(Box::new(input_gate));
            }
        } else if ip1 > 0 {
            // Input is a gate output
            if let Some(&gate_idx) = inv_map.get(&ip1) {
                if let Some(ref gate_map) = circuit.gates[gate_idx].gate_map {
                    let input_gate = (**gate_map).clone();
                    mapping.crossbar[0][mapping.max_jdx as usize].inputs[0] = Some(Box::new(input_gate));
                }
            }
        }
        
        // Connect the second input for NOR gates
        if circuit.gates[i].fanin > 1 && ip2 != -1 {
            if ip2 >= MAX_GATES as i32 {
                // Input is a primary input
                let input_num = ip2 - MAX_GATES as i32;
                if input_num < circuit.num_inputs as i32 {
                    let input_gate = mapping.crossbar[0][input_num as usize].clone();
                    mapping.crossbar[0][mapping.max_jdx as usize].inputs[1] = Some(Box::new(input_gate));
                }
            } else if ip2 > 0 {
                // Input is a gate output
                if let Some(&gate_idx) = inv_map.get(&ip2) {
                    if let Some(ref gate_map) = circuit.gates[gate_idx].gate_map {
                        let input_gate = (**gate_map).clone();
                        mapping.crossbar[0][mapping.max_jdx as usize].inputs[1] = Some(Box::new(input_gate));
                    }
                }
            }
        }
    }
    
    mapping
}

pub fn create_compact_mapping(circuit: &mut Circuit) -> CrossbarMapping {
    let mut mapping = CrossbarMapping::new();
    
    // Reset crossbar array
    for i in 0..MAX_ROW {
        for j in 0..mapping.crossbar[i].len() {
            mapping.crossbar[i][j].value = -1;
            mapping.crossbar[i][j].idx = -1;
            mapping.crossbar[i][j].jdx = -1;
            mapping.crossbar[i][j].asap_level = -1;
            mapping.crossbar[i][j].fanin = 0;
            mapping.crossbar[i][j].is_copy = false;
        }
    }
    
    // Reset gate mappings
    for i in 0..circuit.num_gates {
        circuit.gates[i].gate_map = None;
    }
    
    // Sort gates by ASAP level
    circuit.gates.sort_by(|a, b| a.asap_level.cmp(&b.asap_level));
    
    // Create inverse mapping for gate lookup by output
    let mut inv_map = HashMap::new();
    for i in 0..circuit.num_gates {
        inv_map.insert(circuit.gates[i].out, i);
    }
    
    // Handle case where there are no inputs
    if circuit.num_inputs == 0 {
        return mapping;
    }
    
    // Track available positions in each row
    let mut av_row = vec![0; MAX_ROW];
    
    // Map primary inputs - each in its own row
    for i in 0..circuit.num_inputs {
        mapping.crossbar[i][0].value = (MAX_GATES + i) as i32;
        mapping.crossbar[i][0].idx = i as i32;
        mapping.crossbar[i][0].jdx = 0;
        av_row[i] = 1; // Set first available column to 1
    }
    
    // Max row index is the last primary input row
    mapping.max_idx = if circuit.num_inputs > 0 {
        (circuit.num_inputs - 1) as i32
    } else {
        0
    };
    
    // Map gates
    for i in 0..circuit.num_gates {
        let ip1 = circuit.gates[i].inputs[0];
        
        if circuit.gates[i].fanin == 1 {
            // NOT Gate
            let map_idx = if ip1 >= MAX_GATES as i32 {
                // Input is a primary input - use its row
                let input_num = ip1 - MAX_GATES as i32;
                if input_num < circuit.num_inputs as i32 {
                    input_num as usize
                } else {
                    0
                }
            } else if let Some(&gate_idx) = inv_map.get(&ip1) {
                // Input is a gate - use its row
                if let Some(ref gate_map) = circuit.gates[gate_idx].gate_map {
                    gate_map.idx as usize
                } else {
                    0
                }
            } else {
                0
            };
            
            // Get next available column in the row
            let map_jdx = av_row[map_idx];
            av_row[map_idx] += 1;
            
            // Create the NOT gate
            let mut mem_gate = MemristiveGate::default();
            mem_gate.idx = map_idx as i32;
            mem_gate.jdx = map_jdx as i32;
            mem_gate.fanin = 1;
            mem_gate.value = circuit.gates[i].out;
            mem_gate.asap_level = circuit.gates[i].asap_level;
            
            // Connect input
            if ip1 >= MAX_GATES as i32 {
                let input_num = ip1 - MAX_GATES as i32;
                if input_num < circuit.num_inputs as i32 {
                    let input_gate = mapping.crossbar[input_num as usize][0].clone();
                    mem_gate.inputs[0] = Some(Box::new(input_gate));
                }
            } else if let Some(&gate_idx) = inv_map.get(&ip1) {
                if let Some(ref gate_map) = circuit.gates[gate_idx].gate_map {
                    let input_gate = (**gate_map).clone();
                    mem_gate.inputs[0] = Some(Box::new(input_gate));
                }
            }
            
            // Place gate in crossbar and update gate mapping
            mapping.crossbar[map_idx][map_jdx] = mem_gate.clone();
            circuit.gates[i].gate_map = Some(Box::new(mem_gate));
            
            // Update max_jdx if needed
            if map_jdx as i32 > mapping.max_jdx {
                mapping.max_jdx = map_jdx as i32;
            }
        } else {
            // NOR Gate
            let ip2 = circuit.gates[i].inputs[1];
            
            // Get row and column of first input
            let temp_idx = if ip1 >= MAX_GATES as i32 {
                let input_num = ip1 - MAX_GATES as i32;
                if input_num < circuit.num_inputs as i32 {
                    input_num as usize
                } else {
                    0
                }
            } else if let Some(&gate_idx) = inv_map.get(&ip1) {
                if let Some(ref gate_map) = circuit.gates[gate_idx].gate_map {
                    gate_map.idx as usize
                } else {
                    0
                }
            } else {
                0
            };
            
            let temp_jdx = if ip1 >= MAX_GATES as i32 {
                0 // Primary inputs are always in column 0
            } else if let Some(&gate_idx) = inv_map.get(&ip1) {
                if let Some(ref gate_map) = circuit.gates[gate_idx].gate_map {
                    gate_map.jdx as usize
                } else {
                    0
                }
            } else {
                0
            };
            
            // Get row and column of second input
            let temp_udx = if ip2 >= MAX_GATES as i32 {
                let input_num = ip2 - MAX_GATES as i32;
                if input_num < circuit.num_inputs as i32 {
                    input_num as usize
                } else {
                    0
                }
            } else if let Some(&gate_idx) = inv_map.get(&ip2) {
                if let Some(ref gate_map) = circuit.gates[gate_idx].gate_map {
                    gate_map.idx as usize
                } else {
                    0
                }
            } else {
                0
            };
            
            let temp_vdx = if ip2 >= MAX_GATES as i32 {
                0 // Primary inputs are always in column 0
            } else if let Some(&gate_idx) = inv_map.get(&ip2) {
                if let Some(ref gate_map) = circuit.gates[gate_idx].gate_map {
                    gate_map.jdx as usize
                } else {
                    0
                }
            } else {
                0
            };
            
            // Decide where to place the NOR gate
            let (map_idx, map_jdx) = if temp_idx == temp_udx {
                // Both inputs are on the same row
                let idx = temp_idx;
                let jdx = av_row[idx];
                av_row[idx] += 1;
                (idx, jdx)
            } else {
                // Inputs are on different rows - create a copy of first input on second input's row
                let idx = temp_udx;
                let jdx = av_row[idx];
                av_row[idx] += 1;
                
                // Create copy gate
                let mut copy_gate = MemristiveGate::default();
                copy_gate.is_copy = true;
                
                // Copy points to the original gate
                if ip1 >= MAX_GATES as i32 || (ip1 > 0 && inv_map.contains_key(&ip1)) {
                    let input_gate = if ip1 >= MAX_GATES as i32 {
                        let input_num = ip1 - MAX_GATES as i32;
                        if input_num < circuit.num_inputs as i32 {
                            mapping.crossbar[input_num as usize][0].clone()
                        } else {
                            mapping.crossbar[0][0].clone() // Fallback
                        }
                    } else if let Some(&gate_idx) = inv_map.get(&ip1) {
                        if let Some(ref gate_map) = circuit.gates[gate_idx].gate_map {
                            (**gate_map).clone()
                        } else {
                            mapping.crossbar[0][0].clone() // Fallback
                        }
                    } else {
                        mapping.crossbar[0][0].clone() // Fallback
                    };
                    
                    copy_gate.inputs[0] = Some(Box::new(input_gate));
                    copy_gate.value = ip1;
                }
                
                copy_gate.idx = idx as i32;
                copy_gate.jdx = jdx as i32;
                
                // Place copy gate in crossbar
                mapping.crossbar[idx][jdx] = copy_gate;
                
                // NOR gate will be placed right after the copy
                (idx, av_row[idx])
            };
            
            // Increment available column counter for the chosen row
            av_row[map_idx] += 1;
            
            // Create NOR gate
            let mut mem_gate = MemristiveGate::default();
            mem_gate.asap_level = circuit.gates[i].asap_level;
            mem_gate.value = circuit.gates[i].out;
            mem_gate.idx = map_idx as i32;
            mem_gate.jdx = map_jdx as i32;
            mem_gate.fanin = 2;
            
            // Connect inputs based on placement scenario
            if temp_idx == temp_udx {
                // Both inputs on same row - connect directly
                let input1 = mapping.crossbar[temp_idx][temp_jdx].clone();
                let input2 = mapping.crossbar[temp_udx][temp_vdx].clone();
                mem_gate.inputs[0] = Some(Box::new(input1));
                mem_gate.inputs[1] = Some(Box::new(input2));
            } else {
                // One input was copied - use the copy and the original second input
                let input1 = mapping.crossbar[map_idx][map_jdx - 1].clone(); // The copy
                let input2 = mapping.crossbar[temp_udx][temp_vdx].clone();
                mem_gate.inputs[0] = Some(Box::new(input1));
                mem_gate.inputs[1] = Some(Box::new(input2));
            }
            
            // Place gate in crossbar and update gate mapping
            mapping.crossbar[map_idx][map_jdx] = mem_gate.clone();
            circuit.gates[i].gate_map = Some(Box::new(mem_gate));
            
            // Update max dimensions
            mapping.max_idx = max(mapping.max_idx, map_idx as i32);
            mapping.max_jdx = max(mapping.max_jdx, map_jdx as i32);
        }
    }
    
    mapping
}