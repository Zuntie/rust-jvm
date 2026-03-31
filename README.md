# Rust JVM
A lightweight, experimental Java Virtual Machine (JVM) written entirely in Rust from scratch.

This project was built to explore the  inner workings of language runtimes, bytecode execution, and memory management. It features a custom `.class` file parser, a dual-mode execution engine (Interpreter & x86_64 AOT Compiler), and an implementation of the Java memory model including a custom Mark-and-Sweep Garbage Collector.

## Features
#### Dual-Mode Execution Engine
Supports both a traditional stack-based interpreter and an Ahead-of-Time (AOT) compiler that translates JVM bytecode directly into x86_64 assembly.

#### Custom Garbage Collector
Implements a tracing Mark-and-Sweep garbage collector to manage heap allocations, built from scratch in Rust.

#### Zero-Dependency Class Parser
Manually parses compiled Java bytecode (`.class files`), resolving the constant pool, interfaces, fields, methods, and attributes (`class_file.rs`).

#### Dynamic Memory Layout
Dynamically computes object sizes and field offsets by walking the class inheritance hierarchy.

#### Native Interop (FFI)
Links emitted assembly with C-based runtime stubs (`stub.c`) for low-level memory allocation and I/O handling.

## Implementation Details
### Execution Engine (Interpreter vs. AOT Compiler)
The VM operates in two distinct modes (`--mode interpret` or `--mode` compile):

Interpreter

> A classic stack-based VM loop that decodes and executes JVM opcodes. It manages its own program counter (PC), frame pointer (FP), and a `frame_stack` to handle context switching during method invocations (`INVOKESTATIC`, `INVOKEVIRTUAL`).

x86_64 Compiler

> Translates JVM stack operations into native x86_64 assembly. It performs basic block analysis (`find_leader`) to resolve branching targets for instructions like `IF_ICMPGE` and `GOTO`, and maps the JVM operand stack directly to the native `rsp/rbp` stack.

### Memory Management & Garbage Collection
Memory is divided into a thread-local execution stack (`Vec<StackValue>`) and a globally accessible Heap (`Vec<HeapObject>`). To manage dynamic allocations (`NEW`, `NEWARRAY`), the VM utilizes a custom Mark-and-Sweep Garbage Collector:


Allocation

> Objects and arrays are allocated sequentially. If the heap reaches its capacity (`max_heap_size`), the GC is synchronously triggered.

Mark Phase (Worklist Algorithm)

> The GC identifies the "root set" by scanning the current execution stack for `StackValue::Ref`. It uses a worklist-based approach (rather than recursion) to traverse the object graph, marking reachable `Object` and `ArrayObject` structures to prevent stack overflows during deep object graph traversal.

Sweep Phase
> The heap is linearly scanned. Any unvisited `HeapObject` is replaced with a `HeapObject::Free` tombstone, making the slot available for future allocations without requiring immediate heap compaction.

### Object Layout & Field Resolution
When the `NEW` opcode is encountered, the VM dynamically computes the required byte size of the object by recursively walking up the superclass chain (stopping at `java/lang/Object`). Field offsets are calculated dynamically, ensuring that `PUTFIELD` and `GETFIELD` assembly instructions access the correct memory offsets relative to the object's base pointer.


## Architecture & File Structure
`src/main.rs` - The main entry point, VM state struct, AOT compiler, and Mark-and-Sweep GC logic.

`src/class_file.rs` - Zero-dependency parsing of the JVM .class binary format.

`src/opcodes.rs` - Definitions and mapping of JVM instructions.

`src/value.rs` - Internal representation of JVM data types (StackValue, StackFrame, HeapObject).

`src/cursor.rs` - Byte-stream utility for parsing big-endian JVM bytecode.

`src/stub.c` - Native C stubs for memory allocation (alloc_stub) and console printing, linked during AOT compilation.

`src/tests/` - A suite of compiled Java .class files used to validate the JVM.

### Monolithic Approach
You might notice that a significant portion of the core execution loop, compiler, and garbage collector lives within a single, large main.rs file. This monolithic design was an intentional trade-off. Because this project was built primarily as an educational deep-dive, the focus was entirely on rapidly prototyping and understanding the JVM internals. Keeping the VM's state closely coupled in a monolithic structure allowed for faster iteration and easier debugging of the core memory concepts, rather than spending time on modularity and file separation.

## Getting Started
### Prerequisites

* Rust & Cargo (latest stable version)

* Java Development Kit (JDK) (only required to compile new test files)

> Note: To run the AOT compiled mode, you will need NASM/GCC depending on your assembler setup.

### Building the VM
Clone the repository and build the project using Cargo:
````sh
git clone https://github.com/Zuntie/rust-jvm
cd rust-jvm
cargo build --release
````

### Running Java Programs

Execute a compiled .class file by passing it to the VM.
````sh
# Run the Factorial test in interpreter mode (default)
cargo run -- src/tests/Factorial.class --mode interpret

# Run the Garbage Collection stress test
cargo run -- src/tests/TestGC.class

# Compile a test to x86_64 assembly
cargo run -- src/tests/compiler_while.class --mode compile
````

## Test Suite
The `src/tests/` directory contains various Java programs designed to stress test different parts of the VM. To modify and recompile a test, simply use `javac`:
```` sh
cd src/tests
javac SimpleMath.java
````

### Current Test Coverage:

`TestGC` & `TestHeap` - Stresses the garbage collector, object allocation, and field offsets.

`compiler_while` & `compiler_jump` - Tests basic block analysis, conditional jumps, and branching.

`Factorial` - Tests recursive method invocation and stack frame management.

`compiler_npe` - Tests runtime exception handling (NullPointerException).

## Contributing

This is an educational project, but contributions, issues, and feature requests are welcome! If you want to add support for a missing JVM opcode, or fix a bug, feel free to open a Pull Request.

## License
This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.
