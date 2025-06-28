pub struct App {
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self { should_quit: false }
    }

    pub fn on_key(&mut self, key: char) {
        if key == 'q' {
            self.should_quit = true;
        }
    }
}
