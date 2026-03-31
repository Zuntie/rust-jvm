#[derive(Debug, Clone)]
pub struct StackFrame {
    pub pc: usize,
    pub fp: usize,
    pub program: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub class_index: u16,
    pub fields: Vec<(u16, StackValue)>,
    pub marked: bool,
}

#[derive(Debug, Clone)]
pub struct ArrayObject {
    pub class_index: u16,
    pub elements: Vec<StackValue>,
    pub marked: bool,
}

#[derive(Debug, Clone)]
pub enum HeapObject {
    Object(Object),
    Array(ArrayObject),
    Free,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StackValue {
    Int(i32),
    Ref(usize),
}
