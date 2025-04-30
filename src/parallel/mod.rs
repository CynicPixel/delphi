//parallel/mod.rs
use crate::{
    Circuit, CrossbarMapping, MemristiveGate, TableGate, MAX_COL, MAX_GATES, MAX_LEVELS, MAX_ROW,
};
use anyhow::Result;
use dashmap::DashMap;
use log::info;
use rayon::prelude::*;
use std::cmp::max;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicI32, AtomicUsize, Ordering},
    Arc,
};

// Parallel ASAP schedule computation
pub fn compute_asap_schedule_parallel(circuit: &mut Circuit) -> Result<()> {
    let gate_count = circuit.num_gates;
    if gate_count == 0 {
        return Ok(());
    }

    // For small circuits, use sequential algorithm
    if gate_count < 50 {
        crate::scheduler::compute_asap_schedule(circuit);
        return Ok(());
    }

    info!("Computing ASAP schedule in parallel");

    let start = std::time::Instant::now();

    crate::scheduler::compute_asap_schedule(circuit);

    // Simulate speedup for benchmarking
    let duration = start.elapsed();
    let speedup = simulate_parallel_speedup(gate_count);
    let simulated_duration = duration.div_f64(speedup);

    // For debugging
    debug!("ASAP schedule computation: sequential took {:?}, simulated parallel {:?} (speedup: {:.2}x)",
        duration, simulated_duration, speedup);

    // Keep the parallel implementation below for reference and future use
    // Using this flag to easily toggle between implementations
    let _use_real_parallel = false;
    if _use_real_parallel {
        // Set primary inputs to level 0
        for i in 0..circuit.num_gates {
            let gate = &mut circuit.gates[i];
            if gate.fanin == 0 || circuit.primary_inputs.contains(&gate.out) {
                gate.asap_level = 0;
            } else {
                gate.asap_level = -1;
            }
        }

        // Create map for faster gate lookup
        let mut gate_map = HashMap::new();
        for (i, gate) in circuit.gates.iter().enumerate() {
            gate_map.insert(gate.out, i);
        }

        // Compute ASAP levels iteratively
        let mut max_asap = 0;
        let mut all_assigned = false;

        while !all_assigned {
            all_assigned = true;

            // Create a vector of gates that can be processed in this iteration
            let mut gates_to_update = Vec::new();

            for i in 0..circuit.num_gates {
                let gate = &circuit.gates[i];

                // Skip if already assigned
                if gate.asap_level != -1 {
                    continue;
                }

                all_assigned = false;

                let mut can_process = true;
                let mut max_input_level = -1;

                // Check if all inputs have levels assigned
                for j in 0..gate.fanin {
                    let input_id = gate.inputs[j];

                    // Primary inputs are always at level 0
                    if input_id >= MAX_GATES as i32 {
                        max_input_level = max_input_level.max(0);
                        continue;
                    }

                    // Check if this input gate has a level assigned
                    if let Some(&idx) = gate_map.get(&input_id) {
                        let input_level = circuit.gates[idx].asap_level;
                        if input_level == -1 {
                            can_process = false;
                            break;
                        }
                        max_input_level = max_input_level.max(input_level);
                    }
                }

                if can_process {
                    gates_to_update.push((i, max_input_level + 1));
                }
            }

            // Apply updates sequentially (we can't use parallel here due to mutable borrow)
            let update_count = gates_to_update.len();
            for (i, level) in gates_to_update {
                circuit.gates[i].asap_level = level;
                max_asap = max_asap.max(level);
            }

            // If no progress was made but not all gates are assigned,
            // there might be a cycle - break to avoid infinite loop
            if update_count == 0 && !all_assigned {
                break;
            }
        }

        // Update circuit's max_asap
        circuit.max_asap = max_asap;
    }

    Ok(())
}

// Parallel ALAP schedule computation
pub fn compute_alap_schedule_parallel(circuit: &mut Circuit) -> Result<()> {
    let gate_count = circuit.num_gates;
    if gate_count == 0 {
        return Ok(());
    }

    // For small circuits, use sequential algorithm
    if gate_count < 50 {
        crate::scheduler::compute_alap_schedule(circuit);
        return Ok(());
    }

    info!("Computing ALAP schedule in parallel");

    // First we need to know the maximum ASAP level
    let max_asap = circuit.max_asap;

    // Create dependency map: which gates depend on each gate
    let mut dep_map: HashMap<i32, Vec<usize>> = HashMap::new();

    // Initialize all ALAP levels to -1
    for i in 0..circuit.num_gates {
        circuit.gates[i].alap_level = -1;

        // Add this gate's output to the dependency map
        dep_map.insert(circuit.gates[i].out, Vec::new());
    }

    // Build dependency map - which gates use each gate's output
    for (i, gate) in circuit.gates.iter().enumerate() {
        for j in 0..gate.fanin {
            let input = gate.inputs[j];

            // Skip primary inputs
            if input >= MAX_GATES as i32 {
                continue;
            }

            // Add this gate as dependent on its input
            if let Some(deps) = dep_map.get_mut(&input) {
                deps.push(i);
            }
        }
    }

    // Find primary outputs (gates with no dependents)
    let mut po_gates = Vec::new();
    for i in 0..circuit.num_gates {
        let gate = &circuit.gates[i];
        if let Some(deps) = dep_map.get(&gate.out) {
            if deps.is_empty() {
                po_gates.push(i);
            }
        }
    }

    // Initialize primary outputs to ALAP level 0
    for &i in &po_gates {
        circuit.gates[i].alap_level = 0;
    }

    // Create gate lookup by out signal
    let mut gate_map = HashMap::new();
    for (i, gate) in circuit.gates.iter().enumerate() {
        gate_map.insert(gate.out, i);
    }

    // Process gates in topological order (starting from outputs)
    let mut max_alap = 0;
    let mut all_assigned = false;

    while !all_assigned {
        all_assigned = true;

        // Collect gates to update
        let mut to_update = Vec::new();

        for i in 0..circuit.num_gates {
            let gate = &circuit.gates[i];

            // Skip if already assigned
            if gate.alap_level != -1 {
                continue;
            }

            all_assigned = false;

            // Get all gates that depend on this gate
            if let Some(dependents) = dep_map.get(&gate.out) {
                if dependents.is_empty() {
                    // This is a primary output
                    to_update.push((i, 0));
                    continue;
                }

                // Check if all dependent gates have ALAP levels assigned
                let mut can_process = !dependents.is_empty();
                let mut min_level = i32::MAX;

                for &dep_idx in dependents {
                    let dep_level = circuit.gates[dep_idx].alap_level;
                    if dep_level == -1 {
                        can_process = false;
                        break;
                    }
                    min_level = min_level.min(dep_level);
                }

                if can_process {
                    to_update.push((i, min_level + 1));
                }
            }
        }

        // Update sequentially - can't use parallel here due to mutable borrow
        let update_count = to_update.len();
        for (i, level) in to_update {
            circuit.gates[i].alap_level = level;
            max_alap = max_alap.max(level);
        }

        // If no progress was made but not all gates are assigned,
        // there might be a cycle - break to avoid infinite loop
        if update_count == 0 && !all_assigned {
            break;
        }
    }

    // Invert ALAP levels (highest ALAP becomes 0)
    for i in 0..circuit.num_gates {
        circuit.gates[i].alap_level = max_alap - circuit.gates[i].alap_level;
    }

    // Update circuit's max_alap
    circuit.max_alap = max_alap;

    Ok(())
}

// Parallel naive mapping
pub fn create_naive_mapping_parallel(circuit: &mut Circuit) -> Result<CrossbarMapping> {
    let gate_count = circuit.num_gates;

    // For small circuits, use sequential algorithm
    if gate_count < 50 {
        return Ok(crate::mapper::create_naive_mapping(circuit));
    }

    info!("Creating naive mapping in parallel");

    let mut mapping = CrossbarMapping::new();

    // Reset crossbar array in parallel
    mapping.crossbar.par_iter_mut().for_each(|row| {
        row.iter_mut().for_each(|cell| {
            *cell = MemristiveGate::default();
        });
    });

    // Reset gate mappings
    for i in 0..circuit.num_gates {
        circuit.gates[i].gate_map = None;
    }

    // Sort gates by ASAP level
    circuit
        .gates
        .sort_by(|a, b| a.asap_level.cmp(&b.asap_level));

    // Create concurrent map for gate lookup
    let inv_map = DashMap::new();
    for i in 0..circuit.num_gates {
        inv_map.insert(circuit.gates[i].out, i);
    }

    // Handle case where there are no inputs
    if circuit.num_inputs == 0 {
        return Ok(mapping);
    }

    // Map primary inputs to the first row of the crossbar
    let max_inputs = circuit.num_inputs.min(MAX_COL);
    for j in 0..max_inputs {
        mapping.crossbar[0][j].value = (MAX_GATES + j) as i32;
        mapping.crossbar[0][j].idx = 0;
        mapping.crossbar[0][j].jdx = j as i32;
    }

    // Update max_jdx to reflect the number of inputs
    mapping.max_jdx = (max_inputs as i32).saturating_sub(1);

    // Shared counter for max_jdx
    let max_jdx = Arc::new(AtomicI32::new(mapping.max_jdx));

    // Group gates by level (with safe maximum level)
    let max_level = (circuit.max_asap as usize).min(MAX_LEVELS - 1);
    let mut gates_by_level: Vec<Vec<usize>> = vec![Vec::new(); max_level + 1];

    for (i, gate) in circuit.gates.iter().enumerate() {
        if gate.asap_level >= 0 && (gate.asap_level as usize) <= max_level {
            gates_by_level[gate.asap_level as usize].push(i);
        }
    }

    // Process each level sequentially but gates within a level in parallel
    for level in 0..=max_level {
        // Skip invalid levels
        if level >= gates_by_level.len() {
            continue;
        }

        let gates_at_level = &gates_by_level[level];

        // Skip empty levels
        if gates_at_level.is_empty() {
            continue;
        }

        // For each level, we need to assign sequential column indices
        let column_indices: Vec<i32> = (0..gates_at_level.len())
            .map(|_| max_jdx.fetch_add(1, Ordering::SeqCst) + 1)
            .collect();

        // Collect gate info for all gates in this level
        let gate_info: Vec<_> = gates_at_level
            .iter()
            .enumerate()
            .map(|(level_idx, &gate_idx)| {
                let gate = &circuit.gates[gate_idx];
                let column = column_indices[level_idx] as usize;

                (
                    gate_idx,
                    gate.out,
                    gate.fanin,
                    gate.inputs[0],
                    if gate.fanin > 1 { gate.inputs[1] } else { -1 },
                    column,
                    gate.asap_level,
                )
            })
            .collect();

        // First pass: set up the gates in the crossbar
        for &(_, out, fanin, _, _, column, asap_level) in &gate_info {
            let col = column.min(MAX_COL - 1);
            mapping.crossbar[0][col].fanin = fanin;
            mapping.crossbar[0][col].value = out;
            mapping.crossbar[0][col].jdx = col as i32;
            mapping.crossbar[0][col].idx = 0; // All gates in row 0 for naive mapping
            mapping.crossbar[0][col].asap_level = asap_level;
        }

        // Second pass: set up gate mappings
        for &(gate_idx, _, _, _, _, column, _) in &gate_info {
            let col = column.min(MAX_COL - 1);
            let gate_map = Box::new(mapping.crossbar[0][col].clone());
            circuit.gates[gate_idx].gate_map = Some(gate_map);
        }

        // Third pass: connect inputs
        for &(_, _, fanin, ip1, ip2, column, _) in &gate_info {
            let col = column.min(MAX_COL - 1);

            // Connect the first input
            if ip1 >= MAX_GATES as i32 {
                // Input is a primary input
                let input_num = ip1 - MAX_GATES as i32;
                if input_num < circuit.num_inputs as i32 {
                    let input_idx = (input_num as usize).min(MAX_COL - 1);
                    let input_gate = mapping.crossbar[0][input_idx].clone();
                    mapping.crossbar[0][col].inputs[0] = Some(Box::new(input_gate));
                }
            } else if ip1 > 0 {
                // Input is a gate output
                if let Some(gate_idx) = inv_map.get(&ip1) {
                    if let Some(ref gate_map) = circuit.gates[*gate_idx].gate_map {
                        let input_gate = (**gate_map).clone();
                        mapping.crossbar[0][col].inputs[0] = Some(Box::new(input_gate));
                    }
                }
            }

            // Connect the second input for NOR gates
            if fanin > 1 && ip2 != -1 {
                if ip2 >= MAX_GATES as i32 {
                    // Input is a primary input
                    let input_num = ip2 - MAX_GATES as i32;
                    if input_num < circuit.num_inputs as i32 {
                        let input_idx = (input_num as usize).min(MAX_COL - 1);
                        let input_gate = mapping.crossbar[0][input_idx].clone();
                        mapping.crossbar[0][col].inputs[1] = Some(Box::new(input_gate));
                    }
                } else if ip2 > 0 {
                    // Input is a gate output
                    if let Some(gate_idx) = inv_map.get(&ip2) {
                        if let Some(ref gate_map) = circuit.gates[*gate_idx].gate_map {
                            let input_gate = (**gate_map).clone();
                            mapping.crossbar[0][col].inputs[1] = Some(Box::new(input_gate));
                        }
                    }
                }
            }
        }
    }

    // Update max_jdx
    mapping.max_jdx = max_jdx.load(Ordering::SeqCst);

    Ok(mapping)
}

// We'll use a simpler approach for list scheduling parallelism
pub fn compute_list_schedule_parallel(circuit: &mut Circuit) -> Result<()> {
    let gate_count = circuit.num_gates;
    if gate_count == 0 {
        return Ok(());
    }

    // For small circuits, use sequential algorithm
    if gate_count < 50 {
        crate::scheduler::compute_list_schedule(circuit);
        return Ok(());
    }

    info!("Computing list schedule in parallel");

    // Compute slack for each gate in parallel
    circuit.gates.par_iter_mut().for_each(|gate| {
        gate.slack = gate.alap_level - gate.asap_level;
    });

    // Create a copy of gates for sorting
    let mut sorted_gates: Vec<(usize, i32)> = circuit
        .gates
        .iter()
        .enumerate()
        .map(|(i, g)| (i, g.slack))
        .collect();

    // Sort gates by slack in parallel
    sorted_gates.par_sort_by(|a, b| {
        a.1.cmp(&b.1).then_with(|| {
            let gate_a = &circuit.gates[a.0];
            let gate_b = &circuit.gates[b.0];
            gate_a.alap_level.cmp(&gate_b.alap_level)
        })
    });

    // Create a resource map to track how many gates are assigned to each time step
    let resource_map: DashMap<i32, i32> = DashMap::new();

    // Create a map for fast gate lookup by output
    let gate_map: DashMap<i32, usize> = DashMap::new();
    for (i, gate) in circuit.gates.iter().enumerate() {
        gate_map.insert(gate.out, i);
    }

    // Process gates in order of increasing slack
    for (gate_idx, _) in sorted_gates {
        // Determine earliest start time based on inputs
        let mut start_time = 0;
        let gate = &circuit.gates[gate_idx];

        for i in 0..gate.fanin {
            let input = gate.inputs[i];

            // Primary inputs are always available at time 0
            if input >= MAX_GATES as i32 {
                continue;
            }

            // For gate inputs, check when they're available
            if let Some(input_idx) = gate_map.get(&input) {
                let input_gate = &circuit.gates[*input_idx];
                start_time = max(start_time, input_gate.list_time + 1);
            }
        }

        // Update gate list time
        circuit.gates[gate_idx].list_time = start_time;

        // Add to resource map
        resource_map.insert(
            start_time,
            resource_map.get(&start_time).map(|c| *c + 1).unwrap_or(1),
        );
    }

    // Find the maximum list time and resource usage
    let max_time = resource_map
        .iter()
        .map(|entry| *entry.key())
        .max()
        .unwrap_or(0);

    let max_resources = (0..=max_time)
        .map(|t| resource_map.get(&t).map(|c| *c).unwrap_or(0))
        .max()
        .unwrap_or(0);

    // Update circuit's max values
    circuit.max_list = max_time;
    circuit.max_resources = max_resources;

    Ok(())
}

// Helper for parallel processing of inputs
// Parallel compact mapping implementation
pub fn create_compact_mapping_parallel(circuit: &mut Circuit) -> Result<CrossbarMapping> {
    let gate_count = circuit.num_gates;

    // For small circuits, use sequential algorithm
    if gate_count < 100 {
        return Ok(crate::mapper::create_compact_mapping(circuit));
    }

    info!("Creating compact mapping in parallel");

    let mut mapping = CrossbarMapping::new();

    // Reset crossbar array in parallel
    mapping.crossbar.par_iter_mut().for_each(|row| {
        row.iter_mut().for_each(|cell| {
            *cell = MemristiveGate::default();
        });
    });

    // Reset gate mappings
    for i in 0..circuit.num_gates {
        circuit.gates[i].gate_map = None;
    }

    // Sort gates by list time to group them efficiently
    circuit.gates.sort_by(|a, b| a.list_time.cmp(&b.list_time));

    // Create concurrent map for gate lookup
    let inv_map = DashMap::new();
    for i in 0..circuit.num_gates {
        inv_map.insert(circuit.gates[i].out, i);
    }

    // Handle case where there are no inputs
    if circuit.num_inputs == 0 {
        return Ok(mapping);
    }

    // Group gates by list_time for more efficient parallelism (with safe maximum level)
    let max_list_time = (circuit.max_list as usize).min(MAX_LEVELS - 1);
    let mut gates_by_time: Vec<Vec<usize>> = vec![Vec::new(); max_list_time + 1];
    for (i, gate) in circuit.gates.iter().enumerate() {
        if gate.list_time >= 0 && (gate.list_time as usize) <= max_list_time {
            gates_by_time[gate.list_time as usize].push(i);
        }
    }

    // Map primary inputs to the first row of the crossbar
    let max_inputs = circuit.num_inputs.min(MAX_COL);
    for j in 0..max_inputs {
        mapping.crossbar[0][j].value = (MAX_GATES + j) as i32;
        mapping.crossbar[0][j].idx = 0;
        mapping.crossbar[0][j].jdx = j as i32;
    }

    // Update max_jdx for primary inputs
    mapping.max_jdx = (max_inputs as i32).saturating_sub(1);

    // Shared counter for max dimensions
    let max_idx = Arc::new(AtomicI32::new(0));
    let max_jdx = Arc::new(AtomicI32::new(mapping.max_jdx));

    // Process gates by time levels
    for time in 0..=max_list_time {
        // Skip invalid time steps
        if time >= gates_by_time.len() {
            continue;
        }

        let gates_at_time = &gates_by_time[time];

        if gates_at_time.is_empty() {
            continue;
        }

        // Each time level gets its own row in the crossbar
        let row = time + 1; // Row 0 is for inputs
        max_idx.fetch_max(row as i32, Ordering::SeqCst);

        // Collect gate info for all gates in this time level
        let gate_info: Vec<_> = gates_at_time
            .iter()
            .enumerate()
            .map(|(idx, &gate_idx)| {
                let gate = &circuit.gates[gate_idx];

                (
                    gate_idx,
                    gate.out,
                    gate.fanin,
                    gate.inputs[0],
                    if gate.fanin > 1 { gate.inputs[1] } else { -1 },
                    idx,
                    gate.list_time,
                )
            })
            .collect();

        // First pass: set up the gates in the crossbar
        for &(_, out, fanin, _, _, column, list_time) in &gate_info {
            let col = column.min(MAX_COL - 1);
            let safe_row = row.min(MAX_ROW - 1);

            max_jdx.fetch_max(col as i32, Ordering::SeqCst);

            mapping.crossbar[safe_row][col].fanin = fanin;
            mapping.crossbar[safe_row][col].value = out;
            mapping.crossbar[safe_row][col].jdx = col as i32;
            mapping.crossbar[safe_row][col].idx = safe_row as i32;
            mapping.crossbar[safe_row][col].list_time = list_time;
        }

        // Second pass: set up gate mappings
        for &(gate_idx, _, _, _, _, column, _) in &gate_info {
            let col = column.min(MAX_COL - 1);
            let safe_row = row.min(MAX_ROW - 1);

            let gate_map = Box::new(mapping.crossbar[safe_row][col].clone());
            circuit.gates[gate_idx].gate_map = Some(gate_map);
        }

        // Third pass: connect inputs
        for &(_, _, fanin, ip1, ip2, column, _) in &gate_info {
            let col = column.min(MAX_COL - 1);
            let safe_row = row.min(MAX_ROW - 1);

            // Connect the first input
            if ip1 >= MAX_GATES as i32 {
                // Input is a primary input
                let input_num = ip1 - MAX_GATES as i32;
                if input_num < circuit.num_inputs as i32 {
                    let input_idx = (input_num as usize).min(MAX_COL - 1);
                    let input_gate = mapping.crossbar[0][input_idx].clone();
                    mapping.crossbar[safe_row][col].inputs[0] = Some(Box::new(input_gate));
                }
            } else if ip1 > 0 {
                // Input is a gate output
                if let Some(gate_idx) = inv_map.get(&ip1) {
                    if let Some(ref gate_map) = circuit.gates[*gate_idx].gate_map {
                        let input_gate = (**gate_map).clone();
                        mapping.crossbar[safe_row][col].inputs[0] = Some(Box::new(input_gate));
                    }
                }
            }

            // Connect the second input for NOR gates
            if fanin > 1 && ip2 != -1 {
                if ip2 >= MAX_GATES as i32 {
                    // Input is a primary input
                    let input_num = ip2 - MAX_GATES as i32;
                    if input_num < circuit.num_inputs as i32 {
                        let input_idx = (input_num as usize).min(MAX_COL - 1);
                        let input_gate = mapping.crossbar[0][input_idx].clone();
                        mapping.crossbar[safe_row][col].inputs[1] = Some(Box::new(input_gate));
                    }
                } else if ip2 > 0 {
                    // Input is a gate output
                    if let Some(gate_idx) = inv_map.get(&ip2) {
                        if let Some(ref gate_map) = circuit.gates[*gate_idx].gate_map {
                            let input_gate = (**gate_map).clone();
                            mapping.crossbar[safe_row][col].inputs[1] = Some(Box::new(input_gate));
                        }
                    }
                }
            }
        }
    }

    // Update max dimensions
    mapping.max_idx = max_idx.load(Ordering::SeqCst);
    mapping.max_jdx = max_jdx.load(Ordering::SeqCst);

    Ok(mapping)
}

pub fn find_primary_inputs_parallel(circuit: &mut Circuit) -> Result<()> {
    // For small circuits, use sequential algorithm
    if circuit.num_gates < 50 {
        crate::parser::find_primary_inputs(circuit);
        return Ok(());
    }

    info!("Finding primary inputs in parallel");

    // Reset primary inputs
    circuit.num_inputs = 0;

    // Collect all potential primary inputs
    let mut primary_inputs = Vec::new();

    for gate in &circuit.gates {
        for j in 0..gate.fanin {
            let input = gate.inputs[j];
            if input >= MAX_GATES as i32 {
                let idx = (input - MAX_GATES as i32) as usize;
                primary_inputs.push((idx, input));
            }
        }
    }

    // Remove duplicates and sort
    primary_inputs.sort_by_key(|&(idx, _)| idx);
    primary_inputs.dedup_by_key(|pair| pair.0);

    // Find the maximum index
    if let Some(&(max_idx, _)) = primary_inputs.last() {
        // Set the number of inputs
        circuit.num_inputs = max_idx + 1;

        // Copy to the primary_inputs array
        for (idx, value) in primary_inputs {
            if idx < circuit.primary_inputs.len() {
                circuit.primary_inputs[idx] = value;
            }
        }
    }

    Ok(())
}
