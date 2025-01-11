use regex::Regex;
use std::str;

#[derive(Debug, Clone)]
pub struct Id3 {
    buffer: Vec<u8>,
    valid: bool,
    version: String,
    offset: usize,
    id3_flags: [bool; 4],
    extended_header_size: usize,
    id3_frames: [Vec<String>; 2],
}

impl Id3 {
    pub fn new(buffer: Vec<u8>) -> Self {
        let mut id3 = Id3 {
            buffer,
            valid: false,
            version: String::new(),
            offset: 0,
            id3_flags: [false; 4],
            extended_header_size: 0,
            id3_frames: [Vec::new(), Vec::new()],
        };

        if id3.buffer.get(0) == Some(&b'I')
            && id3.buffer.get(1) == Some(&b'D')
            && id3.buffer.get(2) == Some(&b'3')
        {
            id3.set_version(id3.buffer[3], id3.buffer[4]);
            if id3.set_flags(id3.buffer[5]) {
                id3.valid = true;
                id3.set_offset(id3.char_to_int(&id3.buffer[6..10]));
                id3.set_extended_header_size(id3.char_to_int(&id3.buffer[10..14]));
                id3.set_fields(&id3.buffer[10 + id3.extended_header_size..]);
            }
        }
        id3
    }
    
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    pub fn get_id3_version(&self) -> String {
        self.version.clone()
    }

    pub fn get_id3_flags(&self) -> [bool; 4] {
        self.id3_flags.clone()
    }

    pub fn get_id3_offset(&self) -> usize {
        self.offset
    }

    pub fn get_id3_extended_header_size(&self) -> usize {
        self.extended_header_size
    }

    pub fn get_id3_fields(&self) -> &[Vec<String>; 2] {
        &self.id3_frames
    }

    pub fn get_id3_fields_length(&self) -> usize {
        self.id3_frames[1].len()
    }

    fn set_version(&mut self, version: u8, revision: u8) {
        self.version = format!("2.{}.{}", version, revision);
    }

    fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    fn set_extended_header_size(&mut self, size: usize) {
        self.extended_header_size = size;
    }

    fn set_flags(&mut self, flags: u8) -> bool {
        for bit_num in 0..4 {
            if flags >> bit_num & 1 != 0 {
                return false;
            }
        }

        for bit_num in 4..8 {
            self.id3_flags[bit_num - 4] = flags >> bit_num & 1 != 0;
        }

        true
    }

    fn set_fields(&mut self, buffer: &[u8]) {
        let footer_size = if self.id3_flags[flags::FOOTER_PRESENT] { 10 } else { 0 };
        let size = self.offset - self.extended_header_size - footer_size;
        let mut i = 0;
    
        let _re = Regex::new("[A-Z0-9]").unwrap();
    
        while i < size {
            let id = str::from_utf8(&buffer[i..i + 4]).unwrap().to_string();
            self.id3_frames[0].push(id.clone());
    
            i += 4;
            let field_size = self.char_to_int(&buffer[i..i + 4]);
            i += 6;
    
            let content = str::from_utf8(&buffer[i..i + field_size]).unwrap().to_string();
            self.id3_frames[1].push(content);
    
            i += field_size;
        }
    }

    fn char_to_int(&self, buffer: &[u8]) -> usize {
        let buffer_str = str::from_utf8(buffer).unwrap();
        buffer_str.trim().parse().unwrap()
    }
}

#[allow(dead_code)]
mod flags {
    pub const FOOTER_PRESENT: usize = 0;
    pub const EXPERIMENTAL_INDICATOR: usize = 1;
    pub const EXTENDED_HEADER: usize = 2;
    pub const UNSYNCHRONISATION: usize = 3; 
}
