use std::vec::Vec;
use super::tables;
use crate::codecs::audio::mp3::mp3utils::get_bits_inc;
use crate::codecs::audio::mp3::mp3utils::get_bits;
use std::f64::consts::PI;

const SQRT2: f64 = std::f64::consts::SQRT_2;
const NUM_PREV_FRAMES: usize = 9;

pub struct Mp3 {
    buffer: Vec<u8>,
    valid: bool,
    mpeg_version: f32,
    layer: u8,
    crc: bool,
    bit_rate: u32,
    sampling_rate: u32,
    padding: bool,
    channel_mode: ChannelMode,
    channels: i32,
    mode_extension: [u32; 2],
    emphasis: Emphasis,
    info: [bool; 3],
    band_index: BandIndex,
    band_width: BandWidth,
    prev_frame_size: [usize; NUM_PREV_FRAMES],
    frame_size: i32,
    main_data_begin: i32,
    scfsi: [[bool; 4]; 2],
    part2_3_length: [[i32; 2]; 2],
    part2_length: [[i32; 2]; 2],
    big_value: [[i32; 2]; 2],
    global_gain: [[i32; 2]; 2],
    scalefac_compress: [[i32; 2]; 2],
    slen1: [[i32; 2]; 2],
    slen2: [[i32; 2]; 2],
    window_switching: [[bool; 2]; 2],
    block_type: [[i32; 2]; 2],
    mixed_block_flag: [[bool; 2]; 2],
    switch_point_l: [[i32; 2]; 2],
    switch_point_s: [[i32; 2]; 2],
    table_select: [[[i32; 3]; 2]; 2],
    subblock_gain: [[[i32; 3]; 2]; 2],
    region0_count: [[i32; 2]; 2],
    region1_count: [[i32; 2]; 2],
    preflag: [[i32; 2]; 2],
    scalefac_scale: [[i32; 2]; 2],
    count1table_select: [[i32; 2]; 2],
    scalefac_l: [[[i32; 22]; 2]; 2],
    scalefac_s: [[[[i32; 13]; 3]; 2]; 2],
    prev_samples: [[[f32; 18]; 32]; 2],
    fifo: [[f32; 1024]; 2],
    main_data: Vec<u8>,
    samples: [[[f32; 576]; 2]; 2],
    pcm: [f32; 576 * 4],
}

impl Default for Mp3 {
    fn default() -> Self {
        Mp3 {
            buffer: Vec::new(),
            valid: false,
            mpeg_version: 1.0,
            layer: 0,
            crc: false,
            bit_rate: 0,
            sampling_rate: 0,
            padding: false,
            channel_mode: ChannelMode::Stereo,
            channels: 2,
            mode_extension: [0, 0],
            emphasis: Emphasis::None,
            info: [false; 3],
            band_index: BandIndex {
                long_win: &[], // Provide actual values here
                short_win: &[], // Provide actual values here
            },
            band_width: BandWidth {
                long_win: &[], // Provide actual values here
                short_win: &[], // Provide actual values here
            },
            prev_frame_size: [0; 9],
            frame_size: 0,
            main_data_begin: 0,
            scfsi: [[false; 4]; 2],
            part2_3_length: [[0; 2]; 2],
            part2_length: [[0; 2]; 2],
            big_value: [[0; 2]; 2],
            global_gain: [[0; 2]; 2],
            scalefac_compress: [[0; 2]; 2],
            slen1: [[0; 2]; 2],
            slen2: [[0; 2]; 2],
            window_switching: [[false; 2]; 2],
            block_type: [[0; 2]; 2],
            mixed_block_flag: [[false; 2]; 2],
            switch_point_l: [[0; 2]; 2],
            switch_point_s: [[0; 2]; 2],
            table_select: [[[0; 3]; 2]; 2],
            subblock_gain: [[[0; 3]; 2]; 2],
            region0_count: [[0; 2]; 2],
            region1_count: [[0; 2]; 2],
            preflag: [[0; 2]; 2],
            scalefac_scale: [[0; 2]; 2],
            count1table_select: [[0; 2]; 2],
            scalefac_l: [[[0; 22]; 2]; 2],
            scalefac_s: [[[[0; 13]; 3]; 2]; 2],
            prev_samples: [[[0.0; 18]; 32]; 2],
            fifo: [[0.0; 1024]; 2],
            main_data: Vec::new(),
            samples: [[[0.0; 576]; 2]; 2],
            pcm: [0.0; 576 * 4],
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ChannelMode {
    Stereo = 0,
    JointStereo = 1,
    DualChannel = 2,
    Mono = 3,
}

impl From<u8> for ChannelMode {
    fn from(value: u8) -> Self {
        match value {
            0 => ChannelMode::Stereo,
            1 => ChannelMode::JointStereo,
            2 => ChannelMode::DualChannel,
            3 => ChannelMode::Mono,
            _ => panic!("Invalid value for ChannelMode: {}", value),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Emphasis {
    None = 0,
    MS5015 = 1,
    Reserved = 2,
    CCITJ17 = 3,
}

impl From<u8> for Emphasis {
    fn from(value: u8) -> Self {
        match value {
            0 => Emphasis::None,
            1 => Emphasis::MS5015,
            2 => Emphasis::Reserved,
            3 => Emphasis::CCITJ17,
            _ => panic!("Invalid value for ChannelMode: {}", value),
        }
    }
}

pub struct BandIndex {
    long_win: &'static [u32],
    short_win: &'static [u32],
}

pub struct BandWidth {
    long_win: &'static [u32],
    short_win: &'static [u32],
}

impl Mp3 {
    pub fn new(buffer: &[u8]) -> Self {
        if buffer[0] == 0xFF && buffer[1] >= 0xE0 {
            let mut mp3 = Mp3 {
                valid: true,
                buffer: buffer.to_vec(),
                ..Default::default() // Use default values for other fields
            };
            mp3.init_header_params(buffer);
            mp3
        } else {
            Mp3::default()
        }
    }

    fn init_header_params(&mut self, buffer: &[u8]) {
        if buffer[0] == 0xFF && buffer[1] >= 0xE0 {
            self.buffer = buffer.to_vec();
            self.set_mpeg_version();
            self.set_layer(buffer[1]);
            self.set_crc();
            self.set_info();
            self.set_emphasis(buffer);
            self.set_sampling_rate();
            self.set_tables();
            self.set_channel_mode(buffer);
            self.set_mode_extension(buffer);
            self.set_padding();
            self.set_bit_rate(buffer);
            self.set_frame_size();
        } else {
            self.valid = false;
        }
    }

    pub fn init_frame_params(&mut self, buffer: &[u8]) {
        self.set_side_info(&buffer[self.crc as usize * 6..]);
        self.set_main_data(buffer);
        for gr in 0..2 {
            for ch in 0..self.channels {
                let ch_converted: usize = ch.try_into().unwrap();
                self.requantize(gr, ch_converted);
            }
            if self.channel_mode == ChannelMode::JointStereo && self.mode_extension[0] != 0 {
                self.ms_stereo(gr);
            }
            for ch in 0..self.channels {
                let ch_converted: usize = ch.try_into().unwrap();
                if self.block_type[gr][ch_converted] == 2 || self.mixed_block_flag[gr][ch_converted] {
                    self.reorder(gr, ch_converted);
                } else {
                    self.alias_reduction(gr, ch_converted);
                }
                self.imdct(gr, ch_converted);
                self.frequency_inversion(gr, ch_converted);
                self.synth_filterbank(gr, ch_converted);
            }
        }
        self.interleave();
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    pub fn set_mpeg_version(&mut self) {
        if (self.buffer[1] & 0x10) == 0x10 && (self.buffer[1] & 0x08) == 0x08 {
            self.mpeg_version = 1.0;
        } else if (self.buffer[1] & 0x10) == 0x10 && (self.buffer[1] & 0x08) != 0x08 {
            self.mpeg_version = 2.0;
        } else if (self.buffer[1] & 0x10) != 0x10 && (self.buffer[1] & 0x08) == 0x08 {
            self.mpeg_version = 0.0;
        } else if (self.buffer[1] & 0x10) != 0x10 && (self.buffer[1] & 0x08) != 0x08 {
            self.mpeg_version = 2.5;
        }
    }

    pub fn get_mpeg_version(&self) -> f32 {
        self.mpeg_version
    }

    pub fn set_layer(&mut self, byte: u8) {
        let byte = byte << 5;
        let byte = byte >> 6;
        self.layer = 4 - byte;
    }

    pub fn get_layer(&self) -> u32 {
        self.layer as u32
    }

    pub fn set_crc(&mut self) {
        self.crc = (self.buffer[1] & 0x01) != 0;
    }

    pub fn get_crc(&self) -> bool {
        self.crc
    }

    pub fn set_bit_rate(&mut self, buffer: &[u8]) {
        if self.mpeg_version == 1.0 {
            match self.layer {
                1 => {
                    self.bit_rate = buffer[2] as u32 * 32;
                }
                2 | 3 => {
                    let rates = [32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384];
                    let index = (buffer[2] >> 4) - 1;
                    self.bit_rate = rates[index as usize] * 1000;
                }
                _ => {
                    self.valid = false;
                }
            }
        } else {
            match self.layer {
                1 => {
                    let rates = [32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256];
                    let index = (buffer[2] >> 4) - 1;
                    self.bit_rate = rates[index as usize] * 1000;
                }
                2 | 3 => {
                    let rates = [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];
                    let index = (buffer[2] >> 4) - 1;
                    self.bit_rate = rates[index as usize] * 1000;
                }
                _ => {
                    self.valid = false;
                }
            }
        }
    }

    pub fn get_bit_rate(&self) -> u32 {
        self.bit_rate
    }

    pub fn set_sampling_rate(&mut self) {
        let rates = [
            [44100, 48000, 32000],
            [22050, 24000, 16000],
            [11025, 12000, 8000],
        ];

        for version in 1..=3 {
            if self.mpeg_version == version as f32 {
                if (self.buffer[2] & 0x08) != 0x08 && (self.buffer[2] & 0x04) != 0x04 {
                    self.sampling_rate = rates[version - 1][0];
                    break;
                } else if (self.buffer[2] & 0x08) != 0x08 && (self.buffer[2] & 0x04) == 0x04 {
                    self.sampling_rate = rates[version - 1][1];
                    break;
                } else if (self.buffer[2] & 0x08) == 0x08 && (self.buffer[2] & 0x04) != 0x04 {
                    self.sampling_rate = rates[version - 1][2];
                    break;
                }
            }
        }
    }

    pub fn get_sampling_rate(&self) -> u32 {
        self.sampling_rate
    }

    pub fn set_tables(&mut self) {
        match self.sampling_rate {
            32000 => {
                self.band_index.short_win = &tables::BAND_INDEX_TABLE.short_32;
                self.band_width.short_win = &tables::BAND_WIDTH_TABLE.short_32;
                self.band_index.long_win = &tables::BAND_INDEX_TABLE.long_32;
                self.band_width.long_win = &tables::BAND_WIDTH_TABLE.long_32;
            }
            44100 => {
                self.band_index.short_win = &tables::BAND_INDEX_TABLE.short_44;
                self.band_width.short_win = &tables::BAND_WIDTH_TABLE.short_44;
                self.band_index.long_win = &tables::BAND_INDEX_TABLE.long_44;
                self.band_width.long_win = &tables::BAND_WIDTH_TABLE.long_44;
            }
            48000 => {
                self.band_index.short_win = &tables::BAND_INDEX_TABLE.short_48;
                self.band_width.short_win = &tables::BAND_WIDTH_TABLE.short_48;
                self.band_index.long_win = &tables::BAND_INDEX_TABLE.long_48;
                self.band_width.long_win = &tables::BAND_WIDTH_TABLE.long_48;
            }
            _ => {} // Handle other cases if needed
        }
    }

    pub fn set_padding(&mut self) {
        self.padding = (self.buffer[2] & 0x02) != 0;
    }

    pub fn get_padding(&self) -> bool {
        self.padding
    }

    pub fn set_channel_mode(&mut self, buffer: &[u8]) {
        let value = buffer[3] >> 6;
        self.channel_mode = ChannelMode::from(value);
        self.channels = if self.channel_mode == ChannelMode::Mono { 1 } else { 2 };
    }

    pub fn get_channel_mode(&self) -> ChannelMode {
        self.channel_mode
    }

    pub fn set_mode_extension(&mut self, buffer: &[u8]) {
        if self.layer == 3 {
            self.mode_extension[0] = (buffer[3] & 0x20) as u32;
            self.mode_extension[1] = (buffer[3] & 0x10) as u32;
        }
    }

    pub fn get_mode_extension(&self) -> &[u32; 2] {
        &self.mode_extension
    }

    pub fn set_emphasis(&mut self, buffer: &[u8]) {
        let value = (buffer[3] << 6) >> 6;
        self.emphasis = Emphasis::from(value);
    }

    pub fn get_emphasis(&self) -> Emphasis {
        self.emphasis
    }

    pub fn set_info(&mut self) {
        self.info[0] = (self.buffer[2] & 0x01) != 0;
        self.info[1] = (self.buffer[3] & 0x08) != 0;
        self.info[2] = (self.buffer[3] & 0x04) != 0;
    }

    pub fn get_info(&self) -> &[bool] {
        &self.info
    }

    pub fn set_frame_size(&mut self) {
        let samples_per_frame = match self.layer {
            3 => {
                if self.mpeg_version == 1 as f32 {
                    1152
                } else {
                    576
                }
            }
            2 => 1152,
            1 => 384,
            _ => 0, // Handle other cases if needed
        };

        for i in (1..NUM_PREV_FRAMES).rev() {
            self.prev_frame_size[i] = self.prev_frame_size[i - 1];
        } 
        self.prev_frame_size[0] = self.frame_size as usize;
        self.frame_size = (samples_per_frame / 8 * self.bit_rate / self.sampling_rate) as i32;
        if self.padding == true {
            self.frame_size += 1;
        }
    }

    pub fn get_frame_size(&self) -> usize {
        self.frame_size as usize
    }

    pub fn get_header_size(&self) -> usize {
        4
    }

    pub fn set_side_info(&mut self, buffer: &[u8]) {
        let mut count = 0;

        self.main_data_begin = get_bits_inc(buffer, &mut count, 9) as i32;

        count += if self.channel_mode == ChannelMode::Mono { 5 } else { 3 };

        for ch in 0..self.channels as usize {
            for scfsi_band in 0..4 {
                self.scfsi[ch][scfsi_band] = get_bits_inc(buffer, &mut count, 1) != 0;
            }
        }

        for gr in 0..2 {
            for ch in 0..self.channels as usize {
                self.part2_3_length[gr][ch] = get_bits_inc(buffer, &mut count, 12) as i32;
                self.big_value[gr][ch] = get_bits_inc(buffer, &mut count, 9) as i32;
                self.global_gain[gr][ch] = get_bits_inc(buffer, &mut count, 8) as i32;
                self.scalefac_compress[gr][ch] = get_bits_inc(buffer, &mut count, 4) as i32;

                self.slen1[gr as usize][ch as usize] = tables::SLEN[self.scalefac_compress[gr as usize][ch as usize] as usize][0] as i32;
                self.slen2[gr as usize][ch as usize] = tables::SLEN[self.scalefac_compress[gr as usize][ch as usize] as usize][1] as i32;

                self.window_switching[gr][ch] = get_bits_inc(buffer, &mut count, 1) == 1;

                if self.window_switching[gr][ch] {
                    self.block_type[gr][ch] = get_bits_inc(buffer, &mut count, 2) as i32;
                    self.mixed_block_flag[gr][ch] = get_bits_inc(buffer, &mut count, 1) == 1;
                    self.switch_point_l[gr][ch] = if self.mixed_block_flag[gr][ch] { 8 } else { 0 };
                    self.switch_point_s[gr][ch] = if self.mixed_block_flag[gr][ch] { 3 } else { 0 };

                    self.region0_count[gr][ch] = if self.block_type[gr][ch] == 2 { 8 } else { 7 };
                    self.region1_count[gr][ch] = 20 - self.region0_count[gr][ch];

                    for region in 0..2 {
                        self.table_select[gr][ch][region] = get_bits_inc(buffer, &mut count, 5) as i32;
                    }
                    for window in 0..3 {
                        self.subblock_gain[gr][ch][window] = get_bits_inc(buffer, &mut count, 3) as i32;
                    }
                } else {
                    self.block_type[gr][ch] = 0;
                    self.mixed_block_flag[gr][ch] = false;

                    for region in 0..3 {
                        self.table_select[gr][ch][region] = get_bits_inc(buffer, &mut count, 5) as i32;
                    }

                    self.region0_count[gr][ch] = get_bits_inc(buffer, &mut count, 4) as i32;
                    self.region1_count[gr][ch] = get_bits_inc(buffer, &mut count, 3) as i32;
                }

                self.preflag[gr][ch] = get_bits_inc(buffer, &mut count, 1) as i32;
                self.scalefac_scale[gr][ch] = get_bits_inc(buffer, &mut count, 1) as i32;
                self.count1table_select[gr][ch] = get_bits_inc(buffer, &mut count, 1) as i32;
            }
        }
    }

    pub fn set_main_data(&mut self, buffer: &[u8]) {
        #[allow(unused_unsafe)]
        unsafe {
            let constant = if self.channel_mode == ChannelMode::Mono { 21_usize } else { 36_usize };
            let constant = if !self.crc { constant + 2 } else { constant };
    
            if self.main_data_begin == 0 {
                self.main_data.resize(self.frame_size as usize - constant, 0);
                self.main_data.copy_from_slice(&buffer[constant..self.frame_size as usize]);
            } else {
                let mut bound: usize = 0;
                for frame in 0..NUM_PREV_FRAMES {
                    bound += self.prev_frame_size[frame] - constant;
                    if self.main_data_begin < bound as i32 {
                        let mut ptr_offset: usize = self.main_data_begin as usize + frame as usize * constant;
                        let mut buffer_offset: usize = 0;
    
                        let mut part = vec![0_usize; NUM_PREV_FRAMES];
                        part[frame] = self.main_data_begin as usize;
                        for i in 0..frame {
                            part[i] = self.prev_frame_size[i] - constant;
                            part[frame] -= part[i];
                        }
    
                        self.main_data.resize(self.frame_size as usize - constant + self.main_data_begin as usize, 0);
                        self.main_data[..part[frame]].copy_from_slice(&buffer[buffer_offset..buffer_offset + part[frame]]);
                        ptr_offset -= part[frame] + constant;
                        buffer_offset += part[frame];
                        for i in (0..frame).rev() {
                            self.main_data[buffer_offset..buffer_offset + part[i]].copy_from_slice(&buffer[ptr_offset..ptr_offset + part[i]]);
                            ptr_offset -= part[i] + constant;
                            buffer_offset += part[i];
                        }
                        self.main_data[self.main_data_begin as usize..].copy_from_slice(&buffer[constant..self.frame_size as usize - constant]);
                        break;
                    }
                }
            }
    
            let mut bit: usize = 0;
            for gr in 0..2 {
                for ch in 0..self.channels {
                    let max_bit = bit + self.part2_3_length[gr][ch as usize] as usize;
                    self.unpack_scalefac(gr, ch as usize, &mut bit);
                    self.unpack_samples( gr, ch as usize, bit, max_bit);
                    bit = max_bit;
                }
            }
        }
    }
    
    

    pub fn unpack_scalefac(&mut self, gr: usize, ch: usize, bit: &mut usize) {
        let mut sfb = 0;
        let mut window = 0;
        let scalefactor_length = [
            tables::SLEN[self.scalefac_compress[gr as usize][ch as usize] as usize][0] as i32,
            tables::SLEN[self.scalefac_compress[gr as usize][ch as usize] as usize][1] as i32
        ];

        if self.block_type[gr][ch] == 2 && self.window_switching[gr][ch] {
            if self.mixed_block_flag[gr][ch] == true {
                for _ in 0..8 {
                    self.scalefac_l[gr][ch][sfb] = get_bits_inc(&mut self.main_data, bit, scalefactor_length[0].try_into().unwrap()) as i32;
                    sfb += 1;
                }

                for _ in 3..6 {
                    for _ in 0..3 {
                        self.scalefac_s[gr][ch][window][sfb] = get_bits_inc(&mut self.main_data, bit, scalefactor_length[0].try_into().unwrap()) as i32;
                        window += 1;
                    }
                }
            } else {
                for _ in 0..6 {
                    for _ in 0..3 {
                        self.scalefac_s[gr][ch][window][sfb] = get_bits_inc(&mut self.main_data, bit, scalefactor_length[0].try_into().unwrap()) as i32;
                        window += 1;
                    }
                }

                for _ in 6..12 {
                    for _ in 0..3 {
                        self.scalefac_s[gr][ch][window][sfb] = get_bits_inc(&mut self.main_data, bit, scalefactor_length[1].try_into().unwrap()) as i32;
                        window += 1;
                    }
                }

                for window in 0..3 {
                    self.scalefac_s[gr][ch][window][12] = 0;
                }
            }
        } else {
            if gr == 0 {
                for _ in 0..11 {
                    self.scalefac_l[gr][ch][sfb] = get_bits_inc(&mut self.main_data, bit, scalefactor_length[0].try_into().unwrap()) as i32;
                    sfb += 1;
                }
                for _ in 11..21 {
                    self.scalefac_l[gr][ch][sfb] = get_bits_inc(&mut self.main_data, bit, scalefactor_length[1].try_into().unwrap()) as i32;
                    sfb += 1;
                }
            } else {
                let sb = [6, 11, 16, 21];
                for i in 0..2 {
                    for _ in sfb..sb[i] {
                        if self.scfsi[ch][i] {
                            self.scalefac_l[gr][ch][sfb] = self.scalefac_l[0][ch][sfb];
                        } else {
                            self.scalefac_l[gr][ch][sfb] = get_bits_inc(&mut self.main_data, bit, scalefactor_length[0].try_into().unwrap()) as i32;
                        }
                        sfb += 1;
                    }
                }
                for i in 2..4 {
                    for _ in sfb..sb[i] {
                        if self.scfsi[ch][i] {
                            self.scalefac_l[gr][ch][sfb] = self.scalefac_l[0][ch][sfb];
                        } else {
                            self.scalefac_l[gr][ch][sfb] = get_bits_inc(&mut self.main_data, bit, scalefactor_length[1].try_into().unwrap()) as i32;
                        }
                        sfb += 1;
                    }
                }
            }
            self.scalefac_l[gr][ch][21] = 0;
        }
    }

    pub fn unpack_samples(&mut self, gr: usize, ch: usize, mut bit: usize, max_bit: usize) {
        let mut sample = 0;
        let mut table_num: usize;
        let mut table: &[u32];
    
        for i in 0..576 {
            self.samples[gr][ch][i] = 0.0;
        }
    
        let (region0, region1) = if self.window_switching[gr][ch] && self.block_type[gr][ch] == 2 {
            (36, 576)
        } else {
            (
                self.band_index.long_win[(self.region0_count[gr][ch] + 1) as usize],
                self.band_index.long_win[(self.region0_count[gr][ch] + 1 + self.region1_count[gr][ch] + 1) as usize],
            )
        };
    
        while sample < self.big_value[gr][ch] as usize * 2 {
            if sample < region0 as usize {
                table_num = self.table_select[gr][ch][0] as usize;
                table = tables::BIG_VALUE_TABLE[table_num];
            } else if sample < region1 as usize {
                table_num = self.table_select[gr][ch][1] as usize;
                table = tables::BIG_VALUE_TABLE[table_num];
            } else {
                table_num = self.table_select[gr][ch][2] as usize;
                table = tables::BIG_VALUE_TABLE[table_num];
            }
    
            if table_num == 0 {
                self.samples[gr][ch][sample] = 0.0;
                sample += 2;
                continue;
            }
    
            let mut repeat = true;
            let bit_sample = get_bits(&self.main_data, bit, bit + 32);
    
            for row in 0..tables::BIG_VALUE_MAX[table_num] {
                for col in 0..tables::BIG_VALUE_MAX[table_num] {
                    let i = 2 * tables::BIG_VALUE_MAX[table_num] * row + 2 * col;
                    let value = table[i as usize];
                    let size: u32 = table[(i + 1) as usize];
    
                    if (value >> (32 - size)) == (bit_sample >> (32 - size)) {
                        bit += size as usize;
    
                        let mut values = [row as i32, col as i32];
                        for i in 0..2 {
                            let mut linbit: i32 = 0;
                            let table_num_usize: usize = table_num.try_into().unwrap();
                            if tables::BIG_VALUE_LINBIT[table_num_usize] != 0 && values[i] == (tables::BIG_VALUE_MAX[table_num_usize] - 1) as i32 {
                                linbit = get_bits_inc(&self.main_data, &mut bit, tables::BIG_VALUE_LINBIT[table_num_usize] as usize) as i32;
                            }
    
                            let sign = if values[i] > 0 {
                                if get_bits_inc(&self.main_data, &mut bit, 1) == 1 {
                                    -1
                                } else {
                                    1
                                }
                            } else {
                                1
                            };
    
                            self.samples[gr][ch][sample + i] = (sign * (values[i] + linbit)) as f32;
                        }
    
                        repeat = false;
                        break;
                    }
                }
                if !repeat {
                    break;
                }
            }
    
            sample += 2;
        }
    
        while bit < max_bit && sample + 4 < 576 {
            let mut values = [0; 4];
    
            if self.count1table_select[gr][ch] == 1 {
                let bit_sample = get_bits_inc(&self.main_data, &mut bit, 4);
                values[0] = if (bit_sample & 0x08) > 0 { 0 } else { 1 };
                values[1] = if (bit_sample & 0x04) > 0 { 0 } else { 1 };
                values[2] = if (bit_sample & 0x02) > 0 { 0 } else { 1 };
                values[3] = if (bit_sample & 0x01) > 0 { 0 } else { 1 };
            } else {
                let bit_sample = get_bits(&self.main_data, bit, bit + 32);
                for entry in 0..16 {
                    let value = tables::QUAD_TABLE_1.hcod[entry];
                    let size = tables::QUAD_TABLE_1.hlen[entry];
    
                    if (value >> (32 - size)) == (bit_sample >> (32 - size)) {
                        bit += size as usize;
                        for i in 0..4 {
                            values[i] = tables::QUAD_TABLE_1.value[entry][i];
                        }
                        break;
                    }
                }
            }

            let mut values: [i8; 4] = [0; 4];
    
            for i in 0..4 {
                if values[i] > 0 && get_bits_inc(&self.main_data, &mut bit, 1) == 1 {
                    values[i] = -values[i];
                }
            }
    
            for i in 0..4 {
                self.samples[gr][ch][sample + i] = values[i] as f32;
            }
    
            sample += 4;
        }
    
        while sample < 576 {
            self.samples[gr][ch][sample] = 0.0;
            sample += 1;
        }
    }
    

    pub fn requantize(&mut self, gr: usize, ch: usize) {
        let mut exp1: f32;
        let mut exp2: f32;
        let mut window: usize = 0;
        let mut sfb: usize = 0;
        let scalefac_mult: f32 = if self.scalefac_scale[gr][ch] == 0 { 0.5 } else { 1.0 };
    
        let mut i = 0;
        for sample in 0..576 {
            if self.block_type[gr][ch] == 2 || (self.mixed_block_flag[gr][ch] && sfb >= 8) {
                if i == self.band_width.short_win[sfb] as usize {
                    i = 0;
                    if window == 2 {
                        window = 0;
                        sfb += 1;
                    } else {
                        window += 1;
                    }
                }
    
                exp1 = self.global_gain[gr][ch] as f32 - 210.0 - 8.0 * self.subblock_gain[gr][ch][window] as f32;
                exp2 = scalefac_mult * self.scalefac_s[gr][ch][window][sfb] as f32;
            } else {
                if sample == self.band_index.long_win[sfb + 1] as usize {
                    sfb += 1;
                }
    
                exp1 = self.global_gain[gr][ch] as f32 - 210.0;
                exp2 = scalefac_mult * (self.scalefac_l[gr][ch][sfb] as f32 + self.preflag[gr][ch] as f32 * tables::PRETAB[sfb] as f32);
            }
    
            let sign: f32 = if self.samples[gr][ch][sample] < 0.0 { -1.0 } else { 1.0 };
            let a: f32 = self.samples[gr][ch][sample].abs().powf(4.0 / 3.0);
            let b: f32 = 2.0_f32.powf(exp1 / 4.0);
            let c: f32 = 2.0_f32.powf(-exp2);
            self.samples[gr][ch][sample] = sign * a * b * c;
    
            i += 1;
        }
    }    

    pub fn reorder(&mut self, gr: usize, ch: usize) {
        let mut total = 0;
        let mut start = 0;
        let mut block = 0;
        let mut samples = [0.0; 576];

        for sb in 0..12 {
            let sb_width = self.band_width.short_win[sb] as usize;

            for ss in 0..sb_width {
                samples[start + block + 0] = self.samples[gr][ch][total + ss + sb_width * 0];
                samples[start + block + 6] = self.samples[gr][ch][total + ss + sb_width * 1];
                samples[start + block + 12] = self.samples[gr][ch][total + ss + sb_width * 2];

                if block != 0 && block % 5 == 0 {
                    start += 18;
                    block = 0;
                } else {
                    block += 1;
                }
            }

            total += sb_width * 3;
        }

        self.samples[gr][ch].copy_from_slice(&samples);
    }

    pub fn ms_stereo(&mut self, gr: usize) {
        for sample in 0..576 {
            let middle = self.samples[gr][0][sample];
            let side = self.samples[gr][1][sample];
            self.samples[gr][0][sample] = (middle + side) / SQRT2 as f32;
            self.samples[gr][1][sample] = (middle - side) / SQRT2 as f32;
        }
    }

    pub fn alias_reduction(&mut self, gr: usize, ch: usize) {
        const CS: [f32; 8] = [
            0.8574929257, 0.8817419973, 0.9496286491, 0.9833145925,
            0.9955178161, 0.9991605582, 0.9998991952, 0.9999931551,
        ];

        const CA: [f32; 8] = [
            -0.5144957554, -0.4717319686, -0.3133774542, -0.1819131996,
            -0.0945741925, -0.0409655829, -0.0141985686, -0.0036999747,
        ];

        let sb_max = if self.mixed_block_flag[gr][ch] { 2 } else { 32 };

        for sb in 1..sb_max {
            for sample in 0..8 {
                let offset1 = 18 * sb - sample - 1;
                let offset2 = 18 * sb + sample;
                let s1 = self.samples[gr][ch][offset1];
                let s2 = self.samples[gr][ch][offset2];
                self.samples[gr][ch][offset1] = s1 * CS[sample] - s2 * CA[sample];
                self.samples[gr][ch][offset2] = s2 * CS[sample] + s1 * CA[sample];
            }
        }
    }

    pub fn imdct(&mut self, gr: usize, ch: usize) {
        static mut INIT: bool = true;
        static mut SINE_BLOCK: [[f32; 36]; 4] = [[0.0; 36]; 4];
        let mut sample_block = [0.0; 36];

        unsafe {
            if INIT {
                let mut _i = 0;
                for i in 0..36 {
                    SINE_BLOCK[0][i] = (std::f32::consts::PI / 36.0 * (i as f32 + 0.5)).sin();
                }
                for i in 0..18 {
                    SINE_BLOCK[1][i] = (std::f32::consts::PI / 36.0 * (i as f32 + 0.5)).sin();
                }
                for i in 18..24 {
                    SINE_BLOCK[1][i] = 1.0;
                }
                for i in 24..30 {
                    SINE_BLOCK[1][i] = (std::f32::consts::PI / 12.0 * (i as f32 - 18.0 + 0.5)).sin();
                }
                for i in 30..36 {
                    SINE_BLOCK[1][i] = 0.0;
                }
                for i in 0..12 {
                    SINE_BLOCK[2][i] = (std::f32::consts::PI / 12.0 * (i as f32 + 0.5)).sin();
                }
                for i in 0..6 {
                    SINE_BLOCK[3][i] = 0.0;
                }
                for i in 6..12 {
                    SINE_BLOCK[3][i] = (std::f32::consts::PI / 12.0 * (i as f32 - 6.0 + 0.5)).sin();
                }
                for i in 12..18 {
                    SINE_BLOCK[3][i] = 1.0;
                }
                for i in 18..36 {
                    SINE_BLOCK[3][i] = (std::f32::consts::PI / 36.0 * (i as f32 + 0.5)).sin();
                }
                INIT = false;
            }
        }

        let n = if self.block_type[gr][ch] == 2 { 12 } else { 36 };
        let half_n = n / 2;
        let mut sample = 0;

        for block in 0..32 {
            for win in 0..if self.block_type[gr][ch] == 2 { 3 } else { 1 } {
                for i in 0..n {
                    let mut xi = 0.0;
                    for k in 0..half_n {
                        let s = self.samples[gr][ch][18 * block + half_n * win + k];
                        xi += s * ((std::f32::consts::PI / (2 * n) as f32 * (2 as f32 * i as f32 + 1.0 + half_n as f32) * (2 as f32 * k as f32 + 1.0)).cos());
                    }
                    sample_block[win * n + i] = xi * unsafe { SINE_BLOCK[self.block_type[gr][ch] as usize][i as usize] };
                }
            }

            if self.block_type[gr][ch] == 2 {
                let mut temp_block = [0.0; 36];
                temp_block.copy_from_slice(&sample_block);

                let mut _i = 0;
                for i in 0..6 {
                    sample_block[i] = 0.0;
                }
                for i in 6..12 {
                    sample_block[i] = temp_block[0 + i - 6];
                }
                for i in 12..18 {
                    sample_block[i] = temp_block[0 + i - 6] + temp_block[12 + i - 12];
                }
                for i in 18..24 {
                    sample_block[i] = temp_block[12 + i - 12] + temp_block[24 + i - 18];
                }
                for i in 24..30 {
                    sample_block[i] = temp_block[24 + i - 18];
                }
                for i in 30..36 {
                    sample_block[i] = 0.0;
                }
            }

            for i in 0..18 {
                self.samples[gr][ch][sample + i] = sample_block[i] + self.prev_samples[ch][block][i];
                self.prev_samples[ch][block][i] = sample_block[18 + i];
            }
            sample += 18;
        }
    }

    pub fn frequency_inversion(&mut self, gr: usize, ch: usize) {
        for sb in (1..18).step_by(2) {
            for i in (1..32).step_by(2) {
                self.samples[gr][ch][i * 18 + sb] *= -1.0;
            }
        }
    }

    pub fn synth_filterbank(&mut self, gr: usize, ch: usize) {
        static mut N: [[f32; 32]; 64] = [[0.0; 32]; 64];
        static mut INIT: bool = true;

        unsafe {
            if INIT {
                INIT = false;
                for i in 0..64 {
                    for j in 0..32 {
                        N[i][j] = ((16.0 + i as f32) * (2.0 * j as f32 + 1.0) * (PI as f32 / 64.0)).cos();
                    }
                }
            }
        }

        let mut s = [0.0; 32];
        let mut u = [0.0; 512];
        let mut w = [0.0; 512];
        let mut pcm = [0.0; 576];

        for sb in 0..18 {
            for i in 0..32 {
                s[i] = self.samples[gr][ch][i * 18 + sb];
            }

            for i in (63..1024).rev() {
                self.fifo[ch][i] = self.fifo[ch][i - 64];
            }

            for i in 0..64 {
                self.fifo[ch][i] = 0.0;
                for j in 0..32 {
                    self.fifo[ch][i] += s[j] * unsafe { N[i][j] };
                }
            }

            for i in 0..8 {
                for j in 0..32 {
                    u[i * 64 + j] = self.fifo[ch][i * 128 + j];
                    u[i * 64 + j + 32] = self.fifo[ch][i * 128 + j + 96];
                }
            }

            for i in 0..512 {
                w[i] = u[i] * tables::SYNTH_WINDOW[i] as f32;
            }

            for i in 0..32 {
                let mut sum = 0.0;
                for j in 0..16 {
                    sum += w[j * 32 + i];
                }
                pcm[32 * sb + i] = sum;
            }
        }

        self.samples[gr][ch].copy_from_slice(&pcm);
    }

    pub fn interleave(&mut self) {
        let mut i = 0;
        for gr in 0..2{
            for sample in 0..576{
                for ch in 0..self.channels {
                    self.pcm[i] = self.samples[gr][ch as usize][sample];
                    i += 1;
                }
            }
        }
    }

    pub fn get_samples(&self) -> &[f32] {
        &self.pcm
    }
}