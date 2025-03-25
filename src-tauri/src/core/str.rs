/// split string into a list of lines
/// each line ends with new line characters, i.e. `\r`, `\n` and `\r\n`
///
/// note `String.lines()` treats new line characters as the same
/// which causes out of indirectly exposes index shift problem in original string
pub fn split_lines_with_endings(input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line_start = 0;

    let mut i = 0;
    while i < input.len() {
        let current_char = input[i..].chars().next().unwrap();
        let next_char = if i + current_char.len_utf8() < input.len() {
            input[i + current_char.len_utf8()..].chars().next().unwrap()
        } else {
            // NULL char
            '\0'
        };

        if current_char == '\n' || current_char == '\r' {
            // case \r or \n
            lines.push(input[line_start..=i].to_string());
            line_start = i + current_char.len_utf8();

            // case \r\n
            if current_char == '\r' && next_char == '\n' {
                line_start += next_char.len_utf8();
                // proceed on \n
                i += next_char.len_utf8();
            }
        }

        i += current_char.len_utf8();
    }

    // last line
    if line_start < input.len() {
        lines.push(input[line_start..].to_string());
    }

    lines
}

/// get byte start and end of multi byte string
pub fn multibyte_str_byte_indices(
    text: &str,
    index_start: usize,
    index_end: usize,
) -> Option<(usize, usize)> {
    let char_indices = text.char_indices().collect::<Vec<_>>();

    // Ensure the start and end indices are within bounds
    if index_start >= char_indices.len()
        || index_end > char_indices.len()
        || index_start >= index_end
    {
        return None;
    }

    // Get the byte indices corresponding to the char indices
    let byte_start = char_indices[index_start].0;
    let byte_end = char_indices[index_end - 1].0 + char_indices[index_end - 1].1.len_utf8();

    Some((byte_start, byte_end))
}

/// convert bytes array to hex chars string
pub fn bytes_to_hex_dump(bytes: &[u8]) -> String {
    const BYTES_PER_ROW: usize = 8;

    let mut output = String::new();

    for (i, chunk) in bytes.chunks(BYTES_PER_ROW).enumerate() {
        // address offset
        output.push_str(&format!("{:08x}  ", i * BYTES_PER_ROW));

        // hex bytes
        for (j, byte) in chunk.iter().enumerate() {
            if j > 0 && j % 4 == 0 {
                output.push(' '); // Extra spacing every 4 bytes
            }
            output.push_str(&format!("{:02x} ", byte));
        }

        // ascii section
        let hex_width = (BYTES_PER_ROW * 3) + (BYTES_PER_ROW / 4); // Account for spaces
        output.push_str(&" ".repeat(hex_width - chunk.len() * 3 - (chunk.len() / 4)));

        // ascii representation
        output.push('|');
        for &byte in chunk {
            output.push(if byte.is_ascii_graphic() || byte.is_ascii_whitespace() {
                byte as char
            } else {
                '.'
            });
        }
        output.push('|');
        output.push('\n');
    }

    output
}
