//generator/mod.rs
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::cmp::max;
use anyhow::{Result, Context};

use crate::{Circuit, CrossbarMapping, MemristiveGate, TableGate, MAX_GATES};

pub fn generate_stats<P: AsRef<Path>>(circuit: &Circuit, path: P) -> Result<()> {
    let mut file = File::create(path)
        .context("Failed to create stats file")?;
    
    // ASAP Schedule Statistics
    generate_schedule_stats(&mut file, circuit, "ASAP", |g| g.asap_level)?;
    
    // ALAP Schedule Statistics
    generate_schedule_stats(&mut file, circuit, "ALAP", |g| g.alap_level)?;
    
    // List Schedule Statistics 
    generate_schedule_stats(&mut file, circuit, "LIST", |g| g.list_level)?;
    
    Ok(())
}

fn generate_schedule_stats<F>(
    file: &mut File, 
    circuit: &Circuit, 
    schedule_name: &str,
    level_getter: F
) -> Result<()>
where
    F: Fn(&TableGate) -> i32
{
    writeln!(file, "{} SCHEDULE:", schedule_name)?;
    writeln!(file, "=============")?;
    
    // Gate distribution across levels
    let max_level = circuit.gates.iter()
        .map(|g| level_getter(g))
        .max()
        .unwrap_or(0);
    
    // Ensure max_level is at least 1 to prevent empty vector
    // and limit the max size to avoid overflow (some circuits might have very high level values)
    let vector_size = max(1, max_level as usize).min(500);  // Limit to 500 levels for output display
    
    let mut gate_count = vec![0; vector_size];
    for gate in &circuit.gates {
        let level = level_getter(gate);
        if level > 0 && (level as usize) <= vector_size {
            gate_count[(level - 1) as usize] += 1;
        }
    }
    
    writeln!(file, "Gate distribution across levels:\n  {}", 
        gate_count.iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" "))?;
    
    // Find maximum gates at any level
    let max_gates = *gate_count.iter().max().unwrap_or(&0);
    
    writeln!(file, "Number of levels: {}, MaxGates: {}", max_level, max_gates)?;
    
    // Count memristors and time steps
    let mut cross_rows = vec![0; max_gates];
    
    for level in 1..=max_level {
        let mut gates_level = 0;
        
        for gate in &circuit.gates {
            if level_getter(gate) == level {
                if gate.fanin == 1 {
                    // NOT gate
                    if cross_rows[gates_level] == 2 {
                        cross_rows[gates_level] = 3;
                    } else {
                        cross_rows[gates_level] = 1;
                    }
                    gates_level += 1;
                } else if gate.fanin == 2 {
                    // NOR gate
                    if cross_rows[gates_level] == 1 {
                        cross_rows[gates_level] = 3;
                    } else {
                        cross_rows[gates_level] = 2;
                    }
                    gates_level += 1;
                }
            }
        }
    }
    
    // Count NOT and NOR gates
    let (not_count, nor_count) = circuit.gates.iter()
        .fold((0, 0), |(not, nor), gate| {
            if gate.fanin == 1 {
                (not + 1, nor)
            } else {
                (not, nor + 1)
            }
        });
    
    // Calculate total memristors (serial configuration)
    let mut memr_serial = 0;
    for i in 0..max_gates {
        if cross_rows[i] == 1 {
            memr_serial += 2;
        } else {
            memr_serial += 3;
        }
    }
    
    writeln!(file, "Number of memristors: {}", memr_serial)?;
    
    // Calculate time steps
    let time_parallel = 2 * max_level;
    let time_serial = not_count + nor_count + max_level;
    
    writeln!(file, "Time steps (serial): {}, Time steps (parallel): {}",
        time_serial, time_parallel)?;
    
    // Calculate crossbar size
    writeln!(file, "Crossbar size (serial): {} x {}", max_gates, 3)?;
    
    let count = cross_rows.iter().filter(|&&x| x == 1).count();
    writeln!(file, "Crossbar size (parallel): {} x {}", 
        max_gates, (3 * max_gates) - count)?;
    
    Ok(())
}

pub fn generate_magic_verilog<P: AsRef<Path>>(circuit: &Circuit, path: P) -> Result<()> {
    //println!("[VERILOG] Opening file: {:?}", path.as_ref());
    let mut file = File::create(path)
        .context("Failed to create Verilog file")?;
    //println!("[VERILOG] File opened successfully.");

    // Sort gates by ASAP level for correct ordering
    //println!("[VERILOG] Sorting gates by ASAP level...");
    let mut sorted_gates = circuit.gates.clone();
    sorted_gates.sort_by(|a, b| a.asap_level.cmp(&b.asap_level));
    //println!("[VERILOG] Gates sorted. Total gates: {}", sorted_gates.len());

    // Generate verilog header with bench name (following C format)
    writeln!(file, "// NOR_NOT mapped module module_name\n")?;
    //println!("[VERILOG] Wrote header comment.");

    // Module declaration - use module_name like the C implementation
    writeln!(file, "module module_name (")?;
    //println!("[VERILOG] Wrote module declaration.");

    // Inputs - use ip_X format like the C implementation 
    //println!("[VERILOG] Declaring {} inputs...", circuit.num_inputs);
    if circuit.num_inputs == 0 {
        println!("[VERILOG][WARNING] No inputs detected!");
    }
    for i in 0..(circuit.num_inputs.saturating_sub(1)) {
        writeln!(file, "  input  ip_{},", i + 1)?;
    }
    if circuit.num_inputs > 0 {
        writeln!(file, "  input  ip_{},", circuit.num_inputs)?;
    }
    //println!("[VERILOG] Inputs declared.");

    // Outputs - use op_X format like the C implementation
    //println!("[VERILOG] Declaring {} outputs...", circuit.num_outputs);
    if circuit.num_outputs == 0 {
        println!("[VERILOG][WARNING] No outputs detected!");
    }
    for i in 0..(circuit.num_outputs.saturating_sub(1)) {
        writeln!(file, "  output op_{},", i + 1)?;
    }
    if circuit.num_outputs > 0 {
        writeln!(file, "  output op_{}\n);", circuit.num_outputs)?;
    }
    //println!("[VERILOG] Outputs declared.");

    // Internal wires
    writeln!(file)?;
    // println!(
    //     "[VERILOG] Declaring internal wires for indices {} to {}...",
    //     circuit.num_outputs + 1,
    //     circuit.num_gates
    // );
    // if circuit.num_gates < circuit.num_outputs + 1 {
    //     println!(
    //         "[VERILOG][WARNING] num_gates < num_outputs+1: {} < {}",
    //         circuit.num_gates,
    //         circuit.num_outputs + 1
    //     );
    // }
    for i in (circuit.num_outputs + 1)..=circuit.num_gates {
        writeln!(file, "  wire wr_{};", i)?;
    }
    writeln!(file)?;
    //println!("[VERILOG] Internal wires declared.");

    // Generate gate instances
    //println!("[VERILOG] Generating gate instances...");
    for (i, gate) in sorted_gates.iter().enumerate() {
        let gate_name = format!("g{}", i + 1);

        // println!(
        //     "[VERILOG] Gate {}: fanin={}, out={}, inputs={:?}",
        //     gate_name, gate.fanin, gate.out, gate.inputs
        // );

        if gate.fanin == 1 {
            // NOT gate
            let ip1 = gate.inputs[0];
            writeln!(
                file,
                "  not    {}( {} ,           {} );",
                format!("{:<5}", gate_name),
                format_wire(gate.out),
                format_wire(ip1)
            )?;
        } else {
            // NOR gate
            let ip1 = gate.inputs[0];
            let ip2 = gate.inputs[1];
            writeln!(
                file,
                "  nor    {}( {} , {} , {} );",
                format!("{:<5}", gate_name),
                format_wire(gate.out),
                format_wire(ip1),
                format_wire(ip2)
            )?;
        }
    }
    //println!("[VERILOG] All gate instances written.");

    // Module end
    writeln!(file, "\nendmodule")?;
    //println!("[VERILOG] Wrote endmodule. Verilog generation complete.");

    Ok(())
}


fn format_wire(id: i32) -> String {
    if id >= MAX_GATES as i32 {
        // Primary inputs - use ip_X format following C implementation 
        format!("ip_{:<5}", id - MAX_GATES as i32 + 1)
    } else if id > 0 {
        // Internal wires - use wr_X format
        format!("wr_{:<5}", id)
    } else {
        // Outputs - use op_X format following C implementation
        format!("op_{:<5}", -id)
    }
}

pub fn generate_micro_ops<P: AsRef<Path>>(
    circuit: &Circuit, 
    mapping: &CrossbarMapping, 
    is_naive: bool,
    path: P
) -> Result<()> {
    let mut file = File::create(path)
        .context("Failed to create micro-ops file")?;
    
    let mut curr_level = 0;
    let mut any_gates_printed = false;
    
    // Process by level - matches C implementation
    for l in 0..circuit.max_asap {
        for i in 0..=mapping.max_idx as usize {
            for j in 0..=mapping.max_jdx as usize {
                // Skip irrelevant gates - same logic as C implementation
                if mapping.crossbar[i][j].value == -1 || 
                   mapping.crossbar[i][j].value >= MAX_GATES as i32 ||
                   mapping.crossbar[i][j].is_copy || 
                   mapping.crossbar[i][j].asap_level != l {
                    continue;
                }
                
                // Print level header when level changes - matches C format
                if mapping.crossbar[i][j].asap_level > curr_level {
                    curr_level = mapping.crossbar[i][j].asap_level;
                    writeln!(file, "# Level: {:2} _____________________________________", curr_level)?;
                }
                
                any_gates_printed = true;
                
                // Print gate information - matches C format exactly
                write!(file, "{:4} {:5} ", mapping.crossbar[i][j].idx, "False")?;
                
                if let Some(ref ip1) = mapping.crossbar[i][j].inputs[0] {
                    write!(file, "{:4} ", ip1.jdx)?;
                    write!(file, "{:9} ", format_gate_name(ip1))?;
                } else {
                    write!(file, "{:14} ", " ")?;
                }
                
                if mapping.crossbar[i][j].fanin > 1 {
                    if let Some(ref ip2) = mapping.crossbar[i][j].inputs[1] {
                        write!(file, "{:4}", ip2.jdx)?;
                        write!(file, "{:9} ", format_gate_name(ip2))?;
                    } else {
                        write!(file, "{:14} ", " ")?;
                    }
                } else {
                    write!(file, "{:14}", " ")?;
                }
                
                writeln!(file, "{:4} True", mapping.crossbar[i][j].jdx)?;
            }
        }
    }
    
    // Add metrics - matches C implementation format
    writeln!(file, "\nMetrics")?;
    writeln!(file, "-------")?;
    writeln!(file, "Primary Inputs    : {}", circuit.num_inputs)?;
    
    if any_gates_printed {
        writeln!(file, "Levels            : {}", curr_level)?;
    } else {
        writeln!(file, "Levels            : 0")?;
    }
    
    writeln!(file, "Read Operations   : {}", circuit.max_asap)?;
    writeln!(file, "Write Operations  : {}", 2 * circuit.max_asap + 1)?;
    writeln!(file, "Evaluation Cycles : {}", circuit.max_asap)?;
    writeln!(file, "Total Cycles      : {}", 4 * circuit.max_asap + 1)?;
    
    // Crossbar size
    if is_naive {
        if mapping.max_jdx < 0 {
            writeln!(file, "Crossbar Size     : {}x{}", 1, 1)?;
        } else {
            writeln!(file, "Crossbar Size     : {}x{}", 1, mapping.max_jdx + 1)?;
        }
    } else {
        if mapping.max_idx < 0 || mapping.max_jdx < 0 {
            writeln!(file, "Crossbar Size     : {}x{}", 1, 1)?;
        } else {
            writeln!(file, "Crossbar Size     : {}x{}", mapping.max_idx + 1, mapping.max_jdx + 1)?;
        }
    }
    writeln!(file, "---------------------------\n\n")?;
    
    Ok(())
}

fn format_gate_name(mem: &MemristiveGate) -> String {
    if mem.value >= MAX_GATES as i32 {
        // Primary input format - matches C implementation
        format!("/{}", mem.value - MAX_GATES as i32)
    } else if mem.is_copy {
        if let Some(ref input) = mem.inputs[0] {
            format_gate_name(input)
        } else {
            String::from("???")
        }
    } else {
        // Coordinates format - matches C implementation
        format!("{}x{}", mem.idx, mem.jdx)
    }
}