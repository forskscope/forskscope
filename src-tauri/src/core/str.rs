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
