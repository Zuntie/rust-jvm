mod class_file;
mod cursor;
mod logger;
mod opcodes;
mod value;

use clap::Parser;
use class_file::ClassFile;
use class_file::CodeAttribute;
use class_file::ConstantPoolEntry;
use cursor::Cursor;
use logger::{LogLevel, Logger};
use opcodes::*;
use value::{ArrayObject, HeapObject, Object, StackFrame, StackValue};

use colored::Colorize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;

const LOG_LEVEL: LogLevel = LogLevel::Asm;

#[derive(Parser, Debug)]
#[command(name = "rust-jvm")]
#[command(about = "A PoC JVM implementation in Rust")]
struct Args {
    #[arg(value_name = "FILE")]
    file: String,

    #[arg(short, long, value_name = "MODE", default_value = "compile")]
    mode: String,
}

struct VM<'a> {
    stack: Vec<StackValue>,
    program: Vec<u8>,
    pc: usize,
    fp: usize,
    frame_stack: Vec<StackFrame>,
    class_file: &'a ClassFile,
    class_size_cache: HashMap<String, usize>,
    heap: Vec<HeapObject>,
    max_heap_size: usize,
    log: Logger,
    current_method_name: String,
    arg_count: usize,
}

impl<'a> VM<'a> {
    fn new(
        program: Vec<u8>,
        class_file: &'a ClassFile,
        method_name: String,
        arg_count: usize,
    ) -> Self {
        let log = Logger::new(LOG_LEVEL);

        log.info("   > Initializing VM");

        VM {
            stack: Vec::new(),
            program,
            pc: 0,
            fp: 0,
            frame_stack: Vec::new(),
            class_file,
            class_size_cache: HashMap::new(),
            heap: Vec::new(),
            max_heap_size: 5,
            log,
            current_method_name: method_name,
            arg_count,
        }
    }

    fn pop_int(&mut self) -> i32 {
        match self.stack.pop() {
            Some(StackValue::Int(v)) => v,
            Some(StackValue::Ref(_)) => panic!("Runtime Error: Expected Int, found Ref"),
            None => panic!("Stack Underflow"),
        }
    }

    fn pop_ref(&mut self) -> usize {
        match self.stack.pop() {
            Some(StackValue::Ref(v)) => v,
            Some(StackValue::Int(_)) => panic!("Runtime Error: Expected Ref, found Int"),
            None => panic!("Stack Underflow"),
        }
    }

    fn exec(&mut self) {
        println!("{}", "   VM Execution".bold());
        while self.pc < self.program.len() {
            let opcode: u8 = self.program[self.pc];
            self.pc += 1;

            match opcode {
                BIPUSH => {
                    let value = self.program[self.pc] as i32;
                    self.pc += 1;
                    self.stack.push(StackValue::Int(value));
                }
                IADD => {
                    let v1 = self.pop_int();
                    let v2 = self.pop_int();
                    self.stack.push(StackValue::Int(v1 + v2));
                }
                ISUB => {
                    let v1 = self.pop_int();
                    let v2 = self.pop_int();
                    self.stack.push(StackValue::Int(v2 - v1));
                }
                IFEQ => {
                    let branchbyte1 = self.program[self.pc] as u8;
                    let branchbyte2 = self.program[self.pc + 1] as u8;
                    self.pc += 2;
                    let offset = u16::from_be_bytes([branchbyte1, branchbyte2]) as i16;
                    let value = self.pop_int();
                    if value == 0 {
                        self.pc = (self.pc as isize + offset as isize - 3) as usize;
                    }
                }
                IF_ICMPGE => {
                    let branchbyte1 = self.program[self.pc] as u8;
                    let branchbyte2 = self.program[self.pc + 1] as u8;
                    self.pc += 2;
                    let offset = u16::from_be_bytes([branchbyte1, branchbyte2]) as i16;
                    let v2 = self.pop_int();
                    let v1 = self.pop_int();
                    if v1 >= v2 {
                        self.pc = (self.pc as isize + offset as isize - 3) as usize;
                    }
                }
                IFLT => {
                    let branchbyte1 = self.program[self.pc] as u8;
                    let branchbyte2 = self.program[self.pc + 1] as u8;
                    self.pc += 2;
                    let offset = u16::from_be_bytes([branchbyte1, branchbyte2]) as i16;
                    let value = self.pop_int();
                    if value < 0 {
                        self.pc = (self.pc as isize + offset as isize - 3) as usize;
                    }
                }
                IFLE => {
                    let branchbyte1 = self.program[self.pc] as u8;
                    let branchbyte2 = self.program[self.pc + 1] as u8;
                    self.pc += 2;
                    let offset = u16::from_be_bytes([branchbyte1, branchbyte2]) as i16;
                    let value = self.pop_int();
                    if value <= 0 {
                        self.pc = (self.pc as isize + offset as isize - 3) as usize;
                    }
                }
                IINC => {
                    let index = self.program[self.pc] as usize;
                    let constant = self.program[self.pc + 1] as i8 as i32;
                    self.pc += 2;
                    if self.fp + index >= self.stack.len() {
                        self.stack.resize(self.fp + index + 1, StackValue::Int(0));
                    }
                    if let StackValue::Int(val) = &mut self.stack[self.fp + index] {
                        *val += constant;
                    } else {
                        panic!("Runtime Error: IINC on non-int");
                    }
                }
                IF_ICMPGT => {
                    let branchbyte1 = self.program[self.pc] as u8;
                    let branchbyte2 = self.program[self.pc + 1] as u8;
                    self.pc += 2;
                    let offset = u16::from_be_bytes([branchbyte1, branchbyte2]) as i16;
                    let v2 = self.pop_int();
                    let v1 = self.pop_int();
                    if v1 > v2 {
                        self.pc = (self.pc as isize + offset as isize - 3) as usize;
                    }
                }
                IMUL => {
                    let v1 = self.pop_int();
                    let v2 = self.pop_int();
                    self.stack.push(StackValue::Int(v1 * v2));
                }
                GOTO => {
                    let branchbyte1 = self.program[self.pc] as u8;
                    let branchbyte2 = self.program[self.pc + 1] as u8;

                    self.pc += 2;
                    let offset = u16::from_be_bytes([branchbyte1, branchbyte2]) as i16;
                    self.pc = (self.pc as isize + offset as isize - 3) as usize;
                }
                CALL => {
                    let indexbyte1 = self.program[self.pc] as u8;
                    let indexbyte2 = self.program[self.pc + 1] as u8;
                    let argc = self.program[self.pc + 2] as usize;
                    self.pc += 3;

                    let method_index = u16::from_be_bytes([indexbyte1, indexbyte2]) as usize;

                    self.frame_stack.push(StackFrame {
                        pc: self.pc,
                        fp: self.fp,
                        program: self.program.clone(),
                    });

                    if self.stack.len() < argc {
                        println!(
                            "Error: Stack underflow. Needed {} arguments, but stack only has {}.",
                            argc,
                            self.stack.len()
                        );
                        return;
                    }

                    self.fp = self.stack.len() - argc;
                    self.pc = method_index;
                }
                RET => {
                    if let Some(frame) = self.frame_stack.pop() {
                        self.pc = frame.pc;
                        self.program = frame.program;

                        let return_value = self.stack.pop().expect("Stack underflow on RET value");

                        if self.stack.len() > self.fp {
                            self.stack.truncate(self.fp);
                        }
                        self.fp = frame.fp;

                        self.stack.push(return_value);
                    } else {
                        println!("{}", "   VM Execution Complete".bold());
                        return;
                    }
                }
                RETURN => {
                    if let Some(frame) = self.frame_stack.pop() {
                        self.pc = frame.pc;
                        self.program = frame.program;

                        if self.stack.len() > self.fp {
                            self.stack.truncate(self.fp);
                        }
                        self.fp = frame.fp;
                    } else {
                        println!("{}", "   VM Execution Complete".bold());
                        return;
                    }
                }
                IRETURN => {
                    if let Some(frame) = self.frame_stack.pop() {
                        self.pc = frame.pc;
                        self.program = frame.program;

                        let return_value = self.pop_int();

                        if self.stack.len() > self.fp {
                            self.stack.truncate(self.fp);
                        }
                        self.fp = frame.fp;

                        self.stack.push(StackValue::Int(return_value));
                    } else {
                        println!("   VM Execution Complete");
                        return;
                    }
                }
                ILOAD => {
                    let index = self.program[self.pc] as usize;
                    self.pc += 1;
                    let v = self.stack[self.fp + index];
                    self.stack.push(v);
                }
                ISTORE => {
                    let index = self.program[self.pc] as usize;
                    self.pc += 1;
                    let v = self.pop_int();
                    if self.fp + index >= self.stack.len() {
                        self.stack.resize(self.fp + index + 1, StackValue::Int(0));
                    }
                    self.stack[self.fp + index] = StackValue::Int(v);
                }
                GETSTATIC => {
                    let _indexbyte1 = self.program[self.pc];
                    let _indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    self.stack.push(StackValue::Ref(0));
                }
                LDC => {
                    let index = self.program[self.pc] as u8;
                    self.pc += 1;

                    if let Some(constant) = self.class_file.get_utf8(index as u16) {
                        println!("LDC loaded constant: {}", constant);
                    } else {
                        println!("LDC failed to load constant at index {}", index);
                    }

                    self.stack.push(StackValue::Int(index as i32));
                }
                INVOKEVIRTUAL => {
                    let indexbyte1 = self.program[self.pc];
                    let indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    let method_index = u16::from_be_bytes([indexbyte1, indexbyte2]) as usize;

                    let mut descriptor = String::new();

                    if let ConstantPoolEntry::Methodref {
                        name_and_type_index,
                        ..
                    } = &self.class_file.constant_pool[method_index]
                    {
                        if let ConstantPoolEntry::NameAndType {
                            descriptor_index, ..
                        } = &self.class_file.constant_pool[*name_and_type_index as usize]
                        {
                            if let Some(desc_str) = self.class_file.get_utf8(*descriptor_index) {
                                descriptor = desc_str;
                            }
                        }
                    }

                    if descriptor == "(I)V" {
                        let val = self.pop_int();
                        let _obj = self.pop_ref();
                        println!("{}", val);
                    } else if descriptor == "(Ljava/lang/String;)V" {
                        let string_index = self.pop_int() as u16;
                        let _obj = self.pop_ref();

                        if let Some(text) = self.class_file.get_utf8(string_index) {
                            println!("{}", text);
                        } else {
                            println!("Runtime Error: Invalid String Index {}", string_index);
                        }
                    } else {
                        println!(
                            "Runtime Warning: invoked unknown method descriptor: {}",
                            descriptor
                        );
                    }

                    self.stack.push(StackValue::Int(0));
                }
                INVOKESTATIC => {
                    let indexbyte1 = self.program[self.pc];
                    let indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    let method_index = u16::from_be_bytes([indexbyte1, indexbyte2]) as usize;

                    let mut method_name = String::new();
                    let mut method_descriptor = String::new();

                    if let ConstantPoolEntry::Methodref {
                        class_index: _,
                        name_and_type_index,
                    } = &self.class_file.constant_pool[method_index]
                    {
                        if let ConstantPoolEntry::NameAndType {
                            name_index,
                            descriptor_index,
                        } = &self.class_file.constant_pool[*name_and_type_index as usize]
                        {
                            if let Some(name_str) = self.class_file.get_utf8(*name_index) {
                                method_name = name_str;
                            }
                            if let Some(desc_str) = self.class_file.get_utf8(*descriptor_index) {
                                method_descriptor = desc_str;
                            }
                        }
                    }

                    'method_search: for method in &self.class_file.methods {
                        let name = self
                            .class_file
                            .get_utf8(method.name_index)
                            .unwrap_or_default();
                        let descriptor = self
                            .class_file
                            .get_utf8(method.descriptor_index)
                            .unwrap_or_default();

                        if name == method_name && descriptor == method_descriptor {
                            for attr in &method.attributes {
                                let attr_name = self
                                    .class_file
                                    .get_utf8(attr.attribute_name_index)
                                    .unwrap_or_default();
                                if attr_name == "Code" {
                                    let code_attr = CodeAttribute::parse(&attr.info);

                                    let mut argc = 0;
                                    let mut chars = method_descriptor.chars();
                                    while let Some(c) = chars.next() {
                                        if c == '(' {
                                            break;
                                        }
                                    }
                                    while let Some(c) = chars.next() {
                                        if c == ')' {
                                            break;
                                        }
                                        if c == 'L' {
                                            while let Some(n) = chars.next() {
                                                if n == ';' {
                                                    break;
                                                }
                                            }
                                        }
                                        if c == '[' {
                                            continue;
                                        }
                                        argc += 1;
                                    }

                                    println!(
                                        "   > Context Switch: {} (argc: {})",
                                        method_name, argc
                                    );

                                    self.frame_stack.push(StackFrame {
                                        pc: self.pc,
                                        fp: self.fp,
                                        program: self.program.clone(),
                                    });

                                    self.fp = self.stack.len() - argc;
                                    self.program = code_attr.code;
                                    self.pc = 0;

                                    break 'method_search;
                                }
                            }
                        }
                    }
                }
                INVOKESPECIAL => {
                    let indexbyte1 = self.program[self.pc];
                    let indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    let _method_index = u16::from_be_bytes([indexbyte1, indexbyte2]) as usize;

                    let mut descriptor = String::new();
                    if let ConstantPoolEntry::Methodref {
                        name_and_type_index,
                        ..
                    } = &self.class_file.constant_pool[_method_index]
                    {
                        if let ConstantPoolEntry::NameAndType {
                            descriptor_index, ..
                        } = &self.class_file.constant_pool[*name_and_type_index as usize]
                        {
                            if let Some(desc_str) = self.class_file.get_utf8(*descriptor_index) {
                                descriptor = desc_str;
                            }
                        }
                    }

                    let _obj_ref = self.pop_ref();

                    if !descriptor.ends_with("V") {
                        self.stack.push(StackValue::Int(0));
                    }
                }
                ICONST_0 => {
                    self.stack.push(StackValue::Int(0));
                }
                ICONST_1 => {
                    self.stack.push(StackValue::Int(1));
                }
                ICONST_2 => {
                    self.stack.push(StackValue::Int(2));
                }
                ICONST_3 => {
                    self.stack.push(StackValue::Int(3));
                }
                ICONST_4 => {
                    self.stack.push(StackValue::Int(4));
                }
                ICONST_5 => {
                    self.stack.push(StackValue::Int(5));
                }
                ISTORE_0 => {
                    let v = self.pop_int();
                    if self.fp + 0 >= self.stack.len() {
                        self.stack.resize(self.fp + 1, StackValue::Int(0));
                    }
                    self.stack[self.fp + 0] = StackValue::Int(v);
                }
                ISTORE_1 => {
                    let v = self.pop_int();
                    if self.fp + 1 >= self.stack.len() {
                        self.stack.resize(self.fp + 2, StackValue::Int(0));
                    }
                    self.stack[self.fp + 1] = StackValue::Int(v);
                }
                ISTORE_2 => {
                    let v = self.pop_int();
                    if self.fp + 2 >= self.stack.len() {
                        self.stack.resize(self.fp + 3, StackValue::Int(0));
                    }
                    self.stack[self.fp + 2] = StackValue::Int(v);
                }
                ISTORE_3 => {
                    let v = self.pop_int();
                    if self.fp + 3 >= self.stack.len() {
                        self.stack.resize(self.fp + 4, StackValue::Int(0));
                    }
                    self.stack[self.fp + 3] = StackValue::Int(v);
                }
                ILOAD_0 => {
                    let v = self.stack[self.fp + 0];
                    self.stack.push(v);
                }
                ILOAD_1 => {
                    let v = self.stack[self.fp + 1];
                    self.stack.push(v);
                }
                ILOAD_2 => {
                    let v = self.stack[self.fp + 2];
                    self.stack.push(v);
                }
                ILOAD_3 => {
                    let v = self.stack[self.fp + 3];
                    self.stack.push(v);
                }
                ASTORE_0 => {
                    let v = self.stack.pop().expect("Stack underflow on ASTORE_0");
                    if self.fp + 0 >= self.stack.len() {
                        self.stack.resize(self.fp + 1, StackValue::Int(0));
                    }
                    self.stack[self.fp + 0] = v;
                }
                ASTORE_1 => {
                    let v = self.stack.pop().expect("Stack underflow on ASTORE_1");
                    if self.fp + 1 >= self.stack.len() {
                        self.stack.resize(self.fp + 2, StackValue::Int(0));
                    }
                    self.stack[self.fp + 1] = v;
                }
                ASTORE_2 => {
                    let v = self.stack.pop().expect("Stack underflow on ASTORE_2");
                    if self.fp + 2 >= self.stack.len() {
                        self.stack.resize(self.fp + 3, StackValue::Int(0));
                    }
                    self.stack[self.fp + 2] = v;
                }
                ASTORE_3 => {
                    let v = self.stack.pop().expect("Stack underflow on ASTORE_3");
                    if self.fp + 3 >= self.stack.len() {
                        self.stack.resize(self.fp + 4, StackValue::Int(0));
                    }
                    self.stack[self.fp + 3] = v;
                }
                ALOAD_0 => {
                    let v = self.stack[self.fp + 0];
                    self.stack.push(v);
                }
                ALOAD_1 => {
                    let v = self.stack[self.fp + 1];
                    self.stack.push(v);
                }
                ALOAD_2 => {
                    let v = self.stack[self.fp + 2];
                    self.stack.push(v);
                }
                ALOAD_3 => {
                    let v = self.stack[self.fp + 3];
                    self.stack.push(v);
                }
                NEW => {
                    let indexbyte1 = self.program[self.pc];
                    let indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    let class_index = u16::from_be_bytes([indexbyte1, indexbyte2]);

                    let obj = Object {
                        class_index,
                        fields: Vec::new(),
                        marked: false,
                    };

                    let obj_ref = self.alloc(HeapObject::Object(obj));
                    self.stack.push(StackValue::Ref(obj_ref));
                }
                PUTFIELD => {
                    let indexbyte1 = self.program[self.pc];
                    let indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    let field_index = u16::from_be_bytes([indexbyte1, indexbyte2]);

                    let value = self.stack.pop().expect("Stack underflow on PUTFIELD");
                    let obj_ref = self.pop_ref();

                    if let Some(HeapObject::Object(obj)) = self.heap.get_mut(obj_ref) {
                        if let Some(field) =
                            obj.fields.iter_mut().find(|(idx, _)| *idx == field_index)
                        {
                            field.1 = value;
                        } else {
                            obj.fields.push((field_index, value));
                        }
                    } else {
                        println!("Runtime Error: Invalid Object Reference {}", obj_ref);
                    }
                }
                GETFIELD => {
                    let indexbyte1 = self.program[self.pc];
                    let indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    let field_index = u16::from_be_bytes([indexbyte1, indexbyte2]);

                    let obj_ref = self.pop_ref();

                    if let Some(HeapObject::Object(obj)) = self.heap.get(obj_ref) {
                        if let Some((_, value)) =
                            obj.fields.iter().find(|(idx, _)| *idx == field_index)
                        {
                            self.stack.push(*value);
                        } else {
                            println!(
                                "Runtime Error: Field Index {} not found in Object {}",
                                field_index, obj_ref
                            );
                            self.stack.push(StackValue::Int(0));
                        }
                    } else {
                        println!("Runtime Error: Invalid Object Reference {}", obj_ref);
                        self.stack.push(StackValue::Int(0));
                    }
                }
                NEWARRAY => {
                    let atype = self.program[self.pc];
                    self.pc += 1;

                    let count = self.pop_int();

                    if count < 0 {
                        println!("Runtime Error: Negative array size {}", count);
                        self.stack.push(StackValue::Ref(0));
                        continue;
                    }

                    let length = count as usize;

                    let array = ArrayObject {
                        class_index: atype as u16,
                        elements: vec![StackValue::Int(0); length],
                        marked: false,
                    };

                    let array_ref = self.alloc(HeapObject::Array(array));
                    self.stack.push(StackValue::Ref(array_ref));
                }
                IASTORE => {
                    let value = self.stack.pop().expect("Stack underflow on IASTORE");
                    let index = self.pop_int() as usize;
                    let array_ref = self.pop_ref();

                    if let Some(HeapObject::Array(array)) = self.heap.get_mut(array_ref) {
                        if index < array.elements.len() {
                            array.elements[index] = value;
                        } else {
                            println!("Runtime Error: Array index {} out of bounds", index);
                        }
                    } else {
                        println!("Runtime Error: Invalid Array Reference {}", array_ref);
                    }
                }
                IALOAD => {
                    let index = self.pop_int() as usize;
                    let array_ref = self.pop_ref();

                    if let Some(HeapObject::Array(array)) = self.heap.get(array_ref) {
                        if index < array.elements.len() {
                            let value = array.elements[index];
                            self.stack.push(value);
                        } else {
                            println!("Runtime Error: Array index {} out of bounds", index);
                            self.stack.push(StackValue::Int(0));
                        }
                    } else {
                        println!("Runtime Error: Invalid Array Reference {}", array_ref);
                        self.stack.push(StackValue::Int(0));
                    }
                }
                DUP => {
                    let value = self.stack.last().expect("Stack underflow on DUP").clone();
                    self.stack.push(value);
                }
                PRINT => {
                    let value = self.pop_int();
                    println!("PRINT: {}", value);
                }
                HALT => {
                    println!("HALT encountered.");
                    break;
                }
                _ => {
                    println!("Unknown opcode: {:#04X} {}", opcode, opcode_to_name(opcode));
                    return;
                }
            }
        }
    }

    fn mark(&mut self) {
        let mut worklist: Vec<usize> = Vec::new();

        for sv in &self.stack {
            if let StackValue::Ref(obj_ref) = sv {
                worklist.push(*obj_ref);
            }
        }

        while let Some(obj_ref) = worklist.pop() {
            if let Some(heap_obj) = self.heap.get_mut(obj_ref) {
                match heap_obj {
                    HeapObject::Object(obj) => {
                        if !obj.marked {
                            obj.marked = true;
                            for (_, field_value) in &obj.fields {
                                if let StackValue::Ref(field_ref) = field_value {
                                    worklist.push(*field_ref);
                                }
                            }
                        }
                    }
                    HeapObject::Array(array) => {
                        if !array.marked {
                            array.marked = true;
                            for element in &array.elements {
                                if let StackValue::Ref(elem_ref) = element {
                                    worklist.push(*elem_ref);
                                }
                            }
                        }
                    }
                    HeapObject::Free => {}
                }
            }
        }
    }

    fn sweep(&mut self) {
        for heap_obj in &mut self.heap {
            match heap_obj {
                HeapObject::Object(obj) => {
                    if !obj.marked {
                        *heap_obj = HeapObject::Free;
                    } else {
                        obj.marked = false;
                    }
                }
                HeapObject::Array(array) => {
                    if !array.marked {
                        *heap_obj = HeapObject::Free;
                    } else {
                        array.marked = false;
                    }
                }
                HeapObject::Free => {}
            }
        }
    }

    fn alloc(&mut self, obj: HeapObject) -> usize {
        let mut free_index: Option<usize> = None;
        for (i, heap_obj) in self.heap.iter().enumerate() {
            if let HeapObject::Free = heap_obj {
                free_index = Some(i);
                break;
            }
        }

        if let Some(index) = free_index {
            self.heap[index] = obj;
            return index;
        }

        if self.heap.len() >= self.max_heap_size {
            println!("{}", "   > Garbage Collection Triggered".bold());
            self.mark();
            self.sweep();

            let mut free_index: Option<usize> = None;
            for (i, heap_obj) in self.heap.iter().enumerate() {
                if let HeapObject::Free = heap_obj {
                    free_index = Some(i);
                    break;
                }
            }

            if let Some(index) = free_index {
                self.heap[index] = obj;
                return index;
            }

            println!(
                "{}",
                "   > Garbage Collection Complete - No Free Space".bold()
            );
            panic!("Out of memory: Unable to allocate object even after garbage collection");
        }

        self.heap.push(obj);
        self.heap.len() - 1
    }

    fn get_opcode_length(&self, opcode: u8, _pc: usize) -> usize {
        match opcode {
            NOP | ACONST_NULL | ICONST_M1 | ICONST_0 | ICONST_1 | ICONST_2 | ICONST_3
            | ICONST_4 | ICONST_5 | LCONST_0 | LCONST_1 | FCONST_0 | FCONST_1 | FCONST_2
            | DCONST_0 | DCONST_1 | ILOAD_0 | ILOAD_1 | ILOAD_2 | ILOAD_3 | LLOAD_0 | LLOAD_1
            | LLOAD_2 | LLOAD_3 | FLOAD_0 | FLOAD_1 | FLOAD_2 | FLOAD_3 | DLOAD_0 | DLOAD_1
            | DLOAD_2 | DLOAD_3 | ALOAD_0 | ALOAD_1 | ALOAD_2 | ALOAD_3 | IALOAD | LALOAD
            | FALOAD | DALOAD | AALOAD | BALOAD | CALOAD | SALOAD | ISTORE_0 | ISTORE_1
            | ISTORE_2 | ISTORE_3 | LSTORE_0 | LSTORE_1 | LSTORE_2 | LSTORE_3 | FSTORE_0
            | FSTORE_1 | FSTORE_2 | FSTORE_3 | DSTORE_0 | DSTORE_1 | DSTORE_2 | DSTORE_3
            | ASTORE_0 | ASTORE_1 | ASTORE_2 | ASTORE_3 | IASTORE | LASTORE | FASTORE | DASTORE
            | AASTORE | BASTORE | CASTORE | SASTORE | POP | POP2 | DUP | DUP_X1 | DUP_X2 | DUP2
            | DUP2_X1 | DUP2_X2 | SWAP | IADD | LADD | FADD | DADD | ISUB | LSUB | FSUB | DSUB
            | IMUL | LMUL | FMUL | DMUL | IDIV | LDIV | FDIV | DDIV | IREM | LREM | FREM | DREM
            | INEG | LNEG | FNEG | DNEG | ISHL | LSHL | ISHR | LSHR | IUSHR | LUSHR | IAND
            | LAND | IOR | LOR | IXOR | LXOR | I2L | I2F | I2D | L2I | L2F | L2D | F2I | F2L
            | F2D | D2I | D2L | D2F | I2B | I2C | I2S | LCMP | FCMPL | FCMPG | DCMPL | DCMPG
            | IRETURN | LRETURN | FRETURN | DRETURN | ARETURN | RETURN | ARRAYLENGTH | ATHROW
            | MONITORENTER | MONITOREXIT | PRINT | HALT => 1,

            BIPUSH | LDC | ILOAD | LLOAD | FLOAD | DLOAD | ALOAD | ISTORE | LSTORE | FSTORE
            | DSTORE | ASTORE | RET | NEWARRAY | CALL => 2,

            SIPUSH | LDC_W | LDC2_W | IINC | IFEQ | IFNE | IFLT | IFGE | IFGT | IFLE
            | IF_ICMPEQ | IF_ICMPNE | IF_ICMPLT | IF_ICMPGE | IF_ICMPGT | IF_ICMPLE | IF_ACMPEQ
            | IF_ACMPNE | GOTO | JSR | GETSTATIC | PUTSTATIC | GETFIELD | PUTFIELD
            | INVOKEVIRTUAL | INVOKESPECIAL | INVOKESTATIC | NEW | ANEWARRAY | CHECKCAST
            | INSTANCEOF => 3,

            MULTIANEWARRAY => 4,

            INVOKEINTERFACE | INVOKEDYNAMIC | GOTO_W | JSR_W => 5,

            _ => panic!("Unknown opcode length for: {}", opcode),
        }
    }

    fn find_leader(&self) -> HashSet<usize> {
        let mut leaders = HashSet::new();
        let mut pc = 0;

        leaders.insert(0);

        while pc < self.program.len() {
            let opcode = self.program[pc];
            let len = self.get_opcode_length(opcode, pc);

            match opcode {
                IFEQ | IFNE | IFLT | IFGE | IFGT | IFLE | IF_ICMPEQ | IF_ICMPNE | IF_ICMPLT
                | IF_ICMPGE | IF_ICMPGT | IF_ICMPLE | IF_ACMPEQ | IF_ACMPNE | GOTO | JSR => {
                    let branchbyte1 = self.program[pc + 1];
                    let branchbyte2 = self.program[pc + 2];
                    let offset = u16::from_be_bytes([branchbyte1, branchbyte2]) as i16;
                    let target_pc = (pc as isize + offset as isize) as usize;
                    leaders.insert(target_pc);
                    leaders.insert(pc + len);
                }
                _ => {}
            }

            pc += len;
        }

        leaders
    }

    fn compile(&mut self) {
        self.log.info("   > Compiling Program");

        let leaders = self.find_leader();
        self.log
            .info(&format!("   > Found {} Basic Blocks", leaders.len()));

        emit!("push rbp");
        emit!("mov rbp, rsp");
        emit!("sub rsp, 64");

        if self.arg_count > 0 {
            for i in 0..self.arg_count {
                emit!("mov rax, [rbp + {}]", 16 + (self.arg_count - 1 - i) * 8);
                emit!("mov [rbp - {}], rax", (i + 1) * 8);
            }
        }

        while self.pc < self.program.len() {
            if leaders.contains(&self.pc) {
                label!("{}_label_{}:", self.current_method_name, self.pc);
            }

            let opcode = self.program[self.pc];
            self.pc += 1;

            match opcode {
                BIPUSH => {
                    let val = self.program[self.pc] as i32;
                    self.pc += 1;
                    emit!("push {}", val);
                }
                IADD => {
                    emit!(";;-- IADD --");
                    emit!("pop rax");
                    emit!("pop rbx");
                    emit!("add rax, rbx");
                    emit!("push rax");
                }
                ISUB => {
                    emit!(";;-- ISUB --");
                    emit!("pop rbx");
                    emit!("pop rax");
                    emit!("sub rax, rbx");
                    emit!("push rax");
                }
                IMUL => {
                    emit!(";;-- IMUL --");
                    emit!("pop rax");
                    emit!("pop rbx");
                    emit!("imul rax, rbx");
                    emit!("push rax");
                }
                IF_ICMPGE => {
                    let branchbyte1 = self.program[self.pc];
                    let branchbyte2 = self.program[self.pc + 1];
                    self.pc += 2;
                    let offset = u16::from_be_bytes([branchbyte1, branchbyte2]) as i16;

                    emit!(";;-- IF_ICMPGE --");
                    emit!("pop rax");
                    emit!("pop rbx");
                    emit!("cmp rbx, rax");
                    emit!(
                        "jge {}_label_{}",
                        self.current_method_name,
                        self.pc as isize + offset as isize - 3
                    );
                }
                IFLE => {
                    let branchbyte1 = self.program[self.pc];
                    let branchbyte2 = self.program[self.pc + 1];
                    self.pc += 2;
                    let offset = u16::from_be_bytes([branchbyte1, branchbyte2]) as i16;

                    emit!(";;-- IFLE --");
                    emit!("pop rax");
                    emit!("cmp rax, 0");
                    emit!(
                        "jle {}_label_{}",
                        self.current_method_name,
                        self.pc as isize + offset as isize - 3
                    );
                }
                IINC => {
                    let index = self.program[self.pc] as u8;
                    let constant = self.program[self.pc + 1] as i8 as i32;
                    self.pc += 2;

                    emit!(";;-- IINC --");
                    emit!("add qword [rbp-{}], {}", (index as usize + 1) * 8, constant);
                }
                NEWARRAY => {
                    let _atype = self.program[self.pc];
                    self.pc += 1;

                    emit!(";;-- NEWARRAY --");
                    emit!("pop rbx");
                    emit!("mov rcx, rbx");
                    emit!("imul rcx, 8");
                    emit!("add rcx, 8");

                    emit!("sub rsp, 32");
                    emit!("call alloc_stub");
                    emit!("add rsp, 32");

                    emit!("mov qword [rax], 99");
                    emit!("push rax")
                }
                IASTORE => {
                    emit!(";;-- IASTORE --");
                    emit!("pop rax");
                    emit!("pop rbx");
                    emit!("pop rcx");

                    emit!("cmp rcx, 0");
                    emit!("je npe_handler");

                    emit!("imul rbx, 8");
                    emit!("add rcx, 8");
                    emit!("add rcx, rbx");
                    emit!("mov [rcx], rax");
                }
                IALOAD => {
                    emit!(";;-- IALOAD --");
                    emit!("pop rbx");
                    emit!("pop rcx");

                    emit!("cmp rcx, 0");
                    emit!("je npe_handler");

                    emit!("imul rbx, 8");
                    emit!("add rcx, 8");
                    emit!("add rcx, rbx");
                    emit!("mov rax, [rcx]");
                    emit!("push rax");
                }
                ICONST_0 => {
                    emit!("push 0");
                }
                ICONST_1 => {
                    emit!("push 1");
                }
                ICONST_2 => {
                    emit!("push 2");
                }
                ICONST_3 => {
                    emit!("push 3");
                }
                ICONST_4 => {
                    emit!("push 4");
                }
                ICONST_5 => {
                    emit!("push 5");
                }
                ISTORE => {
                    let index = self.program[self.pc] as u8;
                    self.pc += 1;

                    emit!("pop rax");
                    emit!("mov [rbp-{}], rax", (index as usize + 1) * 8);
                }
                ISTORE_0 => {
                    emit!("pop rax");
                    emit!("mov [rbp-8], rax")
                }
                ISTORE_1 => {
                    emit!("pop rax");
                    emit!("mov [rbp-16], rax");
                }
                ISTORE_2 => {
                    emit!("pop rax");
                    emit!("mov [rbp-24], rax");
                }
                ISTORE_3 => {
                    emit!("pop rax");
                    emit!("mov [rbp-32], rax");
                }
                ILOAD => {
                    let index = self.program[self.pc] as u8;
                    self.pc += 1;

                    emit!("push qword [rbp-{}]", (index as usize + 1) * 8);
                }
                ILOAD_0 => {
                    emit!("push qword [rbp-8]");
                }
                ILOAD_1 => {
                    emit!("push qword [rbp-16]");
                }
                ILOAD_2 => {
                    emit!("push qword [rbp-24]");
                }
                ILOAD_3 => {
                    emit!("push qword [rbp-32]");
                }
                DUP => {
                    emit!(";;-- DUP --");
                    emit!("mov rax, [rsp]");
                    emit!("push rax");
                }
                PRINT => {
                    emit!(";;-- PRINT --");
                    emit!("pop rcx");
                    emit!("sub rsp, 32");
                    emit!("call print_integer_stub");
                    emit!("add rsp, 32");
                }
                NOP => {}
                LDC => {
                    let index = self.program[self.pc] as u8;
                    self.pc += 1;

                    match &self.class_file.constant_pool[index as usize] {
                        ConstantPoolEntry::Integer { bytes } => {
                            emit!("push {}", *bytes as i32);
                        }

                        ConstantPoolEntry::Float { bytes } => {
                            emit!("push {}", *bytes as i32);
                        }

                        ConstantPoolEntry::String { string_index } => {
                            if let Some(text) = self.class_file.get_utf8(*string_index) {
                                let label_name = format!("str_{}_{}", index, self.pc);

                                label!("section .data");
                                emit!("{}: db \"{}\", 0", label_name, text);

                                label!("section .text");
                                emit!("mov rax, {}", label_name);
                                emit!("push rax");
                            }
                        }

                        _ => {
                            self.log.debug(&format!(
                                "   > Unhandled LDC constant pool entry: {:?}",
                                self.class_file.constant_pool[index as usize]
                            ));
                            emit!("push 0");
                        }
                    }
                }
                GETSTATIC => {
                    let _indexbyte1 = self.program[self.pc];
                    let _indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    // Push 1 represents the System.outobject
                    // Prevents INVOKEVIRTUAL from throwing NPE when trying to print to console.
                    emit!("push 1");
                }
                INVOKEVIRTUAL => {
                    let _indexbyte1 = self.program[self.pc];
                    let _indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    let method_index = u16::from_be_bytes([_indexbyte1, _indexbyte2]) as usize;
                    let mut descriptor = String::new();

                    if let ConstantPoolEntry::Methodref {
                        name_and_type_index,
                        ..
                    } = &self.class_file.constant_pool[method_index]
                    {
                        if let ConstantPoolEntry::NameAndType {
                            descriptor_index, ..
                        } = &self.class_file.constant_pool[*name_and_type_index as usize]
                        {
                            if let Some(desc_str) = self.class_file.get_utf8(*descriptor_index) {
                                descriptor = desc_str;
                            }
                        }
                    }

                    emit!(";;-- INVOKEVIRTUAL ({}) --", descriptor);
                    emit!("pop rcx");
                    emit!("pop rdx");

                    emit!("cmp rdx, 0");
                    emit!("je npe_handler");

                    emit!("cmp rdx, 1");
                    emit!("je .do_print_{}", self.pc);

                    emit!("mov r8, [rdx]");

                    label!(".do_print_{}:", self.pc);
                    emit!("sub rsp, 32");

                    if descriptor == "(Ljava/lang/String;)V" {
                        emit!("call print_string_stub");
                    } else {
                        emit!("call print_integer_stub");
                    }

                    emit!("add rsp, 32");
                }
                INVOKESTATIC => {
                    let indexbyte1 = self.program[self.pc];
                    let indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    let cp_index = u16::from_be_bytes([indexbyte1, indexbyte2]) as usize;
                    let mut target_name = String::new();
                    let mut target_descriptor = String::new();

                    if let ConstantPoolEntry::Methodref {
                        name_and_type_index,
                        ..
                    } = &self.class_file.constant_pool[cp_index]
                    {
                        if let ConstantPoolEntry::NameAndType {
                            name_index,
                            descriptor_index,
                        } = &self.class_file.constant_pool[*name_and_type_index as usize]
                        {
                            target_name = self.class_file.get_utf8(*name_index).unwrap_or_default();
                            target_descriptor = self
                                .class_file
                                .get_utf8(*descriptor_index)
                                .unwrap_or_default();
                        }
                    }

                    let mut target_method_index = 0;
                    let mut found = false;

                    for (i, method) in self.class_file.methods.iter().enumerate() {
                        let name = self
                            .class_file
                            .get_utf8(method.name_index)
                            .unwrap_or_default();
                        let descriptor = self
                            .class_file
                            .get_utf8(method.descriptor_index)
                            .unwrap_or_default();

                        if name == target_name && descriptor == target_descriptor {
                            target_method_index = i;
                            found = true;
                            break;
                        }
                    }

                    if found {
                        emit!(";;-- INVOKESTATIC {} {} --", target_name, target_descriptor);
                        emit!("call method_{}", target_method_index);
                        if !target_descriptor.ends_with("V") {
                            emit!("push rax")
                        }
                    } else {
                        panic!("Method not found: {} {}", target_name, target_descriptor);
                    }
                }
                INVOKESPECIAL => {
                    self.pc += 2;
                    emit!(";;-- INVOKESPECIAL - skip --");
                    emit!("pop rax");
                }
                ASTORE => {
                    let index = self.program[self.pc] as u8;
                    self.pc += 1;

                    emit!("pop rax");
                    emit!("mov [rbp-{}], rax", (index as usize + 1) * 8);
                }
                ASTORE_0 => {
                    emit!(";;-- ASTORE_0 --");
                    emit!("pop rax");
                    emit!("mov [rbp-8], rax");
                }
                ASTORE_1 => {
                    emit!(";;-- ASTORE_1 --");
                    emit!("pop rax");
                    emit!("mov [rbp-16], rax");
                }
                ASTORE_2 => {
                    emit!(";;-- ASTORE_2 --");
                    emit!("pop rax");
                    emit!("mov [rbp-24], rax");
                }
                ASTORE_3 => {
                    emit!(";;-- ASTORE_3 --");
                    emit!("pop rax");
                    emit!("mov [rbp-32], rax");
                }
                ALOAD => {
                    let index = self.program[self.pc] as u8;
                    self.pc += 1;

                    emit!("push qword [rbp-{}]", (index as usize + 1) * 8);
                }
                ALOAD_0 => {
                    emit!("push qword [rbp-8]");
                }
                ALOAD_1 => {
                    emit!("push qword [rbp-16]");
                }
                ALOAD_2 => {
                    emit!("push qword [rbp-24]");
                }
                ALOAD_3 => {
                    emit!("push qword [rbp-32]");
                }
                ACONST_NULL => {
                    emit!(";;-- ACONST_NULL --");
                    emit!("push 0");
                }
                SIPUSH => {
                    let byte1 = self.program[self.pc];
                    let byte2 = self.program[self.pc + 1];
                    self.pc += 2;
                    let val = i16::from_be_bytes([byte1, byte2]) as i32;
                    emit!("push {}", val);
                }
                GOTO => {
                    let branchbyte1 = self.program[self.pc];
                    let branchbyte2 = self.program[self.pc + 1];
                    self.pc += 2;
                    let offset = u16::from_be_bytes([branchbyte1, branchbyte2]) as i16;

                    emit!(";;-- GOTO --");
                    emit!(
                        "jmp {}_label_{}",
                        self.current_method_name,
                        self.pc as isize + offset as isize - 3
                    );
                }
                CALL => {
                    let indexbyte1 = self.program[self.pc];
                    let indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;
                    self.pc += 1; // skips carg for now.

                    let cp_index = u16::from_be_bytes([indexbyte1, indexbyte2]) as usize;
                    let mut target_name = String::new();
                    let mut target_descriptor = String::new();

                    if let ConstantPoolEntry::Methodref {
                        name_and_type_index,
                        ..
                    } = &self.class_file.constant_pool[cp_index]
                    {
                        if let ConstantPoolEntry::NameAndType {
                            name_index,
                            descriptor_index,
                        } = &self.class_file.constant_pool[*name_and_type_index as usize]
                        {
                            target_name = self.class_file.get_utf8(*name_index).unwrap_or_default();
                            target_descriptor = self
                                .class_file
                                .get_utf8(*descriptor_index)
                                .unwrap_or_default();
                        }
                    }

                    let mut target_method_index = 0;
                    let mut found = false;

                    for (i, method) in self.class_file.methods.iter().enumerate() {
                        let name = self
                            .class_file
                            .get_utf8(method.name_index)
                            .unwrap_or_default();
                        let descriptor = self
                            .class_file
                            .get_utf8(method.descriptor_index)
                            .unwrap_or_default();

                        if name == target_name && descriptor == target_descriptor {
                            target_method_index = i;
                            found = true;
                            break;
                        }
                    }

                    if found {
                        emit!(";;-- CALL {} {} --", target_name, target_descriptor);
                        emit!("call method_{}", target_method_index);
                        if !target_descriptor.ends_with("V") {
                            emit!("push rax")
                        }
                    } else {
                        panic!("Method not found: {} {}", target_name, target_descriptor);
                    }
                }
                NEW => {
                    let indexbyte1 = self.program[self.pc];
                    let indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    let class_index = u16::from_be_bytes([indexbyte1, indexbyte2]);

                    let mut class_name = String::from("Unknown");
                    if let ConstantPoolEntry::Class { name_index } =
                        &self.class_file.constant_pool[class_index as usize]
                    {
                        if let Some(name) = self.class_file.get_utf8(*name_index) {
                            class_name = name;
                        }
                    }

                    let object_size = self.calc_object_size(&class_name);

                    emit!(";;-- NEW --");
                    emit!("mov rcx, {}", object_size);
                    emit!("sub rsp, 32");
                    emit!("call alloc_stub");
                    emit!("add rsp, 32");

                    emit!("mov qword [rax], {}", class_index);
                    emit!("push rax");
                }
                GETFIELD => {
                    let indexbyte1 = self.program[self.pc];
                    let indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    let cp_index = u16::from_be_bytes([indexbyte1, indexbyte2]);

                    let mut class_name = String::new();
                    let mut field_name = String::new();

                    if let ConstantPoolEntry::Fieldref {
                        class_index,
                        name_and_type_index,
                    } = &self.class_file.constant_pool[cp_index as usize]
                    {
                        if let ConstantPoolEntry::Class { name_index } =
                            &self.class_file.constant_pool[*class_index as usize]
                        {
                            if let Some(name) = self.class_file.get_utf8(*name_index) {
                                class_name = name;
                            }
                        }

                        if let ConstantPoolEntry::NameAndType { name_index, .. } =
                            &self.class_file.constant_pool[*name_and_type_index as usize]
                        {
                            if let Some(name) = self.class_file.get_utf8(*name_index) {
                                field_name = name;
                            }
                        }
                    }

                    let offset = self.calc_field_offset(&class_name, &field_name);

                    emit!(
                        ";;-- GETFIELD {}.{} (Offset {}) --",
                        class_name,
                        field_name,
                        offset
                    );
                    emit!("pop rcx");

                    emit!("cmp rcx, 0");
                    emit!("je npe_handler");

                    emit!("mov rax, [rcx + {}]", offset);
                    emit!("push rax");
                }
                PUTFIELD => {
                    let indexbyte1 = self.program[self.pc];
                    let indexbyte2 = self.program[self.pc + 1];
                    self.pc += 2;

                    let cp_index = u16::from_be_bytes([indexbyte1, indexbyte2]);

                    let mut class_name = String::new();
                    let mut field_name = String::new();

                    if let ConstantPoolEntry::Fieldref {
                        class_index,
                        name_and_type_index,
                    } = &self.class_file.constant_pool[cp_index as usize]
                    {
                        if let ConstantPoolEntry::Class { name_index } =
                            &self.class_file.constant_pool[*class_index as usize]
                        {
                            if let Some(name) = self.class_file.get_utf8(*name_index) {
                                class_name = name;
                            }
                        }

                        if let ConstantPoolEntry::NameAndType { name_index, .. } =
                            &self.class_file.constant_pool[*name_and_type_index as usize]
                        {
                            if let Some(name) = self.class_file.get_utf8(*name_index) {
                                field_name = name;
                            }
                        }
                    }

                    let offset = self.calc_field_offset(&class_name, &field_name);

                    emit!(
                        ";;-- PUTFIELD {}.{} Offset {} --",
                        class_name,
                        field_name,
                        offset
                    );
                    emit!("pop rax");
                    emit!("pop rcx");

                    emit!("cmp rcx, 0");
                    emit!("je npe_handler");

                    emit!("mov [rcx + {}], rax", offset);
                }
                HALT => {
                    emit!(";;-- HALT --");
                    emit!("mov rcx, 0");
                    emit!("sub rsp, 32");
                    emit!("call exit_stub");
                    break;
                }
                IRETURN => {
                    emit!(";;-- IRETURN --");
                    emit!("pop rax");
                    emit!("mov rsp, rbp");
                    emit!("pop rbp");
                    emit!("ret");
                }
                RETURN => {
                    if self.current_method_name == "main" {
                        emit!(";;-- RETURN from main --");
                        emit!("mov rcx, 0");
                        emit!("sub rsp, 32");
                        emit!("call exit_stub");
                    } else {
                        emit!(";;-- RETURN --");
                        emit!("mov rsp, rbp");
                        emit!("pop rbp");
                        emit!("ret");
                    }
                }
                _ => {
                    self.log.debug(&format!(
                        "   > Unhandled opcode in compilation: {:#04X} {}",
                        opcode,
                        opcode_to_name(opcode)
                    ));
                }
            }
        }
    }

    fn calc_object_size(&mut self, class_name: &str) -> usize {
        if let Some(&size) = self.class_size_cache.get(class_name) {
            return size;
        } else {
            let path = format!("./src/tests/{}.class", class_name);
            let file_date = match fs::read(&path) {
                Ok(data) => data,
                Err(_) => {
                    self.log.debug(&format!(
                        "Failed to read class file for size calculation: {}",
                        path
                    ));
                    return 0;
                }
            };

            let cursor = Cursor::new(file_date);
            let class_file = ClassFile::parse(cursor);

            let super_class_index = class_file.super_class;

            let mut super_name = String::new();
            if let ConstantPoolEntry::Class { name_index } =
                &class_file.constant_pool[super_class_index as usize]
            {
                if let Some(name) = class_file.get_utf8(*name_index) {
                    super_name = name;
                }
            }

            let mut size = if super_name == "java/lang/Object" {
                8
            } else {
                self.calc_object_size(&super_name)
            };

            for field in &class_file.fields {
                if field.access_flags & 0x0008 == 0 {
                    size += 8;
                }
            }

            self.log.info(&format!(
                "   > Resolved Layout: {} = {} bytes",
                class_name, size
            ));
            self.class_size_cache.insert(class_name.to_string(), size);

            size
        }
    }

    fn calc_field_offset(&mut self, class_name: &str, field_name: &str) -> usize {
        let path = format!("./src/tests/{}.class", class_name);
        let file_data = match fs::read(&path) {
            Ok(data) => data,
            Err(_) => {
                self.log.debug(&format!(
                    "Failed to read class file for field offset calculation: {}",
                    path
                ));
                return 0;
            }
        };

        let cursor = Cursor::new(file_data);
        let class_file = ClassFile::parse(cursor);
        let super_class_index = class_file.super_class;

        let mut super_name = String::new();
        if let ConstantPoolEntry::Class { name_index } =
            &class_file.constant_pool[super_class_index as usize]
        {
            if let Some(name) = class_file.get_utf8(*name_index) {
                super_name = name;
            }
        }

        let start_offset = if super_name == "java/lang/Object" {
            8
        } else {
            self.calc_object_size(&super_name)
        };

        let mut current_offset = start_offset;

        for field in &class_file.fields {
            if field.access_flags & 0x0008 != 0 {
                continue;
            }

            let iter_field_name = class_file.get_utf8(field.name_index).unwrap_or_default();
            if iter_field_name == field_name {
                self.log.info(&format!(
                    "   > Field {} found in class {} at offset {}",
                    field_name, class_name, current_offset
                ));
                return current_offset;
            }
            current_offset += 8;
        }

        if !super_name.is_empty() && super_name != "java/lang/Object" {
            return self.calc_field_offset(&super_name, field_name);
        }

        panic!("Field {} not found in class {}", field_name, class_name);
    }
}

fn main() {
    let args = Args::parse();

    let log = Logger::new(LOG_LEVEL);
    let cursor = Cursor::new(fs::read(&args.file).expect("Failed to read class file"));
    let class_file = ClassFile::parse(cursor);
    let mode = args.mode.as_str(); // "interpret" or "compile"

    log.info("Class File:");
    log.info(&format!("  Minor Version: {}", class_file.minor_version));
    log.info(&format!("  Major Version: {}", class_file.major_version));
    log.info(&format!(
        "  Constant Pool Count: {}",
        class_file.constant_pool_count
    ));
    for (i, entry) in class_file.constant_pool.iter().enumerate() {
        log.info(&format!("    #{}: {:?}", i, entry));
    }
    log.info(&format!("  Access Flags: {}", class_file.access_flags));
    log.info(&format!("  This Class: {}", class_file.this_class));
    log.info(&format!("  Super Class: {}", class_file.super_class));
    log.info(&format!(
        "  Interfaces Count: {}",
        class_file.interfaces_count
    ));
    for (i, interface) in class_file.interfaces.iter().enumerate() {
        log.info(&format!("    #{}: {}", i, interface));
    }
    log.info(&format!("  Fields Count: {}", class_file.fields_count));
    for (i, field) in class_file.fields.iter().enumerate() {
        log.info(&format!("    #{}: {:?}", i, field));
    }
    log.info(&format!("  Methods Count: {}", class_file.methods_count));

    if mode == "interpret" {
        for (i, method) in class_file.methods.iter().enumerate() {
            let name = class_file
                .get_utf8(method.name_index)
                .unwrap_or("Invalid UTF-8".to_string());
            let descriptor = class_file
                .get_utf8(method.descriptor_index)
                .unwrap_or("Invalid UTF-8".to_string());
            log.info(&format!(
                "    #{}: {:?}, Name: {}, Descriptor: {}",
                i, method, name, descriptor
            ));

            if Some(name.as_str()) == Some("main") {
                log.info("      Found main method!");

                for attribute in &method.attributes {
                    let attribute_name = class_file
                        .get_utf8(attribute.attribute_name_index)
                        .unwrap_or("Invalid UTF-8".to_string());
                    if attribute_name == "Code" {
                        log.info("      Found Code attribute in main method!");
                        let code_attribute = attribute;
                        let code_length = u32::from_be_bytes([
                            code_attribute.info[4],
                            code_attribute.info[5],
                            code_attribute.info[6],
                            code_attribute.info[7],
                        ]) as usize;
                        let code_start = 8;
                        let code_end = code_start + code_length;
                        let code_bytes = &code_attribute.info[code_start..code_end];
                        log.info(&format!("      Code bytes: {:?}", code_bytes));

                        let mut vm =
                            VM::new(code_bytes.to_vec(), &class_file, "main".to_string(), 0);
                        vm.exec();
                    }
                }
            }
        }
    } else {
        label!("section .text");
        label!("global main");
        label!("extern print_integer_stub");
        label!("extern print_string_stub");
        label!("extern exit_stub");
        label!("extern alloc_stub");
        label!("extern null_pointer_exception");

        label!("npe_handler:");
        emit!("sub rsp, 32");
        emit!("call null_pointer_exception");
        emit!("add rsp, 32");
        emit!("ret");

        for (i, method) in class_file.methods.iter().enumerate() {
            let name = class_file
                .get_utf8(method.name_index)
                .unwrap_or("Invalid UTF-8".to_string());
            let descriptor = class_file
                .get_utf8(method.descriptor_index)
                .unwrap_or("Invalid UTF-8".to_string());
            log.info(&format!(
                "    #{}: {:?}, Name: {}, Descriptor: {}",
                i, method, name, descriptor
            ));
            let mut arg_count = 0;
            let mut chars = descriptor.chars();
            if chars.next() == Some('(') {
                while let Some(c) = chars.next() {
                    if c == ')' {
                        break;
                    }
                    match c {
                        'L' => {
                            while let Some(nc) = chars.next() {
                                if nc == ';' {
                                    break;
                                }
                            }
                            arg_count += 1;
                        }
                        'I' | 'F' | 'D' | 'J' | 'B' | 'C' | 'S' | 'Z' => {
                            arg_count += 1;
                        }
                        _ => {}
                    }
                }
            }

            if name == "main" {
                arg_count = 0;
                emit!("main:");
            } else {
                emit!("method_{}:", i);
            }

            for attribute in &method.attributes {
                let attribute_name = class_file
                    .get_utf8(attribute.attribute_name_index)
                    .unwrap_or("Invalid UTF-8".to_string());
                if attribute_name == "Code" {
                    let code_attribute = attribute;
                    let code_length = u32::from_be_bytes([
                        code_attribute.info[4],
                        code_attribute.info[5],
                        code_attribute.info[6],
                        code_attribute.info[7],
                    ]) as usize;
                    let code_start = 8;
                    let code_end = code_start + code_length;
                    let code_bytes = &code_attribute.info[code_start..code_end];
                    log.info(&format!("      Code bytes: {:?}", code_bytes));

                    let unique_label = if name == "main" {
                        "main".to_string()
                    } else {
                        format!("method_{}", i)
                    };

                    let mut vm = VM::new(code_bytes.to_vec(), &class_file, unique_label, arg_count);
                    vm.compile();
                }
            }
        }
    }

    log.info(&format!(
        "  Attributes Count: {}",
        class_file.attributes_count
    ));
    for (i, attribute) in class_file.attributes.iter().enumerate() {
        log.info(&format!("    #{}: {:?}", i, attribute));
    }
}
