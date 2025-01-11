use super::mp3utils;

pub struct Xing {
    start: u32,
    field_num: u8,
    xing_extensions: [bool; 4],
    byte_quantity: Option<i32>,
    frame_quantity: Option<i32>,
    quality: Option<u8>,
}

impl Xing {
    fn new(buffer: &[u8], offset: u32) -> Self {
        let mut xing = Xing {
            start: 0,
            field_num: 0,
            xing_extensions: [false; 4],
            byte_quantity: None,
            frame_quantity: None,
            quality: None,
        };

        let mut id = String::new();

        let mut offset = offset;
        loop {
            if buffer[offset as usize] == b'I' || buffer[offset as usize] == b'X' {
                for byte in 0..4 {
                    id.push(buffer[(offset + byte) as usize] as char);
                }

                if id == "Info" || id == "Xing" {
                    xing.start = offset + 4;
                    xing.set_xing_extensions(buffer);

                    if xing.xing_extensions[1] {
                        xing.set_frame_quantity(buffer);
                    }
                    if xing.xing_extensions[0] {
                        xing.set_byte_quantity(buffer);
                    }
                    if xing.xing_extensions[2] {
                        xing.set_quality(buffer);
                    }
                    break;
                }
            } else if buffer[offset as usize] == 0xFF && buffer[(offset + 1) as usize] >= 0xE0 {
                break;
            }

            offset += 1;
        }

        xing
    }

    fn set_xing_extensions(&mut self, buffer: &[u8]) {
        let flag_byte = buffer[(self.start + 3) as usize];

        for bit_num in 0..4{
            self.xing_extensions[bit_num] = (flag_byte >> bit_num) & 1 == 1;
        }
    }

    fn set_frame_quantity(&mut self, buffer: &[u8]) {
        self.frame_quantity = Some(mp3utils::char_to_int(&buffer[(self.start + self.field_num as u32 * 4) as usize..]) as i32);
        self.field_num += 1;
    }

    fn get_frame_quantity(&self) -> Option<i32> {
        self.frame_quantity
    }

    fn set_byte_quantity(&mut self, buffer: &[u8]) {
        self.byte_quantity = Some(mp3utils::char_to_int(&buffer[(self.start + self.field_num as u32 * 4) as usize..]) as i32);
        self.field_num += 1;
    }

    fn get_byte_quantity(&self) -> Option<i32> {
        self.byte_quantity
    }

    fn set_quality(&mut self, buffer: &[u8]) {
        let size = if self.xing_extensions[Extensions::TOC as usize] { 100 } else { 0 };
        self.quality = Some(buffer[(self.start + size + self.field_num as u32 * 4 + 3) as usize]);
    }

    fn get_quality(&self) -> Option<u8> {
        self.quality
    }
}

enum Extensions {
    FrameField = 0,
    ByteField = 1,
    TOC = 2,
    Quality = 3,
}