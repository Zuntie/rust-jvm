use crate::cursor::Cursor;
const MAGIC_NUMBER: u32 = 0xCAFEBABE;

pub struct ClassFile {
    pub magic: u32,
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool_count: u16,
    pub constant_pool: Vec<ConstantPoolEntry>,
    pub access_flags: u16,
    pub this_class: u16,
    pub super_class: u16,
    pub interfaces_count: u16,
    pub interfaces: Vec<u16>,
    pub fields_count: u16,
    pub fields: Vec<FieldInfo>,
    pub methods_count: u16,
    pub methods: Vec<MethodInfo>,
    pub attributes_count: u16,
    pub attributes: Vec<AttributeInfo>,
}

#[derive(Debug)]
pub enum ConstantPoolEntry {
    Class {
        name_index: u16,
    },
    Fieldref {
        class_index: u16,
        name_and_type_index: u16,
    },
    Methodref {
        class_index: u16,
        name_and_type_index: u16,
    },
    InterfaceMethodref {
        class_index: u16,
        name_and_type_index: u16,
    },
    String {
        string_index: u16,
    },
    Integer {
        bytes: u32,
    },
    Float {
        bytes: u32,
    },
    Long {
        high_bytes: u32,
        low_bytes: u32,
    },
    Double {
        high_bytes: u32,
        low_bytes: u32,
    },
    NameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    Utf8 {
        length: u16,
        bytes: Vec<u8>,
    },
    MethodHandle {
        reference_kind: u8,
        reference_index: u16,
    },
    MethodType {
        descriptor_index: u16,
    },
    InvokeDynamic {
        bootstrap_method_attr_index: u16,
        name_and_type_index: u16,
    },
}

#[derive(Debug)]
pub struct FieldInfo {
    pub access_flags: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes_count: u16,
    pub attributes: Vec<AttributeInfo>,
}

#[derive(Debug)]
pub struct MethodInfo {
    pub access_flags: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes_count: u16,
    pub attributes: Vec<AttributeInfo>,
}

#[derive(Debug)]
pub struct AttributeInfo {
    pub attribute_name_index: u16,
    pub attribute_length: u32,
    pub info: Vec<u8>,
}

#[derive(Debug)]
pub struct CodeAttribute {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
}

impl ClassFile {
    pub fn parse(mut cursor: Cursor) -> ClassFile {
        let magic = cursor.read_u32();
        if magic != MAGIC_NUMBER {
            panic!("Invalid class file: incorrect magic number");
        }

        let minor_version = cursor.read_u16();
        let major_version = cursor.read_u16();

        let constant_pool_count = cursor.read_u16();
        let mut constant_pool = Vec::new();
        constant_pool.push(ConstantPoolEntry::Class { name_index: 0 });

        let mut i = 1;
        while i < constant_pool_count {
            let tag = cursor.read_u8();

            // https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html#jvms-4.4
            match tag {
                7 => {
                    // Class
                    let name_index = cursor.read_u16();
                    constant_pool.push(ConstantPoolEntry::Class { name_index });
                }
                10 => {
                    // Methodref
                    let class_index = cursor.read_u16();
                    let name_and_type_index = cursor.read_u16();
                    constant_pool.push(ConstantPoolEntry::Methodref {
                        class_index,
                        name_and_type_index,
                    });
                }
                9 => {
                    // Fieldref
                    let class_index = cursor.read_u16();
                    let name_and_type_index = cursor.read_u16();
                    constant_pool.push(ConstantPoolEntry::Fieldref {
                        class_index,
                        name_and_type_index,
                    });
                }
                8 => {
                    // String
                    let string_index = cursor.read_u16();
                    constant_pool.push(ConstantPoolEntry::String { string_index });
                }
                12 => {
                    // NameAndType
                    let name_index = cursor.read_u16();
                    let descriptor_index = cursor.read_u16();
                    constant_pool.push(ConstantPoolEntry::NameAndType {
                        name_index,
                        descriptor_index,
                    });
                }
                5 => {
                    // Long
                    let high_bytes = cursor.read_u32();
                    let low_bytes = cursor.read_u32();
                    constant_pool.push(ConstantPoolEntry::Long {
                        high_bytes,
                        low_bytes,
                    });
                    constant_pool.push(ConstantPoolEntry::Class { name_index: 0 });
                    i += 1;
                }
                6 => {
                    // Double
                    let high_bytes = cursor.read_u32();
                    let low_bytes = cursor.read_u32();
                    constant_pool.push(ConstantPoolEntry::Double {
                        high_bytes,
                        low_bytes,
                    });
                    constant_pool.push(ConstantPoolEntry::Class { name_index: 0 });
                    i += 1;
                }
                3 => {
                    // Integer
                    let bytes = cursor.read_u32();
                    constant_pool.push(ConstantPoolEntry::Integer { bytes });
                }
                4 => {
                    // Float
                    let bytes = cursor.read_u32();
                    constant_pool.push(ConstantPoolEntry::Float { bytes });
                }
                1 => {
                    // Utf8
                    let length = cursor.read_u16();
                    let mut bytes = Vec::new();
                    for _ in 0..length {
                        bytes.push(cursor.read_u8());
                    }
                    constant_pool.push(ConstantPoolEntry::Utf8 { length, bytes });
                }
                18 => {
                    // InvokeDynamic
                    let bootstrap_method_attr_index = cursor.read_u16();
                    let name_and_type_index = cursor.read_u16();
                    constant_pool.push(ConstantPoolEntry::InvokeDynamic {
                        bootstrap_method_attr_index,
                        name_and_type_index,
                    });
                }
                15 => {
                    // MethodHandle
                    let reference_kind = cursor.read_u8();
                    let reference_index = cursor.read_u16();
                    constant_pool.push(ConstantPoolEntry::MethodHandle {
                        reference_kind,
                        reference_index,
                    });
                }
                11 => {
                    // InterfaceMethodref
                    let class_index = cursor.read_u16();
                    let name_and_type_index = cursor.read_u16();
                    constant_pool.push(ConstantPoolEntry::InterfaceMethodref {
                        class_index,
                        name_and_type_index,
                    });
                }
                _ => {
                    panic!("Unknown constant pool tag: {}", tag);
                }
            }
            i += 1;
        }

        let access_flags = cursor.read_u16();
        let this_class = cursor.read_u16();
        let super_class = cursor.read_u16();

        let interfaces_count = cursor.read_u16();
        let mut interfaces = Vec::new();
        for _ in 0..interfaces_count {
            interfaces.push(cursor.read_u16());
        }

        let fields_count = cursor.read_u16();
        let mut fields: Vec<FieldInfo> = Vec::new();
        for _ in 0..fields_count {
            fields.push(FieldInfo::parse(&mut cursor));
        }

        let methods_count = cursor.read_u16();
        let mut methods: Vec<MethodInfo> = Vec::new();
        for _ in 0..methods_count {
            methods.push(MethodInfo::parse(&mut cursor));
        }

        let attributes_count = cursor.read_u16();
        let mut attributes: Vec<AttributeInfo> = Vec::new();
        for _ in 0..attributes_count {
            attributes.push(AttributeInfo::parse(&mut cursor));
        }

        ClassFile {
            magic,
            minor_version,
            major_version,
            constant_pool_count,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces_count,
            interfaces,
            fields_count,
            fields,
            methods_count,
            methods,
            attributes_count,
            attributes,
        }
    }

    pub fn get_utf8(&self, index: u16) -> Option<String> {
        if (index as usize) >= self.constant_pool.len() {
            return None;
        }

        match &self.constant_pool[index as usize] {
            ConstantPoolEntry::Utf8 { length: _, bytes } => {
                Some(String::from_utf8_lossy(bytes).to_string())
            }

            ConstantPoolEntry::String { string_index } => self.get_utf8(*string_index),

            _ => None,
        }
    }
}

impl FieldInfo {
    pub fn parse(cursor: &mut Cursor) -> FieldInfo {
        let access_flags = cursor.read_u16();
        let name_index = cursor.read_u16();
        let descriptor_index = cursor.read_u16();
        let attributes_count = cursor.read_u16();
        let mut attributes: Vec<AttributeInfo> = Vec::new();

        for _ in 0..attributes_count {
            attributes.push(AttributeInfo::parse(cursor));
        }

        FieldInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes_count,
            attributes,
        }
    }
}

impl MethodInfo {
    pub fn parse(cursor: &mut Cursor) -> MethodInfo {
        let access_flags = cursor.read_u16();
        let name_index = cursor.read_u16();
        let descriptor_index = cursor.read_u16();
        let attributes_count = cursor.read_u16();
        let mut attributes: Vec<AttributeInfo> = Vec::new();

        for _ in 0..attributes_count {
            attributes.push(AttributeInfo::parse(cursor));
        }

        MethodInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes_count,
            attributes,
        }
    }
}

impl AttributeInfo {
    pub fn parse(cursor: &mut Cursor) -> AttributeInfo {
        let attribute_name_index = cursor.read_u16();
        let attribute_length = cursor.read_u32();
        let mut info: Vec<u8> = Vec::new();
        for _ in 0..attribute_length {
            info.push(cursor.read_u8());
        }
        AttributeInfo {
            attribute_name_index,
            attribute_length,
            info,
        }
    }
}

impl CodeAttribute {
    pub fn parse(info: &Vec<u8>) -> CodeAttribute {
        let mut cursor = Cursor::new(info.clone());

        let max_stack = cursor.read_u16();
        let max_locals = cursor.read_u16();
        let code_length = cursor.read_u32();

        let mut code = Vec::new();
        for _ in 0..code_length {
            code.push(cursor.read_u8());
        }

        CodeAttribute {
            max_stack,
            max_locals,
            code,
        }
    }
}
