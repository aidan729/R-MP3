# R-MP3

## Summary
R-MP3 is an attempt at a native rust implemetnation of decoding mp3 files. MP3 Files are digital audio, stored as pulse code modulation (PCM), often consumes significant memory, especially when streamed over the internet, TV, or radio. To mitigate this, compression techniques are employed to reduce file size without compromising audio quality.

### Compression Methods
Compression methods can be categorized as either lossless or lossy. Lossless compression retains all original data upon reconstruction, while lossy compression discards non-essential information imperceptible to human ears, thus achieving higher compression ratios.

### MP3 and AAC Encoding
MP3 and AAC encoders combine lossless and lossy principles to optimize storage and transmission of audio files. They selectively remove data beyond human auditory capabilities, allowing for efficient storage and streaming.


<table>
  <tr>
    <td>ID3</td>
    <td colspan=5>MP3</td></tr>
  <tr>
    <td>Meta Data</td>
    <td>Header</td>
    <td>Side Information</td>
    <td>Main Data</td>
    <td>Header</td>
    <td>...</td>
  </tr>
</table>

### Understanding the Decoder's Role

When it comes to compressing audio files, there are two primary methods: lossless and lossy. Lossless compression ensures that the reconstructed file is identical to the original, while lossy compression selectively discards non-critical information, optimizing file size.

In the realm of audio compression, MP3 and AAC encoders effectively blend both lossless and lossy techniques. By removing redundant data outside the human auditory range and exploiting frequency masking phenomena, these codecs enable efficient storage and high-speed streaming of music files.

The decoder, pivotal in the playback process, serves the critical function of reverting the compressed data to its original format. Its primary responsibilities include:

- Extracting PCM data from the compressed bitstream.
- Facilitating the transfer of decoded data to the operating system or media player for playback.

In essence, the decoder acts as the gateway to restoring compressed audio files to their full fidelity, ensuring an optimal listening experience for users.

### Understanding ID3 Metadata

ID3, specifically version 2, functions as a block of bytes dedicated to storing metadata associated with audio files. While this metadata serves various purposes, much of it holds little relevance to the decoder's core function, with one notable exception: an offset indicating the tag's end position.

An inconvenience arises when multiple ID3 tags follow the initial one, complicating the parsing process. Despite this challenge, comprehensive documentation is accessible at ID3.org, offering detailed insights into the structure and usage of ID3 metadata.

In summary, while ID3 metadata enriches audio files with supplementary information, the decoder primarily focuses on extracting essential data for playback, relegating much of the metadata to secondary importance.

| Type       | Offset (bytes) | Description                                         |
| :--------- | :------------- | :-------------------------------------------------- |
| Identifier | 0 - 2          | Indicates the presence of ID3.                      |
| Version    | 3 - 4          | Version and revision of the tag.                    |
| Flags      | 5              | Four single-bit booleans.                           |
| Size       | 6 - 9          | Size of the ID3 tag excluding this ten byte header. |

### Decoding the MP3 Header

The MP3 header encapsulates crucial parameters essential for decoding audio data, including the sampling rate, bit rate, number of channels, and version information. Utilizing this metadata, it becomes feasible to determine the frame size of the MP3 data, consequently providing insights into the location of subsequent headers.

The formula for calculating the size of an MP3 frame involves dividing the number of samples per frame by 8, then multiplying by the ratio of bit rate to sampling rate, and finally incorporating any necessary padding.

$$ \frac{\text{samples per frame}}{8} \times \frac{\text{bit rate}}{\text{sampling rate}} + \text{padding} $$

By leveraging the size of the initial frame, the decoder can effectively pinpoint the position of subsequent headers within the audio file. Moreover, the presence of INFO or XING tags within the first frame often serves as valuable navigational aids, facilitating smoother traversal through the audio data.  

### Understanding Side Information

Following each header, the MP3 file structure includes side information crucial for locating the main audio data. Unlike a straightforward sequential arrangement, the main data does not immediately follow the side information due to the variable sizes of Huffman encoded samples.

In addition to aiding data location, side information incorporates supplementary values essential for the requantization process. These values play a pivotal role in reconstructing the encoded samples into their original, real-number representation.

By encompassing both navigational cues and essential parameters for reconstruction, side information serves as a vital component in the decoding process, ensuring accurate restoration of the audio signal.

## Exploring the Main Data Structure

The main data within an MP3 file is organized into two granules, each accommodating up to two channels, as dictated by the channel mode specified in the header. Each channel within a granule commences with scale factors followed by Huffman-coded bits.

Upon decoding the Huffman bits, 576 frequency lines per channel are extracted, contributing to the reconstruction of the audio signal. Additionally, at the conclusion of the main data, user-defined ancillary data may be present, serving various auxiliary purposes.

### Requantization and Scaling
Scale factors play a pivotal role in the requantization process, where samples are adjusted to real-number representations. Different scaling factors are applied to distinct groups or subbands of samples, ensuring accurate reconstruction of the original audio signal.

### Huffman Coding
After the scale factors, the main data incorporates Huffman-coded bits. This region is subdivided into three distinct regions:

1. Big Value Regions: These regions encode significant amplitude values.
2. Quadruples Region: Reserved for encoding higher-pitched frequencies, utilizing a more tightly compressed format and a reduced set of Huffman tables.
3. Zero Region: Contains omitted samples, reducing redundancy in the encoded data.

Depending on the side information, each region employs specific Huffman tables optimized for efficient compression and decoding.

### Implementation Considerations
To streamline decoding, it may be advantageous to develop a dedicated program capable of embedding Huffman tables into Rust code. This process involves parsing the Huffman tables to ensure proper alignment of Huffman bits within integers and the accurate storage of each Huffman value's length in bits.

By optimizing the decoding process and integrating Huffman tables efficiently, the decoder can effectively reconstruct the original audio signal with minimal loss in fidelity.

```rust
let bit_sample: u32 = get_bits(bitstream, bit, bit + 32);
for ... {
    let value: i32 = table.hcod[entry];
    let size: i32 = table.hlen[entry];

    if value == (bit_sample >> (32 - size)) {
        // ...
    }
}
```

Each region within MP3 decoding is subdivided into two distinct block types:

1. Long Blocks: These blocks offer higher frequency resolution, capturing finer audio details. Although long blocks provide accurate audio representation, they result in larger file sizes.

2. Short Blocks: Short blocks prioritize lower frequency resolution and are one-third the size of long blocks. However, they are not aligned in order, necessitating reordering during decoding. This realignment mitigates artifacts like pre-echo and optimizes compression efficiency.

Short blocks are strategically employed when frequencies are closely spaced, reducing the need for extensive Huffman coding. This approach strikes a balance between frequency resolution and compression efficiency.

In essence, the integration of both long and short blocks in MP3 decoding ensures an optimal balance between audio fidelity and file size.

### Inverse Quantization in MP3 Decoding

Quantization, a process of converting continuous or infinite values into discrete ones, is fundamental to audio compression. In the MP3 decoding context, Huffman samples represent a discrete dataset. The inverse quantization formula reverses this process, reconstructing real numbers (theoretically continuous) from the original Huffman samples.

The inverse quantization formula is expressed as:

$$ S_i = \text{sign}(s_i) \times |s_i|^{\frac{4}{3}} \times 2^{\frac{a}{4}} \times 2^{-b} $$

For long blocks, the exponents are calculated as follows:

```rust
let a_long = global_gain[gr][ch] - 210;
let b_long = (scalefac_scale[gr][ch] == 0) as f32 * 0.5
    + (scalefac_scale[gr][ch] != 0) as f32 * 1.0
    * scalefactor[gr][ch][sb]
    + preflag[gr][ch] * pretab[sb];
```
and for short blocks:

```rust
let a_short = global_gain[gr][ch] - 210 - 8 * subblock_gain[gr][ch][window];
let b_short = (scalefac_scale[gr][ch] == 0) as f32 * 0.5
    + (scalefac_scale[gr][ch] != 0) as f32 * 1.0
    * scalefactor[gr][ch][sb][window];
```
These calculations adjust the scaling factors and other parameters from the side information, ensuring accurate reconstruction of the original audio samples during decoding.

### Reordering

During the decoding process, only short blocks require reordering. When the first few subbands are long blocks, they are excluded from this process. During reordering, samples in each subband are mapped to blocks of 18 samples.

### Inverse Modified Discrete Cosine Transform (IMDCT)

An additional layer of lossy compression in MP3 decoding involves the Modified Discrete Cosine Transform (MDCT). The MDCT maps closely related audio samples onto a cosine function. For long blocks, the MDCT reduces 32 samples to 18 samples, while for short blocks, it reduces 12 samples to 6.

The formula for the MDCT is expressed as:

$$ x_i = \sum_{k=0}^{\frac{n}{2}-1}{X_k \cos{\left(\frac{\pi}{2n}\left[ 2i + 1 + \frac{n}{2} \right]\left[ 2k + 1 \right]\right)}} $$

Here, variable nn represents the block size, which is 12 for short blocks and 36 for long blocks. The MDCT produces x0x0​ through xn−1xn−1​.

Once the cosine transform is complete, the resulting samples undergo windowing and overlapping to mitigate artifacts and ensure smooth transitions between adjacent blocks.

### Fast Fourier Transform (FFT) in MP3 Decoding

The input to the MP3 encoder comprises pulse code modulation (PCM) samples in the time domain. To facilitate compression, the encoder transforms these time domain samples into frequency domain samples using the Fast Fourier Transform (FFT). This conversion enables efficient representation of audio data in terms of frequency components.

During decoding, the process is reversed: the Inverse Modified Discrete Cosine Transform (IMDCT) converts frequency domain samples back into time domain samples. This reconstruction allows for the faithful restoration of the original audio signal.

### Synthesis Filter Bank in MP3 Encoding

In MP3 encoding, the synthesis filter bank plays a crucial role in transforming a pulse code modulation (PCM) stream into frequency bands that approximate critical bands. Critical bands are regions in the frequency spectrum where frequencies sound similar and affect the same area of the basilar membrane in the cochlea.

To mitigate artifacts resulting from quantization, the encoder divides the frequency spectrum into several bands structured similarly to critical bands. This structuring ensures that quantization artifacts are masked within the same frequency bands, enhancing audio quality.

As frequency increases, critical bands become larger, reflecting the decreased ability to discern between individual frequencies at higher pitches. The encoder's filter divides the frequency spectrum into equal-sized bands, reflecting this phenomenon.

It's important to note that while the encoder handles the division of the frequency spectrum and encoding process, the decoder is responsible for reconstructing the original audio signal from these encoded frequency bands.
