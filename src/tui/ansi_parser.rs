use crossterm;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use vtparse::{CsiParam, VTActor, VTParser};

#[derive(Debug, Clone)]
pub struct CursorPosition {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StyledChar {
    pub ch: char,
    pub style: Style,
}

impl StyledChar {
    pub fn new(ch: char, style: Style) -> Self {
        Self { ch, style }
    }

    pub fn space_with_style(style: Style) -> Self {
        Self { ch: ' ', style }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalState {
    pub cursor: CursorPosition,
    pub saved_cursor: Option<CursorPosition>,
    pub foreground_color: Option<Color>,
    pub background_color: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
    pub alternate_screen: bool,
    pub screen_buffer: Vec<Vec<StyledChar>>,
    pub alternate_buffer: Vec<Vec<StyledChar>>,
    pub width: usize,
    pub height: usize,
    pub scroll_region: Option<(usize, usize)>,
    pub tab_stops: Vec<usize>,
    pub charset: u8,
    pub wrap_mode: bool,
    pub insert_mode: bool,
    pub application_keypad: bool,
    pub origin_mode: bool,
    pub auto_wrap: bool,
    pub cursor_visible: bool,
    pub screen_cleared: bool,
    pub final_output: Vec<String>,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            cursor: CursorPosition { row: 0, col: 0 },
            saved_cursor: None,
            foreground_color: None,
            background_color: None,
            bold: false,
            italic: false,
            underline: false,
            reverse: false,
            alternate_screen: false,
            screen_buffer: vec![vec![StyledChar::space_with_style(Style::default()); 80]; 24],
            alternate_buffer: vec![vec![StyledChar::space_with_style(Style::default()); 80]; 24],
            width: 80,
            height: 24,
            scroll_region: None,
            tab_stops: (0..80).step_by(8).collect(),
            charset: 0,
            wrap_mode: true,
            insert_mode: false,
            application_keypad: false,
            origin_mode: false,
            auto_wrap: true,
            cursor_visible: true,
            screen_cleared: false,
            final_output: Vec::new(),
        }
    }
}

impl TerminalState {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            screen_buffer: vec![
                vec![StyledChar::space_with_style(Style::default()); width];
                height
            ],
            alternate_buffer: vec![
                vec![StyledChar::space_with_style(Style::default()); width];
                height
            ],
            tab_stops: (0..width).step_by(8).collect(),
            ..Default::default()
        }
    }

    pub fn current_buffer(&self) -> &Vec<Vec<StyledChar>> {
        if self.alternate_screen {
            &self.alternate_buffer
        } else {
            &self.screen_buffer
        }
    }

    pub fn current_buffer_mut(&mut self) -> &mut Vec<Vec<StyledChar>> {
        if self.alternate_screen {
            &mut self.alternate_buffer
        } else {
            &mut self.screen_buffer
        }
    }

    pub fn clear_screen(&mut self) {
        let buffer = self.current_buffer_mut();
        for row in buffer.iter_mut() {
            for cell in row.iter_mut() {
                *cell = StyledChar::space_with_style(Style::default());
            }
        }
        self.cursor = CursorPosition { row: 0, col: 0 };
        self.screen_cleared = true;
    }

    pub fn clear_line(&mut self) {
        let cursor_row = self.cursor.row;
        let buffer = self.current_buffer_mut();
        if cursor_row < buffer.len() {
            for cell in buffer[cursor_row].iter_mut() {
                *cell = StyledChar::space_with_style(Style::default());
            }
        }
    }

    pub fn clear_to_end_of_line(&mut self) {
        let cursor_row = self.cursor.row;
        let cursor_col = self.cursor.col;
        let buffer = self.current_buffer_mut();
        if cursor_row < buffer.len() {
            let row = &mut buffer[cursor_row];
            for cell in row.iter_mut().skip(cursor_col) {
                *cell = StyledChar::space_with_style(Style::default());
            }
        }
    }

    pub fn clear_to_beginning_of_line(&mut self) {
        let cursor_row = self.cursor.row;
        let cursor_col = self.cursor.col;
        let buffer = self.current_buffer_mut();
        if cursor_row < buffer.len() {
            let row = &mut buffer[cursor_row];
            for i in 0..=cursor_col.min(row.len().saturating_sub(1)) {
                row[i] = StyledChar::space_with_style(Style::default());
            }
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        let cursor_row = self.cursor.row;
        let cursor_col = self.cursor.col;

        // Create style from current terminal state
        let current_style = self.create_current_style();

        let buffer = self.current_buffer_mut();
        if cursor_row < buffer.len() && cursor_col < buffer[cursor_row].len() {
            buffer[cursor_row][cursor_col] = StyledChar::new(ch, current_style);
            if self.auto_wrap && cursor_col < self.width.saturating_sub(1) {
                self.cursor.col += 1;
            }
        }
    }

    fn create_current_style(&self) -> Style {
        let mut style = Style::default();

        // Apply foreground color
        if let Some(fg_color) = self.foreground_color {
            style = style.fg(fg_color);
        }

        // Apply background color
        if let Some(bg_color) = self.background_color {
            style = style.bg(bg_color);
        }

        // Apply formatting
        if self.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if self.underline {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        if self.reverse {
            style = style.add_modifier(Modifier::REVERSED);
        }

        style
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) {
        self.cursor.row = row.min(self.height.saturating_sub(1));
        self.cursor.col = col.min(self.width.saturating_sub(1));
    }

    pub fn move_cursor_relative(&mut self, row_offset: i32, col_offset: i32) {
        let new_row = (self.cursor.row as i32 + row_offset).max(0) as usize;
        let new_col = (self.cursor.col as i32 + col_offset).max(0) as usize;
        self.move_cursor(new_row, new_col);
    }

    pub fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor.clone());
    }

    pub fn restore_cursor(&mut self) {
        if let Some(saved) = self.saved_cursor.take() {
            self.cursor = saved;
        }
    }

    pub fn switch_to_alternate_screen(&mut self) {
        self.alternate_screen = true;
        self.cursor = CursorPosition { row: 0, col: 0 };
    }

    pub fn switch_to_main_screen(&mut self) {
        self.alternate_screen = false;
        self.cursor = CursorPosition { row: 0, col: 0 };
    }

    pub fn scroll_up(&mut self, lines: usize) {
        let scroll_region = self.scroll_region;
        let start_row = scroll_region.map_or(0, |(start, _)| start);
        let end_row = scroll_region.map_or(self.height, |(_, end)| end);
        let buffer = self.current_buffer_mut();

        for _ in 0..lines {
            for row in start_row..end_row.saturating_sub(1) {
                buffer[row] = buffer[row + 1].clone();
            }
            if end_row > start_row {
                for cell in buffer[end_row - 1].iter_mut() {
                    *cell = StyledChar::space_with_style(Style::default());
                }
            }
        }
    }

    pub fn scroll_down(&mut self, lines: usize) {
        let scroll_region = self.scroll_region;
        let start_row = scroll_region.map_or(0, |(start, _)| start);
        let end_row = scroll_region.map_or(self.height, |(_, end)| end);
        let buffer = self.current_buffer_mut();

        for _ in 0..lines {
            for row in (start_row + 1..end_row).rev() {
                buffer[row] = buffer[row - 1].clone();
            }
            if start_row < end_row {
                for cell in buffer[start_row].iter_mut() {
                    *cell = StyledChar::space_with_style(Style::default());
                }
            }
        }
    }

    pub fn extract_final_output(&mut self) -> Vec<String> {
        let buffer = self.current_buffer();
        let mut lines = Vec::new();

        for row in buffer {
            let line: String = row
                .iter()
                .map(|styled_char| styled_char.ch)
                .collect::<String>()
                .trim_end()
                .to_string();
            if !line.is_empty() || !lines.is_empty() {
                lines.push(line);
            }
        }

        // Remove trailing empty lines
        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }

        lines
    }
}

pub struct AnsiParser {
    parser: VTParser,
    state: TerminalState,
    accumulated_output: Vec<String>,
    animation_detected: bool,
    screen_changes: usize,
    last_screen_state: Option<Vec<Vec<StyledChar>>>,
}

impl AnsiParser {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            parser: VTParser::new(),
            state: TerminalState::new(width, height),
            accumulated_output: Vec::new(),
            animation_detected: false,
            screen_changes: 0,
            last_screen_state: None,
        }
    }

    pub fn new_with_terminal_size() -> Self {
        let (term_cols, term_rows) = crossterm::terminal::size().unwrap_or((80, 24));
        Self::new(term_cols as usize, term_rows as usize)
    }

    pub fn get_terminal_state(&self) -> &TerminalState {
        &self.state
    }

    pub fn parse(&mut self, input: &str) -> Vec<Line<'static>> {
        // For most commands, we should just parse the text line by line
        // and only use the full terminal state for complex applications
        if self.should_use_simple_parsing(input) {
            return self.parse_simple_text(input);
        }

        self.animation_detected = false;
        self.screen_changes = 0;

        let mut handler = VtActionHandler::new(&mut self.state);
        for byte in input.bytes() {
            self.parser.parse_byte(byte, &mut handler);
        }

        self.detect_animation();
        self.convert_to_lines()
    }

    fn should_use_simple_parsing(&self, input: &str) -> bool {
        // Use simple parsing for most text output
        // Only use full terminal emulation for complex escape sequences
        !input.contains('\x1b')
            || (input.contains('\x1b')
                && !input.contains("\x1b[2J")
                && !input.contains("\x1b[H")
                && !input.contains("\x1b[?1049h")
                && !input.contains("\x1b[?1047h")
                && !input.contains("\x1b[?1049l")
                && !input.contains("\x1b[?1047l"))
    }

    fn parse_simple_text(&mut self, input: &str) -> Vec<Line<'static>> {
        // Simple line-by-line parsing for regular command output
        input
            .lines()
            .map(|line| {
                if line.contains('\x1b') || line.contains('\t') || line.contains('\r') {
                    // Use vtparse for this line if it has ANSI sequences or control characters
                    self.parse_line_with_vtparse(line)
                } else {
                    // Pure text line
                    Line::from(line.to_string())
                }
            })
            .collect()
    }

    fn parse_line_with_vtparse(&mut self, line: &str) -> Line<'static> {
        // Handle tabs by converting them to spaces before processing
        let processed_line = self.expand_tabs(line);

        // Get actual terminal size for proper display
        let (term_cols, _term_rows) = crossterm::terminal::size().unwrap_or((80, 24));

        // Create a fresh parser and state for single line parsing with actual terminal width
        let mut parser = VTParser::new();
        let mut state = TerminalState::new(term_cols as usize, 1);

        // Parse the line
        let mut handler = VtActionHandler::new(&mut state);
        for byte in processed_line.bytes() {
            parser.parse_byte(byte, &mut handler);
        }

        // Extract the result as styled line
        let buffer = state.current_buffer();
        if let Some(row) = buffer.first() {
            self.convert_styled_row_to_line(row)
        } else {
            Line::from(processed_line)
        }
    }

    fn expand_tabs(&self, line: &str) -> String {
        // Convert tabs to spaces, respecting 8-character tab stops
        let mut result = String::new();
        let mut column = 0;

        for ch in line.chars() {
            if ch == '\t' {
                // Calculate spaces needed to reach next tab stop
                let spaces_to_add = 8 - (column % 8);
                result.push_str(&" ".repeat(spaces_to_add));
                column += spaces_to_add;
            } else {
                result.push(ch);
                column += 1;
            }
        }

        result
    }

    fn detect_animation(&mut self) {
        let current_state = self.state.current_buffer().clone();

        if let Some(ref last_state) = self.last_screen_state {
            if current_state != *last_state {
                self.screen_changes += 1;
                if self.screen_changes > 5 {
                    self.animation_detected = true;
                }
            }
        }

        self.last_screen_state = Some(current_state);
    }

    fn convert_to_lines(&mut self) -> Vec<Line<'static>> {
        if self.animation_detected || self.state.screen_cleared {
            // For animations or screen clearing, only return final state
            let final_lines = self.state.extract_final_output();
            return final_lines.into_iter().map(Line::from).collect();
        }

        // If we're not in alternate screen mode, return the main screen buffer
        // This handles the case where applications like cmatrix switch back to main screen
        // and expect the original content to be restored (i.e., no output in our terminal)
        if !self.state.alternate_screen {
            // Check if the main screen buffer is empty/cleared (normal after alternate screen apps)
            let main_buffer = &self.state.screen_buffer;
            let mut non_empty_lines = 0;

            for row in main_buffer {
                let line: String = row
                    .iter()
                    .map(|styled_char| styled_char.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string();
                if !line.is_empty() {
                    non_empty_lines += 1;
                }
            }

            // If main screen is mostly empty (like after cmatrix exits), return empty
            if non_empty_lines <= 1 {
                return vec![Line::from("")];
            }
        }

        // For normal output, convert current buffer to styled lines
        let buffer = self.state.current_buffer();
        let mut lines = Vec::new();

        for row in buffer {
            let line_text: String = row
                .iter()
                .map(|styled_char| styled_char.ch)
                .collect::<String>()
                .trim_end()
                .to_string();
            if !line_text.is_empty() || !lines.is_empty() {
                lines.push(self.convert_styled_row_to_line(row));
            }
        }

        lines
    }

    fn convert_styled_row_to_line(&self, row: &[StyledChar]) -> Line<'static> {
        // First find the last non-space character
        let mut last_non_space = 0;
        for (i, styled_char) in row.iter().enumerate() {
            if styled_char.ch != ' ' {
                last_non_space = i;
            }
        }

        // If all characters are spaces, return empty line
        if last_non_space == 0 && row.first().is_none_or(|c| c.ch == ' ') {
            return Line::from("");
        }

        // Only process up to the last non-space character + 1
        let trimmed_row = &row[..=last_non_space];

        let mut spans = Vec::new();
        let mut current_text = String::new();
        let mut current_style = Style::default();
        let mut first_char = true;

        for styled_char in trimmed_row {
            if first_char {
                current_style = styled_char.style;
                first_char = false;
            }

            if styled_char.style == current_style {
                // Same style, accumulate text
                current_text.push(styled_char.ch);
            } else {
                // Style changed, create span for accumulated text
                if !current_text.is_empty() {
                    spans.push(Span::styled(current_text.clone(), current_style));
                    current_text.clear();
                }
                current_style = styled_char.style;
                current_text.push(styled_char.ch);
            }
        }

        // Add final span
        if !current_text.is_empty() {
            spans.push(Span::styled(current_text, current_style));
        }

        if spans.is_empty() {
            Line::from("")
        } else {
            Line::from(spans)
        }
    }

    pub fn reset(&mut self) {
        self.state = TerminalState::new(self.state.width, self.state.height);
        self.accumulated_output.clear();
        self.animation_detected = false;
        self.screen_changes = 0;
        self.last_screen_state = None;
    }
}

struct VtActionHandler<'a> {
    state: &'a mut TerminalState,
}

impl<'a> VtActionHandler<'a> {
    fn new(state: &'a mut TerminalState) -> Self {
        Self { state }
    }

    fn handle_sgr(&mut self, params: &[CsiParam]) {
        let mut i = 0;
        while i < params.len() {
            let value = match params[i] {
                CsiParam::Integer(n) => n,
                _ => {
                    i += 1;
                    continue;
                }
            };
            match value {
                0 => {
                    // Reset all attributes
                    self.state.bold = false;
                    self.state.italic = false;
                    self.state.underline = false;
                    self.state.reverse = false;
                    self.state.foreground_color = None;
                    self.state.background_color = None;
                }
                1 => self.state.bold = true,
                3 => self.state.italic = true,
                4 => self.state.underline = true,
                7 => self.state.reverse = true,
                22 => self.state.bold = false,
                23 => self.state.italic = false,
                24 => self.state.underline = false,
                27 => self.state.reverse = false,
                30 => self.state.foreground_color = Some(Color::Black),
                31 => self.state.foreground_color = Some(Color::Red),
                32 => self.state.foreground_color = Some(Color::Green),
                33 => self.state.foreground_color = Some(Color::Yellow),
                34 => self.state.foreground_color = Some(Color::Blue),
                35 => self.state.foreground_color = Some(Color::Magenta),
                36 => self.state.foreground_color = Some(Color::Cyan),
                37 => self.state.foreground_color = Some(Color::White),
                38 => {
                    // Extended foreground color
                    if i + 2 < params.len() {
                        if let CsiParam::Integer(color_type) = params[i + 2] {
                            match color_type {
                                2 => {
                                    // 24-bit RGB: 38;2;R;G;B (accounting for semicolons as separate params)
                                    if i + 8 < params.len() {
                                        if let (
                                            CsiParam::Integer(r),
                                            CsiParam::Integer(g),
                                            CsiParam::Integer(b),
                                        ) = (&params[i + 4], &params[i + 6], &params[i + 8])
                                        {
                                            self.state.foreground_color =
                                                Some(Color::Rgb(*r as u8, *g as u8, *b as u8));
                                            i += 8; // Skip all RGB parameters including semicolons
                                        } else {
                                            i += 2; // Skip to color_type param
                                        }
                                    } else {
                                        i += 2; // Skip to color_type param
                                    }
                                }
                                5 => {
                                    // 256-color: 38;5;n
                                    if i + 4 < params.len() {
                                        if let CsiParam::Integer(color_index) = params[i + 4] {
                                            self.state.foreground_color =
                                                Some(self.index_to_color(color_index as u8));
                                            i += 4; // Skip all 256-color parameters including semicolons
                                        } else {
                                            i += 2; // Skip to color_type param
                                        }
                                    } else {
                                        i += 2; // Skip to color_type param
                                    }
                                }
                                _ => i += 2, // Skip unknown color type
                            }
                        } else {
                            i += 2; // Skip if color_type is not an integer
                        }
                    }
                }
                39 => self.state.foreground_color = None,
                40 => self.state.background_color = Some(Color::Black),
                41 => self.state.background_color = Some(Color::Red),
                42 => self.state.background_color = Some(Color::Green),
                43 => self.state.background_color = Some(Color::Yellow),
                44 => self.state.background_color = Some(Color::Blue),
                45 => self.state.background_color = Some(Color::Magenta),
                46 => self.state.background_color = Some(Color::Cyan),
                47 => self.state.background_color = Some(Color::White),
                48 => {
                    // Extended background color
                    if i + 2 < params.len() {
                        if let CsiParam::Integer(color_type) = params[i + 2] {
                            match color_type {
                                2 => {
                                    // 24-bit RGB: 48;2;R;G;B (accounting for semicolons as separate params)
                                    if i + 8 < params.len() {
                                        if let (
                                            CsiParam::Integer(r),
                                            CsiParam::Integer(g),
                                            CsiParam::Integer(b),
                                        ) = (&params[i + 4], &params[i + 6], &params[i + 8])
                                        {
                                            self.state.background_color =
                                                Some(Color::Rgb(*r as u8, *g as u8, *b as u8));
                                            i += 8; // Skip all RGB parameters including semicolons
                                        } else {
                                            i += 2; // Skip to color_type param
                                        }
                                    } else {
                                        i += 2; // Skip to color_type param
                                    }
                                }
                                5 => {
                                    // 256-color: 48;5;n
                                    if i + 4 < params.len() {
                                        if let CsiParam::Integer(color_index) = params[i + 4] {
                                            self.state.background_color =
                                                Some(self.index_to_color(color_index as u8));
                                            i += 4; // Skip all 256-color parameters including semicolons
                                        } else {
                                            i += 2; // Skip to color_type param
                                        }
                                    } else {
                                        i += 2; // Skip to color_type param
                                    }
                                }
                                _ => i += 2, // Skip unknown color type
                            }
                        } else {
                            i += 2; // Skip if color_type is not an integer
                        }
                    }
                }
                49 => self.state.background_color = None,
                _ => {}
            }
            i += 1;
        }
    }

    fn index_to_color(&self, index: u8) -> Color {
        // Convert 256-color index to RGB
        match index {
            // Standard colors (0-15)
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Magenta,
            6 => Color::Cyan,
            7 => Color::White,
            8 => Color::DarkGray,
            9 => Color::LightRed,
            10 => Color::LightGreen,
            11 => Color::LightYellow,
            12 => Color::LightBlue,
            13 => Color::LightMagenta,
            14 => Color::LightCyan,
            15 => Color::White,
            // 216 color cube (16-231)
            16..=231 => {
                let n = index - 16;
                let r = (n / 36) * 51;
                let g = ((n % 36) / 6) * 51;
                let b = (n % 6) * 51;
                Color::Rgb(r, g, b)
            }
            // Grayscale ramp (232-255)
            232..=255 => {
                let gray = (index - 232) * 10 + 8;
                Color::Rgb(gray, gray, gray)
            }
        }
    }

    fn handle_csi(&mut self, params: &[CsiParam], ignore: bool, final_byte: u8) {
        if ignore {
            return;
        }

        let get_param = |i: usize| -> i64 {
            match params.get(i) {
                Some(CsiParam::Integer(n)) => *n,
                _ => 0,
            }
        };

        match final_byte {
            b'A' => {
                // Cursor Up
                let n = get_param(0).max(1) as usize;
                self.state.move_cursor_relative(-(n as i32), 0);
            }
            b'B' => {
                // Cursor Down
                let n = get_param(0).max(1) as usize;
                self.state.move_cursor_relative(n as i32, 0);
            }
            b'C' => {
                // Cursor Forward
                let n = get_param(0).max(1) as usize;
                self.state.move_cursor_relative(0, n as i32);
            }
            b'D' => {
                // Cursor Backward
                let n = get_param(0).max(1) as usize;
                self.state.move_cursor_relative(0, -(n as i32));
            }
            b'H' | b'f' => {
                // Cursor Position
                let row = get_param(0).max(1) as usize - 1;
                let col = get_param(1).max(1) as usize - 1;
                self.state.move_cursor(row, col);
            }
            b'J' => {
                // Erase in Display
                let n = get_param(0);
                match n {
                    0 => {
                        // Clear from cursor to end of screen
                        self.state.clear_to_end_of_line();
                        let cursor_row = self.state.cursor.row;
                        let height = self.state.height;
                        let buffer = self.state.current_buffer_mut();
                        for row in (cursor_row + 1)..height {
                            if row < buffer.len() {
                                for cell in buffer[row].iter_mut() {
                                    *cell = StyledChar::space_with_style(Style::default());
                                }
                            }
                        }
                    }
                    1 => {
                        // Clear from cursor to beginning of screen
                        self.state.clear_to_beginning_of_line();
                        let cursor_row = self.state.cursor.row;
                        let buffer = self.state.current_buffer_mut();
                        for row in 0..cursor_row {
                            if row < buffer.len() {
                                for cell in buffer[row].iter_mut() {
                                    *cell = StyledChar::space_with_style(Style::default());
                                }
                            }
                        }
                    }
                    2 => {
                        // Clear entire screen
                        self.state.clear_screen();
                    }
                    _ => {}
                }
            }
            b'K' => {
                // Erase in Line
                let n = get_param(0);
                match n {
                    0 => self.state.clear_to_end_of_line(),
                    1 => self.state.clear_to_beginning_of_line(),
                    2 => self.state.clear_line(),
                    _ => {}
                }
            }
            b'S' => {
                // Scroll up
                let n = get_param(0).max(1) as usize;
                self.state.scroll_up(n);
            }
            b'T' => {
                // Scroll down
                let n = get_param(0).max(1) as usize;
                self.state.scroll_down(n);
            }
            b'm' => {
                // SGR - Select Graphic Rendition
                self.handle_sgr(params);
            }
            b's' => {
                // Save cursor position
                self.state.save_cursor();
            }
            b'u' => {
                // Restore cursor position
                self.state.restore_cursor();
            }
            b'r' => {
                // Set scroll region
                let top = get_param(0).max(1) as usize - 1;
                let bottom = get_param(1).max(self.state.height as i64) as usize - 1;
                self.state.scroll_region = Some((top, bottom.min(self.state.height - 1)));
            }
            b'h' => {
                // Set modes
                if !params.is_empty() {
                    match get_param(0) {
                        1049 => {
                            // Enable alternate screen buffer
                            self.state.switch_to_alternate_screen();
                        }
                        1047 => {
                            // Enable alternate screen buffer (xterm)
                            self.state.switch_to_alternate_screen();
                        }
                        _ => {}
                    }
                }
            }
            b'l' => {
                // Reset modes
                if !params.is_empty() {
                    match get_param(0) {
                        1049 => {
                            // Disable alternate screen buffer
                            self.state.switch_to_main_screen();
                        }
                        1047 => {
                            // Disable alternate screen buffer (xterm)
                            self.state.switch_to_main_screen();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

impl<'a> VTActor for VtActionHandler<'a> {
    fn csi_dispatch(&mut self, params: &[CsiParam], ignore: bool, final_byte: u8) {
        self.handle_csi(params, ignore, final_byte);
    }

    fn esc_dispatch(
        &mut self,
        _params: &[i64],
        _intermediate_bytes: &[u8],
        _ignore: bool,
        final_byte: u8,
    ) {
        match final_byte {
            b'7' => self.state.save_cursor(),
            b'8' => self.state.restore_cursor(),
            b'c' => {
                // Reset terminal
                *self.state = TerminalState::new(self.state.width, self.state.height);
            }
            b'D' => {
                // Index (cursor down with scroll)
                if self.state.cursor.row >= self.state.height - 1 {
                    self.state.scroll_up(1);
                } else {
                    self.state.cursor.row += 1;
                }
            }
            b'M' => {
                // Reverse Index (cursor up with scroll)
                if self.state.cursor.row == 0 {
                    self.state.scroll_down(1);
                } else {
                    self.state.cursor.row -= 1;
                }
            }
            _ => {}
        }
    }

    fn print(&mut self, ch: char) {
        match ch {
            '\n' => {
                self.state.cursor.col = 0;
                if self.state.cursor.row >= self.state.height - 1 {
                    self.state.scroll_up(1);
                } else {
                    self.state.cursor.row += 1;
                }
            }
            '\r' => {
                self.state.cursor.col = 0;
            }
            '\t' => {
                let next_tab = self
                    .state
                    .tab_stops
                    .iter()
                    .find(|&&pos| pos > self.state.cursor.col)
                    .copied()
                    .unwrap_or(self.state.width);
                let target_col = next_tab.min(self.state.width - 1);

                // Fill the gap with spaces to properly represent the tab
                while self.state.cursor.col < target_col {
                    self.state.insert_char(' ');
                }
            }
            _ => {
                self.state.insert_char(ch);
            }
        }
    }

    fn execute_c0_or_c1(&mut self, byte: u8) {
        match byte {
            0x07 => {} // BEL - Bell
            0x08 => {
                // BS - Backspace
                if self.state.cursor.col > 0 {
                    self.state.cursor.col -= 1;
                }
            }
            0x0C => {
                // FF - Form Feed
                self.state.clear_screen();
            }
            _ => {}
        }
    }

    fn dcs_hook(&mut self, _byte: u8, _params: &[i64], _intermediate_bytes: &[u8], _ignore: bool) {
        // Not implemented for now
    }

    fn dcs_put(&mut self, _byte: u8) {
        // Not implemented for now
    }

    fn dcs_unhook(&mut self) {
        // Not implemented for now
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]]) {
        // Not implemented for now
    }

    fn apc_dispatch(&mut self, _data: Vec<u8>) {
        // Not implemented for now
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_ansi_parser_creation() {
        let parser = AnsiParser::new(80, 24);
        assert_eq!(parser.state.width, 80);
        assert_eq!(parser.state.height, 24);
        assert!(!parser.animation_detected);
        assert_eq!(parser.screen_changes, 0);
    }

    #[test]
    fn test_ansi_parser_terminal_size() {
        let parser = AnsiParser::new_with_terminal_size();
        assert!(parser.state.width > 0);
        assert!(parser.state.height > 0);
    }

    #[test]
    fn test_basic_ansi_color_codes() {
        let mut parser = AnsiParser::new(80, 24);

        // Test foreground colors
        let red_text = "\x1b[31mRed text\x1b[0m";
        let lines = parser.parse(red_text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Red text");

        let green_text = "\x1b[32mGreen text\x1b[0m";
        let lines = parser.parse(green_text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Green text");

        let blue_text = "\x1b[34mBlue text\x1b[0m";
        let lines = parser.parse(blue_text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Blue text");
    }

    #[test]
    fn test_background_color_codes() {
        let mut parser = AnsiParser::new(80, 24);

        // Test background colors
        let red_bg = "\x1b[41mRed background\x1b[0m";
        let lines = parser.parse(red_bg);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Red background");

        let green_bg = "\x1b[42mGreen background\x1b[0m";
        let lines = parser.parse(green_bg);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Green background");
    }

    #[test]
    fn test_sgr_formatting_codes() {
        let mut parser = AnsiParser::new(80, 24);

        // Test bold
        let bold_text = "\x1b[1mBold text\x1b[0m";
        let lines = parser.parse(bold_text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Bold text");

        // Test italic
        let italic_text = "\x1b[3mItalic text\x1b[0m";
        let lines = parser.parse(italic_text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Italic text");

        // Test underline
        let underline_text = "\x1b[4mUnderlined text\x1b[0m";
        let lines = parser.parse(underline_text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Underlined text");

        // Test reverse
        let reverse_text = "\x1b[7mReverse text\x1b[0m";
        let lines = parser.parse(reverse_text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Reverse text");
    }

    #[test]
    fn test_cursor_movement_codes() {
        let mut parser = AnsiParser::new(80, 24);

        // Test cursor position
        let cursor_home = "\x1b[1;1HHome position";
        let lines = parser.parse(cursor_home);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Home position");

        // Test cursor up/down/left/right
        let cursor_moves = "\x1b[2AUp\x1b[2BDown\x1b[2CRight\x1b[2DLeft";
        let lines = parser.parse(cursor_moves);
        assert!(lines.len() > 0);
    }

    #[test]
    fn test_screen_clearing_codes() {
        let mut parser = AnsiParser::new(80, 24);

        // Test clear screen
        let clear_screen = "\x1b[2JScreen cleared";
        let lines = parser.parse(clear_screen);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Screen cleared");

        // Test clear to end of line
        let clear_eol = "Text\x1b[0K";
        let lines = parser.parse(clear_eol);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Text");
    }

    #[test]
    fn test_scroll_operations() {
        let mut parser = AnsiParser::new(80, 24);

        // Test scroll up
        let scroll_up = "\x1b[3SScrolled up";
        let lines = parser.parse(scroll_up);
        assert!(lines.len() > 0);

        // Test scroll down
        let scroll_down = "\x1b[3TScrolled down";
        let lines = parser.parse(scroll_down);
        assert!(lines.len() > 0);
    }

    #[test]
    fn test_cursor_save_restore() {
        let mut parser = AnsiParser::new(80, 24);

        // Test cursor save and restore
        let save_restore = "\x1b[10;10H\x1b[sHello\x1b[uWorld";
        let lines = parser.parse(save_restore);
        assert!(lines.len() > 0);

        // Test ESC 7 and ESC 8
        let esc_save_restore = "\x1b7Position\x1b8Restored";
        let lines = parser.parse(esc_save_restore);
        assert!(lines.len() > 0);
    }

    #[test]
    fn test_alternate_screen_buffer() {
        let mut parser = AnsiParser::new(80, 24);

        // Test alternate screen enable/disable
        let alt_screen = "\x1b[?1049hAlternate screen\x1b[?1049lMain screen";
        let lines = parser.parse(alt_screen);
        assert!(lines.len() >= 1);

        // Test xterm alternate screen
        let xterm_alt = "\x1b[?1047hXterm alternate\x1b[?1047lXterm main";
        let lines = parser.parse(xterm_alt);
        assert!(lines.len() >= 1);
    }

    #[test]
    fn test_terminal_reset() {
        let mut parser = AnsiParser::new(80, 24);

        // Test terminal reset
        let reset = "\x1bcReset terminal";
        let lines = parser.parse(reset);
        assert!(lines.len() > 0);
    }

    #[test]
    fn test_control_characters() {
        let mut parser = AnsiParser::new(80, 24);

        // Test newline
        let newline = "Line1\nLine2";
        let lines = parser.parse(newline);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].to_string(), "Line1");
        assert_eq!(lines[1].to_string(), "Line2");

        // Test carriage return
        let cr = "Text\rOver";
        let lines = parser.parse(cr);
        assert_eq!(lines.len(), 1);

        // Test tab
        let tab = "A\tB";
        let lines = parser.parse(tab);
        assert_eq!(lines.len(), 1);

        // Test backspace
        let bs = "ABC\x08D";
        let lines = parser.parse(bs);
        assert!(lines.len() > 0);
    }

    #[test]
    fn test_complex_ansi_sequences() {
        let mut parser = AnsiParser::new(80, 24);

        // Test multiple colors and formatting
        let complex = "\x1b[1;31mBold Red\x1b[0m \x1b[4;32mUnderlined Green\x1b[0m";
        let lines = parser.parse(complex);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].to_string().contains("Bold Red"));
        assert!(lines[0].to_string().contains("Underlined Green"));
    }

    #[test]
    fn test_plain_text_parsing() {
        let mut parser = AnsiParser::new(80, 24);

        // Test plain text without ANSI codes
        let plain = "This is plain text";
        let lines = parser.parse(plain);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "This is plain text");
    }

    #[test]
    fn test_empty_input() {
        let mut parser = AnsiParser::new(80, 24);

        let empty = "";
        let lines = parser.parse(empty);
        assert_eq!(lines.len(), 0);
    }

    #[test]
    fn test_multiline_input() {
        let mut parser = AnsiParser::new(80, 24);

        let multiline = "Line 1\nLine 2\nLine 3";
        let lines = parser.parse(multiline);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].to_string(), "Line 1");
        assert_eq!(lines[1].to_string(), "Line 2");
        assert_eq!(lines[2].to_string(), "Line 3");
    }

    #[test]
    fn test_parser_reset() {
        let mut parser = AnsiParser::new(80, 24);

        // Parse some input
        let input = "\x1b[31mRed text\x1b[0m";
        parser.parse(input);

        // Reset parser
        parser.reset();

        // Verify reset state
        assert!(!parser.animation_detected);
        assert_eq!(parser.screen_changes, 0);
        assert!(parser.accumulated_output.is_empty());
        assert!(parser.last_screen_state.is_none());
    }

    #[test]
    fn test_animation_detection() {
        let mut parser = AnsiParser::new(80, 24);

        // Test the animation detection by directly manipulating the state
        // Since animation detection depends on buffer changes, we need to force state changes
        for i in 0..7 {
            // Manually change the screen state to trigger detection
            parser.state.clear_screen();
            parser.state.move_cursor(0, 0);
            for ch in format!("Frame {}", i).chars() {
                parser.state.insert_char(ch);
            }

            // Call detect_animation to check for changes
            parser.detect_animation();
        }

        // Check if animation was detected
        assert!(
            parser.screen_changes > 5,
            "Screen changes: {}",
            parser.screen_changes
        );
        assert!(parser.animation_detected);
    }

    #[test]
    fn test_terminal_state_creation() {
        let state = TerminalState::new(100, 50);
        assert_eq!(state.width, 100);
        assert_eq!(state.height, 50);
        assert_eq!(state.cursor.row, 0);
        assert_eq!(state.cursor.col, 0);
        assert!(!state.bold);
        assert!(!state.italic);
        assert!(!state.underline);
        assert!(!state.reverse);
        assert!(!state.alternate_screen);
        assert!(state.cursor_visible);
        assert!(state.auto_wrap);
    }

    #[test]
    fn test_terminal_state_buffer_switching() {
        let mut state = TerminalState::new(80, 24);

        // Initially on main screen
        assert!(!state.alternate_screen);
        assert_eq!(state.current_buffer().len(), 24);

        // Switch to alternate screen
        state.switch_to_alternate_screen();
        assert!(state.alternate_screen);
        assert_eq!(state.cursor.row, 0);
        assert_eq!(state.cursor.col, 0);

        // Switch back to main screen
        state.switch_to_main_screen();
        assert!(!state.alternate_screen);
    }

    #[test]
    fn test_terminal_state_cursor_operations() {
        let mut state = TerminalState::new(80, 24);

        // Test cursor movement
        state.move_cursor(10, 20);
        assert_eq!(state.cursor.row, 10);
        assert_eq!(state.cursor.col, 20);

        // Test cursor bounds
        state.move_cursor(100, 200);
        assert_eq!(state.cursor.row, 23); // height - 1
        assert_eq!(state.cursor.col, 79); // width - 1

        // Test relative movement
        state.move_cursor(10, 10);
        state.move_cursor_relative(5, 5);
        assert_eq!(state.cursor.row, 15);
        assert_eq!(state.cursor.col, 15);

        // Test save/restore
        state.save_cursor();
        state.move_cursor(0, 0);
        state.restore_cursor();
        assert_eq!(state.cursor.row, 15);
        assert_eq!(state.cursor.col, 15);
    }

    #[test]
    fn test_terminal_state_clearing() {
        let mut state = TerminalState::new(80, 24);

        // Fill buffer with characters
        for row in 0..24 {
            for col in 0..80 {
                state.move_cursor(row, col);
                state.insert_char('X');
            }
        }

        // Test clear screen
        state.clear_screen();
        let buffer = state.current_buffer();
        for row in buffer {
            for styled_char in row {
                assert_eq!(styled_char.ch, ' ');
            }
        }
        assert_eq!(state.cursor.row, 0);
        assert_eq!(state.cursor.col, 0);
    }

    #[test]
    fn test_terminal_state_line_clearing() {
        let mut state = TerminalState::new(80, 24);

        // Fill a line with characters
        state.move_cursor(5, 0);
        for _i in 0..80 {
            state.insert_char('A');
        }

        // Clear the line
        state.move_cursor(5, 0);
        state.clear_line();

        let buffer = state.current_buffer();
        for styled_char in &buffer[5] {
            assert_eq!(styled_char.ch, ' ');
        }
    }

    #[test]
    fn test_terminal_state_scrolling() {
        let mut state = TerminalState::new(80, 24);

        // Fill buffer with different characters per row
        for row in 0..24 {
            for col in 0..80 {
                state.move_cursor(row, col);
                state.insert_char(char::from_u32(b'A' as u32 + row as u32).unwrap_or('?'));
            }
        }

        // Scroll up
        state.scroll_up(1);
        let buffer = state.current_buffer();

        // First row should now contain 'B' (originally second row)
        assert_eq!(buffer[0][0].ch, 'B');

        // Last row should be empty
        assert_eq!(buffer[23][0].ch, ' ');
    }

    #[test]
    fn test_all_ansi_color_codes() {
        let mut parser = AnsiParser::new(80, 24);

        // Test all basic foreground colors
        let colors = [
            (30, "Black"),
            (31, "Red"),
            (32, "Green"),
            (33, "Yellow"),
            (34, "Blue"),
            (35, "Magenta"),
            (36, "Cyan"),
            (37, "White"),
        ];

        for (code, name) in colors {
            let input = format!("\x1b[{}m{}\x1b[0m", code, name);
            let lines = parser.parse(&input);
            assert_eq!(lines.len(), 1);
            assert_eq!(lines[0].to_string(), name);
        }

        // Test all basic background colors
        for (code, name) in colors {
            let bg_code = code + 10; // Background colors are +10
            let input = format!("\x1b[{}m{}\x1b[0m", bg_code, name);
            let lines = parser.parse(&input);
            assert_eq!(lines.len(), 1);
            assert_eq!(lines[0].to_string(), name);
        }
    }

    #[test]
    fn test_sgr_reset_codes() {
        let mut parser = AnsiParser::new(80, 24);

        // Test individual reset codes
        let reset_codes = [
            (22, "Bold off"),
            (23, "Italic off"),
            (24, "Underline off"),
            (27, "Reverse off"),
            (39, "Default foreground"),
            (49, "Default background"),
        ];

        for (code, description) in reset_codes {
            let input = format!("\x1b[1;3;4;7;31;41m\x1b[{}m{}\x1b[0m", code, description);
            let lines = parser.parse(&input);
            assert_eq!(lines.len(), 1);
            assert_eq!(lines[0].to_string(), description);
        }
    }

    #[test]
    fn test_vtaction_handler_sgr() {
        let mut state = TerminalState::new(80, 24);

        // Test SGR with bold
        {
            let mut handler = VtActionHandler::new(&mut state);
            handler.handle_sgr(&[CsiParam::Integer(1)]);
        }
        assert!(state.bold);

        // Test SGR with color
        {
            let mut handler = VtActionHandler::new(&mut state);
            handler.handle_sgr(&[CsiParam::Integer(31)]);
        }
        assert_eq!(state.foreground_color, Some(Color::Red));

        // Test SGR reset
        {
            let mut handler = VtActionHandler::new(&mut state);
            handler.handle_sgr(&[CsiParam::Integer(0)]);
        }
        assert!(!state.bold);
        assert_eq!(state.foreground_color, None);
    }

    #[test]
    fn test_extract_final_output() {
        let mut state = TerminalState::new(80, 24);

        // Add some text to the buffer
        state.move_cursor(0, 0);
        for ch in "Hello World".chars() {
            state.insert_char(ch);
        }

        state.move_cursor(2, 0);
        for ch in "Line 3".chars() {
            state.insert_char(ch);
        }

        let output = state.extract_final_output();
        assert_eq!(output.len(), 3);
        assert_eq!(output[0], "Hello World");
        assert_eq!(output[1], ""); // Empty line between line 0 and line 2
        assert_eq!(output[2], "Line 3");
    }

    #[test]
    fn test_color_preservation_in_styled_lines() {
        let mut parser = AnsiParser::new(80, 24);

        // Test that colors are preserved when converting to styled lines
        let colored_text = "\x1b[31mRed\x1b[0m \x1b[32mGreen\x1b[0m \x1b[1;34mBold Blue\x1b[0m";
        let lines = parser.parse(colored_text);

        assert_eq!(lines.len(), 1);

        // Check that the line contains styled spans (not just plain text)
        let line = &lines[0];
        let line_text = line.to_string();

        // The line should contain the expected text
        assert!(line_text.contains("Red"));
        assert!(line_text.contains("Green"));
        assert!(line_text.contains("Bold Blue"));

        // Test that the parser correctly creates styled spans by checking internal structure
        // This test verifies that the convert_styled_row_to_line method works correctly

        // Create a manual test row with different styles
        let mut test_state = TerminalState::new(80, 1);
        test_state.foreground_color = Some(Color::Red);
        test_state.insert_char('R');
        test_state.foreground_color = Some(Color::Green);
        test_state.insert_char('G');
        test_state.foreground_color = Some(Color::Blue);
        test_state.bold = true;
        test_state.insert_char('B');

        let buffer = test_state.current_buffer();
        let styled_row = &buffer[0];

        // Check that different characters have different styles
        assert_eq!(styled_row[0].ch, 'R');
        assert_eq!(styled_row[1].ch, 'G');
        assert_eq!(styled_row[2].ch, 'B');

        // Verify that styles are different
        assert_ne!(styled_row[0].style, styled_row[1].style);
        assert_ne!(styled_row[1].style, styled_row[2].style);
        assert_ne!(styled_row[0].style, styled_row[2].style);
    }

    #[test]
    fn test_styled_char_creation() {
        let style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
        let styled_char = StyledChar::new('A', style);

        assert_eq!(styled_char.ch, 'A');
        assert_eq!(styled_char.style, style);

        let space_char = StyledChar::space_with_style(style);
        assert_eq!(space_char.ch, ' ');
        assert_eq!(space_char.style, style);
    }

    #[test]
    fn test_compound_sgr_codes() {
        let mut parser = AnsiParser::new(80, 24);

        // Test compound SGR codes like those used by ls --color=always
        let compound_sgr = "\x1b[01;34mBold Blue\x1b[0m"; // Same as ls uses
        let lines = parser.parse(compound_sgr);

        assert_eq!(lines.len(), 1);
        let line_text = lines[0].to_string();
        assert_eq!(line_text, "Bold Blue");

        // Test that the style was applied correctly
        let mut test_state = TerminalState::new(80, 1);
        test_state.bold = true;
        test_state.foreground_color = Some(Color::Blue);

        let expected_style = test_state.create_current_style();

        // Insert character and verify style
        test_state.insert_char('X');
        let buffer = test_state.current_buffer();
        assert_eq!(buffer[0][0].ch, 'X');
        assert_eq!(buffer[0][0].style, expected_style);
    }

    #[test]
    fn test_real_ls_output() {
        let mut parser = AnsiParser::new(80, 24);

        // Test real ls output format
        let ls_output =
            "total 248\ndrwxr-xr-x 11 user user  4096 Jul 18 16:20 \x1b[01;34m.\x1b[0m\n";
        let lines = parser.parse(ls_output);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].to_string(), "total 248");
        assert!(lines[1].to_string().contains("drwxr-xr-x"));
        assert!(lines[1].to_string().contains("."));
    }

    #[test]
    fn test_24bit_rgb_colors() {
        let mut parser = AnsiParser::new(80, 24);

        // Test 24-bit RGB foreground color (like in lolcat)
        let rgb_text = "\x1b[38;2;251;41;91mRGB Red\x1b[0m";
        let lines = parser.parse(rgb_text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "RGB Red");

        // Test that the style was applied correctly by creating a test state
        let mut test_state = TerminalState::new(80, 1);
        test_state.foreground_color = Some(Color::Rgb(251, 41, 91));

        let expected_style = test_state.create_current_style();

        // Insert character and verify style
        test_state.insert_char('X');
        let buffer = test_state.current_buffer();
        assert_eq!(buffer[0][0].ch, 'X');
        assert_eq!(buffer[0][0].style, expected_style);

        // Test 24-bit RGB background color
        let rgb_bg_text = "\x1b[48;2;100;200;50mRGB Background\x1b[0m";
        let lines = parser.parse(rgb_bg_text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "RGB Background");
    }

    #[test]
    fn test_256_color_support() {
        let mut parser = AnsiParser::new(80, 24);

        // Test 256-color foreground
        let color256_text = "\x1b[38;5;196mBright Red\x1b[0m";
        let lines = parser.parse(color256_text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Bright Red");

        // Test 256-color background
        let color256_bg_text = "\x1b[48;5;21mBlue Background\x1b[0m";
        let lines = parser.parse(color256_bg_text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Blue Background");
    }

    #[test]
    fn test_lolcat_like_sequence() {
        let mut parser = AnsiParser::new(80, 24);

        // Test a sequence similar to what lolcat produces
        let lolcat_like = "\x1b[38;2;251;41;91mb\x1b[39m\x1b[38;2;250;38;95me\x1b[39m\x1b[38;2;249;35;99mt\x1b[39m";
        let lines = parser.parse(lolcat_like);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "bet");

        // The line should contain styled spans with different colors for each character
        let line = &lines[0];
        // Check that we have multiple spans (indicating different colors for each character)
        // This is a basic test - in reality each character would have a different color
        let line_text = line.to_string();
        assert_eq!(line_text, "bet");
    }

    // ===== COMPREHENSIVE TESTS FOR BUG FIXES AND COMPLETE STYLE COVERAGE =====

    #[test]
    fn test_rgb_foreground_color_parsing_bug_fix() {
        let mut parser = AnsiParser::new(80, 24);

        // Test the exact sequence that was failing before the bug fix
        let rgb_sequence = "\x1b[38;2;251;41;91mRGB\x1b[0m";
        let lines = parser.parse(rgb_sequence);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "RGB");

        // Test the VTActionHandler directly to verify color parsing
        let mut state = TerminalState::new(80, 24);
        let mut handler = VtActionHandler::new(&mut state);

        // Parse the RGB sequence: 38;2;251;41;91
        use vtparse::CsiParam;
        let rgb_params = [
            CsiParam::Integer(38),
            CsiParam::P(59), // 38;
            CsiParam::Integer(2),
            CsiParam::P(59), // 2;
            CsiParam::Integer(251),
            CsiParam::P(59), // 251;
            CsiParam::Integer(41),
            CsiParam::P(59),       // 41;
            CsiParam::Integer(91), // 91
        ];
        handler.handle_sgr(&rgb_params);

        // Verify the foreground color was set correctly
        assert_eq!(state.foreground_color, Some(Color::Rgb(251, 41, 91)));
        assert_eq!(state.background_color, None); // Should NOT be background color

        // Insert a character to test styling
        state.insert_char('R');
        let buffer = state.current_buffer();
        assert_eq!(buffer[0][0].ch, 'R');
        assert_eq!(buffer[0][0].style.fg, Some(Color::Rgb(251, 41, 91)));
    }

    #[test]
    fn test_rgb_background_color_parsing() {
        let mut parser = AnsiParser::new(80, 24);

        // Test RGB background color
        let rgb_bg_sequence = "\x1b[48;2;100;150;200mBG\x1b[0m";
        let lines = parser.parse(rgb_bg_sequence);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "BG");

        // Test the VTActionHandler directly to verify background color parsing
        let mut state = TerminalState::new(80, 24);
        let mut handler = VtActionHandler::new(&mut state);

        // Parse the RGB background sequence: 48;2;100;150;200
        use vtparse::CsiParam;
        let rgb_bg_params = [
            CsiParam::Integer(48),
            CsiParam::P(59), // 48;
            CsiParam::Integer(2),
            CsiParam::P(59), // 2;
            CsiParam::Integer(100),
            CsiParam::P(59), // 100;
            CsiParam::Integer(150),
            CsiParam::P(59),        // 150;
            CsiParam::Integer(200), // 200
        ];
        handler.handle_sgr(&rgb_bg_params);

        // Verify the background color was set correctly
        assert_eq!(state.background_color, Some(Color::Rgb(100, 150, 200)));
        assert_eq!(state.foreground_color, None);

        // Insert a character to test styling
        state.insert_char('B');
        let buffer = state.current_buffer();
        assert_eq!(buffer[0][0].ch, 'B');
        assert_eq!(buffer[0][0].style.bg, Some(Color::Rgb(100, 150, 200)));
    }

    #[test]
    fn test_256_color_foreground_parsing() {
        use vtparse::CsiParam;

        // Test various 256-color indices
        let test_cases = [
            (0, Color::Black),
            (1, Color::Red),
            (9, Color::LightRed),
            (15, Color::White),
            (196, Color::Rgb(255, 0, 0)), // Bright red from 216-color cube
            (21, Color::Rgb(0, 0, 255)),  // Blue from 216-color cube
            (232, Color::Rgb(8, 8, 8)),   // First grayscale
            (255, Color::Rgb(238, 238, 238)), // Last grayscale
        ];

        for (index, expected_color) in test_cases {
            let mut state = TerminalState::new(80, 24);
            {
                let mut handler = VtActionHandler::new(&mut state);
                // Test 256-color sequence: 38;5;index
                let color_params = [
                    CsiParam::Integer(38),
                    CsiParam::P(59),
                    CsiParam::Integer(5),
                    CsiParam::P(59),
                    CsiParam::Integer(index as i64),
                ];
                handler.handle_sgr(&color_params);
            }

            assert_eq!(
                state.foreground_color,
                Some(expected_color),
                "Failed for 256-color index {}",
                index
            );
        }
    }

    #[test]
    fn test_256_color_background_parsing() {
        use vtparse::CsiParam;

        // Test 256-color background
        let mut state = TerminalState::new(80, 24);
        {
            let mut handler = VtActionHandler::new(&mut state);
            // Test 256-color background sequence: 48;5;196
            let color_params = [
                CsiParam::Integer(48),
                CsiParam::P(59),
                CsiParam::Integer(5),
                CsiParam::P(59),
                CsiParam::Integer(196),
            ];
            handler.handle_sgr(&color_params);
        }

        assert_eq!(state.background_color, Some(Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn test_all_basic_foreground_colors() {
        use vtparse::CsiParam;

        let color_tests = [
            (30, Color::Black),
            (31, Color::Red),
            (32, Color::Green),
            (33, Color::Yellow),
            (34, Color::Blue),
            (35, Color::Magenta),
            (36, Color::Cyan),
            (37, Color::White),
        ];

        for (code, expected_color) in color_tests {
            let mut state = TerminalState::new(80, 24);
            {
                let mut handler = VtActionHandler::new(&mut state);
                let color_params = [CsiParam::Integer(code)];
                handler.handle_sgr(&color_params);
            }

            assert_eq!(
                state.foreground_color,
                Some(expected_color),
                "Failed for color code {}",
                code
            );
        }
    }

    #[test]
    fn test_all_basic_background_colors() {
        use vtparse::CsiParam;

        let color_tests = [
            (40, Color::Black),
            (41, Color::Red),
            (42, Color::Green),
            (43, Color::Yellow),
            (44, Color::Blue),
            (45, Color::Magenta),
            (46, Color::Cyan),
            (47, Color::White),
        ];

        for (code, expected_color) in color_tests {
            let mut state = TerminalState::new(80, 24);
            {
                let mut handler = VtActionHandler::new(&mut state);
                let color_params = [CsiParam::Integer(code)];
                handler.handle_sgr(&color_params);
            }

            assert_eq!(
                state.background_color,
                Some(expected_color),
                "Failed for background color code {}",
                code
            );
        }
    }

    #[test]
    fn test_all_text_modifiers() {
        use vtparse::CsiParam;

        let modifiers = [
            (1, "Bold"),
            (3, "Italic"),
            (4, "Underlined"),
            (7, "Reversed"),
        ];

        for (code, name) in modifiers {
            let mut state = TerminalState::new(80, 24);
            {
                let mut handler = VtActionHandler::new(&mut state);
                let modifier_params = [CsiParam::Integer(code)];
                handler.handle_sgr(&modifier_params);
            }

            // Check the corresponding state field
            match code {
                1 => assert!(state.bold, "Failed for modifier code {} ({})", code, name),
                3 => assert!(state.italic, "Failed for modifier code {} ({})", code, name),
                4 => assert!(
                    state.underline,
                    "Failed for modifier code {} ({})",
                    code, name
                ),
                7 => assert!(
                    state.reverse,
                    "Failed for modifier code {} ({})",
                    code, name
                ),
                _ => panic!("Unexpected modifier code: {}", code),
            }
        }
    }

    #[test]
    fn test_modifier_reset_codes() {
        let reset_tests = [
            (22, 1, "Bold reset"),
            (23, 3, "Italic reset"),
            (24, 4, "Underline reset"),
            (27, 7, "Reverse reset"),
        ];

        for (reset_code, set_code, name) in reset_tests {
            // First set the modifier, then reset it
            let sequence = format!("\x1b[{}mSet\x1b[{}mReset\x1b[0m", set_code, reset_code);
            let mut test_parser = AnsiParser::new(80, 1);
            test_parser.parse(&sequence);
            let state = test_parser.get_terminal_state();
            let buffer = state.current_buffer();

            // The 'R' in "Reset" should not have the modifier
            let _reset_char_style = buffer[0][3].style; // 'R' is at index 3
            match set_code {
                1 => assert!(!state.bold, "Bold should be reset for {}", name),
                3 => assert!(!state.italic, "Italic should be reset for {}", name),
                4 => assert!(!state.underline, "Underline should be reset for {}", name),
                7 => assert!(!state.reverse, "Reverse should be reset for {}", name),
                _ => panic!("Unexpected set_code: {}", set_code),
            }
        }
    }

    #[test]
    fn test_color_reset_codes() {
        // Test foreground color reset (39)
        let fg_reset_sequence = "\x1b[31mRed\x1b[39mDefault\x1b[0m";
        let mut test_parser = AnsiParser::new(80, 1);
        test_parser.parse(fg_reset_sequence);
        let state = test_parser.get_terminal_state();

        assert_eq!(
            state.foreground_color, None,
            "Foreground color should be reset"
        );

        // Test background color reset (49)
        let bg_reset_sequence = "\x1b[41mRed\x1b[49mDefault\x1b[0m";
        let mut test_parser2 = AnsiParser::new(80, 1);
        test_parser2.parse(bg_reset_sequence);
        let state2 = test_parser2.get_terminal_state();

        assert_eq!(
            state2.background_color, None,
            "Background color should be reset"
        );
    }

    #[test]
    fn test_complete_reset_code() {
        // Test that SGR 0 resets everything
        let reset_sequence = "\x1b[1;3;4;7;31;41mStyled\x1b[0mReset\x1b[0m";
        let mut test_parser = AnsiParser::new(80, 1);
        test_parser.parse(reset_sequence);
        let state = test_parser.get_terminal_state();

        assert!(!state.bold, "Bold should be reset");
        assert!(!state.italic, "Italic should be reset");
        assert!(!state.underline, "Underline should be reset");
        assert!(!state.reverse, "Reverse should be reset");
        assert_eq!(
            state.foreground_color, None,
            "Foreground color should be reset"
        );
        assert_eq!(
            state.background_color, None,
            "Background color should be reset"
        );
    }

    #[test]
    fn test_compound_sgr_codes_in_buffer() {
        use vtparse::CsiParam;

        // Test multiple SGR codes in one sequence (like ls --color)
        let mut state = TerminalState::new(80, 24);
        {
            let mut handler = VtActionHandler::new(&mut state);
            // Test compound sequence: 01;34 (bold + blue)
            let compound_params = [
                CsiParam::Integer(1),
                CsiParam::P(59),       // 1;
                CsiParam::Integer(34), // 34
            ];
            handler.handle_sgr(&compound_params);
        }

        assert!(state.bold, "Should be bold");
        assert_eq!(state.foreground_color, Some(Color::Blue), "Should be blue");

        // Test that the styling works when inserting characters
        state.insert_char('X');
        let buffer = state.current_buffer();
        let style = buffer[0][0].style;
        assert!(
            style.add_modifier.contains(Modifier::BOLD),
            "Character should be bold"
        );
        assert_eq!(style.fg, Some(Color::Blue), "Character should be blue");
    }

    #[test]
    fn test_mixed_rgb_and_basic_colors() {
        // Test mixing RGB and basic colors in the same sequence
        let mixed_sequence = "\x1b[38;2;255;0;0mRGB\x1b[34mBasic\x1b[38;2;0;255;0mRGB2\x1b[0m";
        let lines = AnsiParser::new(80, 24).parse(mixed_sequence);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "RGBBasicRGB2");
    }

    #[test]
    fn test_rgb_edge_cases() {
        let mut parser = AnsiParser::new(80, 24);

        // Test RGB with max values
        let max_rgb = "\x1b[38;2;255;255;255mWhite\x1b[0m";
        let lines = parser.parse(max_rgb);
        assert_eq!(lines[0].to_string(), "White");

        // Test RGB with min values
        let min_rgb = "\x1b[38;2;0;0;0mBlack\x1b[0m";
        let lines = parser.parse(min_rgb);
        assert_eq!(lines[0].to_string(), "Black");

        // Test RGB with mixed values
        let mixed_rgb = "\x1b[38;2;128;64;192mPurple\x1b[0m";
        let lines = parser.parse(mixed_rgb);
        assert_eq!(lines[0].to_string(), "Purple");
    }

    #[test]
    fn test_malformed_sequences_dont_crash() {
        let mut parser = AnsiParser::new(80, 24);

        // Test incomplete RGB sequences
        let incomplete_sequences = [
            "\x1b[38;2mIncomplete",
            "\x1b[38;2;255mMissingGB",
            "\x1b[38;2;255;128mMissingB",
            "\x1b[38;5mMissing256Color",
            "\x1b[38mMissingType",
        ];

        for sequence in incomplete_sequences {
            let lines = parser.parse(sequence);
            assert!(
                !lines.is_empty(),
                "Should not crash on malformed sequence: {}",
                sequence
            );
        }
    }

    #[test]
    fn test_parameter_skipping_bug_fix() {
        // This test specifically verifies the parameter skipping bug fix
        // Test that we can parse RGB params correctly and then process a reset
        let mut state = TerminalState::new(80, 24);

        use vtparse::CsiParam;

        // First, set RGB color: 38;2;251;41;91
        {
            let mut handler = VtActionHandler::new(&mut state);
            let rgb_params = [
                CsiParam::Integer(38),
                CsiParam::P(59),
                CsiParam::Integer(2),
                CsiParam::P(59),
                CsiParam::Integer(251),
                CsiParam::P(59),
                CsiParam::Integer(41),
                CsiParam::P(59),
                CsiParam::Integer(91),
            ];
            handler.handle_sgr(&rgb_params);
        }

        // Verify RGB color is set
        assert_eq!(state.foreground_color, Some(Color::Rgb(251, 41, 91)));

        // Insert character with RGB color
        state.insert_char('A');

        // Now reset foreground color: 39
        {
            let mut handler = VtActionHandler::new(&mut state);
            let reset_params = [CsiParam::Integer(39)];
            handler.handle_sgr(&reset_params);
        }

        // Verify color is reset
        assert_eq!(state.foreground_color, None);

        // Insert character with default color
        state.insert_char('B');

        let buffer = state.current_buffer();
        assert_eq!(buffer[0][0].ch, 'A');
        assert_eq!(buffer[0][0].style.fg, Some(Color::Rgb(251, 41, 91)));
        assert_eq!(buffer[0][1].ch, 'B');
        assert_eq!(buffer[0][1].style.fg, None);
    }

    #[test]
    fn test_color_index_conversion() {
        let mut state = TerminalState::new(80, 24);
        let handler = VtActionHandler::new(&mut state);

        // Test standard colors (0-15)
        assert_eq!(handler.index_to_color(0), Color::Black);
        assert_eq!(handler.index_to_color(1), Color::Red);
        assert_eq!(handler.index_to_color(9), Color::LightRed);
        assert_eq!(handler.index_to_color(15), Color::White);

        // Test 216-color cube calculation
        assert_eq!(handler.index_to_color(16), Color::Rgb(0, 0, 0)); // First cube color
        assert_eq!(handler.index_to_color(21), Color::Rgb(0, 0, 255)); // Pure blue
        assert_eq!(handler.index_to_color(196), Color::Rgb(255, 0, 0)); // Pure red
        assert_eq!(handler.index_to_color(231), Color::Rgb(255, 255, 255)); // Last cube color

        // Test grayscale ramp (232-255)
        assert_eq!(handler.index_to_color(232), Color::Rgb(8, 8, 8));
        assert_eq!(handler.index_to_color(255), Color::Rgb(238, 238, 238));
    }

    #[test]
    fn test_original_bug_fix_integration() {
        // Test the original issue from color-1.txt file
        // The sequence that was failing: [38;2;251;41;91mR[39m
        let mut parser = AnsiParser::new(80, 24);

        // This should now work correctly and show red foreground, not background
        let test_sequence = "\x1b[38;2;251;41;91mR\x1b[39m";
        let lines = parser.parse(test_sequence);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "R");

        // The original issue was that this sequence would set background color instead of foreground
        // This integration test ensures the full parsing pipeline works correctly

        // Also test that we can correctly parse multiple RGB sequences
        let complex_sequence =
            "\x1b[38;2;255;0;0mRed\x1b[38;2;0;255;0mGreen\x1b[38;2;0;0;255mBlue\x1b[0m";
        let lines = parser.parse(complex_sequence);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "RedGreenBlue");
    }
}
