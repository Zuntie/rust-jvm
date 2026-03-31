pub struct Cursor {
    data: Vec<u8>,
    position: usize,
}

impl Cursor {
    pub fn new(data: Vec<u8>) -> Self {
        Cursor { data, position: 0 }
    }

    pub fn read_u8(&mut self) -> u8 {
        let val = self.data[self.position];
        self.position += 1;
        val
    }

    pub fn read_u16(&mut self) -> u16 {
        let val = u16::from_be_bytes([self.data[self.position], self.data[self.position + 1]]);
        self.position += 2;
        val
    }

    pub fn read_u32(&mut self) -> u32 {
        let val = u32::from_be_bytes([
            self.data[self.position],
            self.data[self.position + 1],
            self.data[self.position + 2],
            self.data[self.position + 3],
        ]);
        self.position += 4;
        val
    }
}
