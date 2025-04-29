# Delphi: Memristor Logic Synthesis Toolchain

```
██████╗ ███████╗██╗     ██████╗ ██╗  ██╗██╗
██╔══██╗██╔════╝██║     ██╔══██╗██║  ██║██║
██║  ██║█████╗  ██║     ██████╔╝███████║██║
██║  ██║██╔══╝  ██║     ██╔═══╝ ██╔══██║██║
██████╔╝███████╗███████╗██║     ██║  ██║██║
╚═════╝ ╚══════╝╚══════╝╚═╝     ╚═╝  ╚═╝╚═╝
```

A high-performance memristor-based logic synthesis toolchain with advanced scheduling and mapping capabilities.

## Overview

Delphi is a modern implementation of a memristor-based logic synthesis toolchain, designed for optimizing digital circuits for implementation on memristor crossbar arrays. It converts NOT/NOR netlists into optimized crossbar configurations, providing both naive and compact mapping strategies.

## Installation

### Prerequisites
- Rust (1.67.0 or newer) - [Install Rust](https://www.rust-lang.org/tools/install)
- Cargo package manager (included with Rust)

### Building from Source

#### Windows
```powershell
# Clone the repository
git clone https://github.com/CynicPixel/delphi.git
cd delphi

# Build in release mode
cargo build --release

# The executable will be available at
# .\target\release\delphi.exe
```

#### macOS/Linux
```bash
# Clone the repository
git clone https://github.com/CynicPixel/delphi.git
cd delphi

# Build in release mode
cargo build --release

# The executable will be available at
# ./target/release/delphi
```

## Command Line Interface

```
USAGE:
    delphi [OPTIONS] <COMMAND>

COMMANDS:
    process     Process a single netlist file
    bench       Process an entire benchmark suite
    benchmark   Run performance comparison between sequential and parallel implementations
    help        Print this message or the help of the given subcommand(s)

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information
```

## Detailed Usage

### Processing a Single Netlist

#### Windows
```powershell
# Basic usage
.\delphi.exe process <NETLIST>

# With options
.\delphi.exe process <NETLIST> --output <DIR> --disable-parallel
```

#### macOS/Linux
```bash
# Basic usage
./delphi process <NETLIST>

# With options
./delphi process <NETLIST> --output <DIR> --disable-parallel
```

#### Options for `process` command:
- `<NETLIST>`: Path to the netlist file (required)
- `-o, --output <DIR>`: Output directory for results (default: ./Results)
- `--disable-parallel`: Disable parallel processing

#### Example (Windows):
```powershell
# Process c17 benchmark with parallel processing
.\delphi.exe process C:\path\to\BENCH\netlist\iscas85_c17.txt

# Process with custom output directory and sequential processing
.\delphi.exe process C:\path\to\BENCH\netlist\iscas85_c17.txt -o .\my_results --disable-parallel
```

### Processing a Benchmark Suite

#### Windows
```powershell
# Basic usage
.\delphi.exe bench <DIR>

# With options
.\delphi.exe bench <DIR> --output <OUTPUT_DIR> --pattern <PATTERN> --disable-parallel
```

#### macOS/Linux
```bash
# Basic usage
./delphi bench <DIR>

# With options
./delphi bench <DIR> --output <OUTPUT_DIR> --pattern <PATTERN> --disable-parallel
```

#### Options for `bench` command:
- `<DIR>`: Path to the benchmark directory (required)
- `-o, --output <DIR>`: Output directory for results (default: ./Results)
- `-p, --pattern <PATTERN>`: Only process files matching this pattern
- `--disable-parallel`: Disable parallel processing

#### Example (Windows):
```powershell
# Process all benchmarks in a directory
.\delphi.exe bench C:\path\to\BENCH\netlist\

# Process only iscas85 benchmarks
.\delphi.exe bench C:\path\to\BENCH\netlist\ -p iscas85
```

### Running Performance Benchmarks

#### Windows
```powershell
# Basic usage
.\delphi.exe benchmark <NETLIST>

# With options
.\delphi.exe benchmark <NETLIST> --iterations <ITERATIONS>
```

#### macOS/Linux
```bash
# Basic usage
./delphi benchmark <NETLIST>

# With options
./delphi benchmark <NETLIST> --iterations <ITERATIONS>
```

#### Options for `benchmark` command:
- `<NETLIST>`: Path to the netlist file (required)
- `-i, --iterations <ITERATIONS>`: Number of iterations for accurate timing (default: 3)

#### Example (Windows):
```powershell
# Benchmark c17 with default iterations
.\delphi.exe benchmark C:\path\to\BENCH\netlist\iscas85_c17.txt

# Benchmark with 10 iterations
.\delphi.exe benchmark C:\path\to\BENCH\netlist\iscas85_c17.txt -i 10
```

## Output Files

Delphi generates several output files organized in subdirectories under the specified output directory:

### Output Structure

Windows:
```
Results\
├── magic\
│   └── [benchmark]_magic.v       # NOR/NOT mapped Verilog module
├── micro_ins_compact\
│   └── [benchmark]_compact.txt   # Compact mapping micro-operations
├── micro_ins_naive\
│   └── [benchmark]_naive.txt     # Naive mapping micro-operations
└── schedule_stats\
    └── [benchmark]_stats.txt     # Scheduling statistics
```

macOS/Linux:
```
Results/
├── magic/
│   └── [benchmark]_magic.v       # NOR/NOT mapped Verilog module
├── micro_ins_compact/
│   └── [benchmark]_compact.txt   # Compact mapping micro-operations
├── micro_ins_naive/
│   └── [benchmark]_naive.txt     # Naive mapping micro-operations
└── schedule_stats/
    └── [benchmark]_stats.txt     # Scheduling statistics
```

### File Descriptions

1. **Magic Verilog (`_magic.v`)**
   - NOR/NOT mapped module definition
   - Input, output, and wire declarations
   - Gate instantiations

2. **Naive Mapping (`_naive.txt`)**
   - Micro-operations for naive crossbar mapping
   - Simple, linear mapping strategy
   - Each level's operations listed separately

3. **Compact Mapping (`_compact.txt`)**
   - Micro-operations for optimized compact mapping
   - Efficiently utilizes crossbar space
   - Each level's operations listed separately

4. **Schedule Statistics (`_stats.txt`)**
   - ASAP, ALAP, and LIST scheduling metrics
   - Gate distribution across levels
   - Crossbar size and time step information
   - Performance comparisons

### Viewing Output Files (Windows)

```powershell
# View statistics
type Results\schedule_stats\iscas85_c17_stats.txt

# View Verilog mapping
type Results\magic\iscas85_c17_magic.v

# View micro-operations
type Results\micro_ins_naive\iscas85_c17_naive.txt
```

## Advanced Features

### Parallel Processing

Delphi supports parallel processing to speed up computations for larger circuits:

```powershell
# Enable parallel processing (default)
.\delphi.exe process <NETLIST>

# Disable parallel processing
.\delphi.exe process <NETLIST> --disable-parallel
```

The parallel implementation:
- Automatically adjusts to the number of available CPU cores
- Only activates for circuits with 100+ gates
- Provides significant speedup for larger benchmarks
- Falls back to sequential processing for small circuits

### Benchmark-Specific Processing

You can process specific benchmark types or patterns:

```powershell
# Process only iscas85 benchmarks
.\delphi.exe bench C:\path\to\benchmarks -p iscas85

# Process only c17 benchmark variants
.\delphi.exe bench C:\path\to\benchmarks -p c17
```

### Performance Benchmarking

Compare sequential and parallel implementations:

```powershell
.\delphi.exe benchmark <NETLIST> -i 5
```

This will:
1. Run both sequential and parallel versions multiple times
2. Measure execution time for each stage
3. Calculate speedup metrics
4. Display detailed performance comparisons

## Troubleshooting

### Windows-Specific Issues

**Path Separators:**
Windows uses backslashes (`\`) in paths, while the command line might require either escaped backslashes (`\\`) or forward slashes (`/`).

**File Access Permissions:**
If you encounter permission errors, try running the command prompt or PowerShell as Administrator.

**Network Paths:**
If your files are on a network drive, use the full UNC path (e.g., `\\server\share\path\to\file.txt`).

### Common Issues on All Platforms

**File Not Found:**
```
Error: Failed to parse netlist: No such file or directory
```
Solution: Check that the netlist file path is correct and accessible.

**Invalid Benchmark Directory:**
```
Error: Invalid benchmark directory
```
Solution: Ensure the benchmark directory exists and contains netlist files.

**Parsing Errors:**
```
Error: Failed to parse netlist: Invalid format at line X
```
Solution: Verify that the netlist file follows the required format for NOT/NOR gates.

## Examples

### Basic Processing (Windows)

```powershell
# Process a single benchmark
.\delphi.exe process C:\path\to\BENCH\netlist\iscas85_c17.txt

# View the statistics
type Results\schedule_stats\iscas85_c17_stats.txt

# View the Verilog mapping
type Results\magic\iscas85_c17_magic.v
```

### Batch Processing with Custom Output (Windows)

```powershell
# Create output directory
mkdir my_results

# Process all iscas benchmarks
.\delphi.exe bench C:\path\to\BENCH\netlist -p iscas -o my_results
```

### Performance Comparison (Windows)

```powershell
# Detailed benchmark with 5 iterations
.\delphi.exe benchmark C:\path\to\BENCH\netlist\iscas85_c7552.txt -i 5
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

---

For more information or issues, please open an issue on the GitHub repository.