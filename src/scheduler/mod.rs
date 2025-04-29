use crate::Circuit;
use crate::TableGate;
use std::collections::HashMap;

// Scheduling algorithms
pub fn compute_asap_schedule(circuit: &mut Circuit) {
    let mut count = 0;
    
    while count < circuit.num_gates {
        for i in 0..circuit.num_gates {
            if circuit.gates[i].fanin == 1 {
                // NOT gate
                let input_level = get_asap_level(circuit, circuit.gates[i].inputs[0]);
                if input_level != -1 {
                    circuit.gates[i].asap_level = input_level + 1;
                    if input_level + 1 > circuit.max_asap {
                        circuit.max_asap = input_level + 1;
                    }
                    count += 1;
                }
            } else if circuit.gates[i].fanin == 2 {
                // NOR gate
                let input1_level = get_asap_level(circuit, circuit.gates[i].inputs[0]);
                let input2_level = get_asap_level(circuit, circuit.gates[i].inputs[1]);
                
                let max_level = input1_level.max(input2_level);
                
                if input1_level != -1 && input2_level != -1 {
                    circuit.gates[i].asap_level = max_level + 1;
                    if max_level + 1 > circuit.max_asap {
                        circuit.max_asap = max_level + 1;
                    }
                    count += 1;
                }
            }
        }
    }
}

pub fn compute_alap_schedule(circuit: &mut Circuit) {
    // Initialize PO levels
    for i in 0..circuit.num_gates {
        if is_po(circuit, circuit.gates[i].out) {
            circuit.gates[i].alap_level = 1;
        }
    }
    
    // Iterate until all gates have been labeled
    while !all_alap_labeled(circuit) {
        for i in 0..circuit.num_gates {
            update_alap(circuit, i, circuit.gates[i].out);
        }
    }
    
    // Additional iterations to ensure convergence
    for _ in 0..10 {
        for i in 0..circuit.num_gates {
            update_alap(circuit, i, circuit.gates[i].out);
        }
    }
    
    // Correct ALAP levels
    let mut max_level = 0;
    for i in 0..circuit.num_gates {
        if circuit.gates[i].alap_level > max_level {
            max_level = circuit.gates[i].alap_level;
        }
    }
    
    for i in 0..circuit.num_gates {
        circuit.gates[i].alap_level = max_level - circuit.gates[i].alap_level + 1;
    }
    
    circuit.max_alap = max_level;
}

pub fn compute_list_schedule(circuit: &mut Circuit) {
    // Compute mobilities
    let mut max_level = 0;
    for i in 0..circuit.num_gates {
        circuit.gates[i].mobility = circuit.gates[i].alap_level - circuit.gates[i].asap_level;
        
        if circuit.gates[i].asap_level > max_level {
            max_level = circuit.gates[i].asap_level;
        }
    }
    
    // Sort gates by mobility (smallest first)
    circuit.gates.sort_by(|a, b| a.mobility.cmp(&b.mobility));
    
    // Find minimal number of gates per level
    for max_gates in 2..20 {
        // Reinitialize list levels
        for i in 0..circuit.num_gates {
            circuit.gates[i].list_level = -1;
        }
        
        if list_schedule_possible(circuit, max_level, max_gates) {
            circuit.max_list = max_level;
            break;
        }
    }
}

// Helper functions
fn get_asap_level(circuit: &Circuit, line_id: i32) -> i32 {
    if is_pi(circuit, line_id) {
        return 0;
    }
    
    for i in 0..circuit.num_gates {
        if circuit.gates[i].out == line_id {
            return circuit.gates[i].asap_level;
        }
    }
    
    // Error case
    -1
}

fn get_alap_level(circuit: &Circuit, line_id: i32) -> i32 {
    if is_po(circuit, line_id) {
        return 0;
    }
    
    for i in 0..circuit.num_gates {
        if circuit.gates[i].out == line_id {
            return circuit.gates[i].alap_level;
        }
    }
    
    // Error case
    -1
}

fn get_list_level(circuit: &Circuit, line_id: i32) -> i32 {
    if is_pi(circuit, line_id) {
        return 0;
    }
    
    for i in 0..circuit.num_gates {
        if circuit.gates[i].out == line_id {
            return circuit.gates[i].list_level;
        }
    }
    
    // Error case
    -1
}

fn is_pi(circuit: &Circuit, line_id: i32) -> bool {
    for i in 0..circuit.num_gates {
        if circuit.gates[i].out == line_id {
            return false;
        }
    }
    
    true
}

fn is_po(circuit: &Circuit, line_id: i32) -> bool {
    for i in 0..circuit.num_gates {
        for j in 0..circuit.gates[i].fanin {
            if circuit.gates[i].inputs[j] == line_id {
                return false;
            }
        }
    }
    
    true
}

fn all_alap_labeled(circuit: &Circuit) -> bool {
    for i in 0..circuit.num_gates {
        if circuit.gates[i].alap_level == -1 {
            return false;
        }
    }
    
    true
}

fn update_alap(circuit: &mut Circuit, index: usize, line_id: i32) {
    for j in 0..circuit.num_gates {
        for k in 0..circuit.gates[j].fanin {
            if line_id == circuit.gates[j].inputs[k] && circuit.gates[j].alap_level != -1 {
                if circuit.gates[index].alap_level <= circuit.gates[j].alap_level {
                    circuit.gates[index].alap_level = circuit.gates[j].alap_level + 1;
                }
            }
        }
    }
}

fn list_schedule_possible(circuit: &mut Circuit, max_level: i32, max_gates: i32) -> bool {
    let mut ngates = 0;
    let mut max_level_assigned = 0;
    
    while ngates < circuit.num_gates {
        let mut gates_in_level = 0;
        let mut flag = false;
        
        for i in 0..circuit.num_gates {
            if circuit.gates[i].fanin == 1 {
                // NOT gate
                let input_level = get_list_level(circuit, circuit.gates[i].inputs[0]);
                
                if input_level != -1 {
                    circuit.gates[i].list_level = input_level + 1;
                    if max_level_assigned < input_level + 1 {
                        max_level_assigned = input_level + 1;
                    }
                    gates_in_level += 1;
                    ngates += 1;
                    flag = true;
                    
                    if gates_in_level == max_gates {
                        break; // Current level filled up
                    }
                }
            } else if circuit.gates[i].fanin == 2 {
                // NOR gate
                let input1_level = get_list_level(circuit, circuit.gates[i].inputs[0]);
                let input2_level = get_list_level(circuit, circuit.gates[i].inputs[1]);
                
                let max_input_level = input1_level.max(input2_level);
                
                if input1_level != -1 && input2_level != -1 {
                    circuit.gates[i].list_level = max_input_level + 1;
                    if max_level_assigned < max_input_level + 1 {
                        max_level_assigned = max_input_level + 1;
                    }
                    gates_in_level += 1;
                    ngates += 1;
                    flag = true;
                    
                    if gates_in_level == max_gates {
                        break; // Current level filled up
                    }
                }
            }
        }
        
        if !flag {
            return false; // List schedule could not be formed
        }
    }
    
    max_level_assigned == max_level
}