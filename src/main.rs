use color_eyre::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    widgets::{Block, Clear, Paragraph, Wrap},
    text::Text,
    DefaultTerminal, Frame,
};
use std::process::Command;

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::default().run(terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    FindFiles,
    FindDirs,
    Grep,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}

struct App {
    mode: Mode,
    input: String,
    cursor_position: usize,
    results: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            mode: Mode::Normal,
            input: String::new(),
            cursor_position: 0,
            results: String::new(),
        }
    }
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match (self.mode, key.code) {
                        // Quit when in Normal mode
                        (Mode::Normal, KeyCode::Char('q')) => return Ok(()),

                        // Enter FindFiles mode: Space + f
                        (Mode::Normal, KeyCode::Char(' ')) => {
                            // wait for next key...
                            if let Event::Key(next) = event::read()? {
                                if next.code == KeyCode::Char('f') {
                                    self.mode = Mode::FindFiles;
                                } else if next.code == KeyCode::Char('g') {
                                    self.mode = Mode::Grep;
                                }
                            }
                        }

                        // In FindFiles, hitting 'd' switches to FindDirs
                        (Mode::FindFiles, KeyCode::Char('d')) => {
                            self.mode = Mode::FindDirs;
                        }

                        // Any other char in a popup just goes into input
                        (Mode::FindFiles | Mode::FindDirs | Mode::Grep, KeyCode::Char(c)) => {
                            self.input.insert(self.cursor_position, c);
                            self.cursor_position += 1;
                        }
                        (Mode::FindFiles | Mode::FindDirs | Mode::Grep, KeyCode::Backspace) => {
                            if self.cursor_position > 0 {
                                self.cursor_position -= 1;
                                self.input.remove(self.cursor_position);
                            }
                        }
                        (Mode::FindFiles | Mode::FindDirs | Mode::Grep, KeyCode::Left) => {
                            self.cursor_position = self.cursor_position.saturating_sub(1);
                        }
                        (Mode::FindFiles | Mode::FindDirs | Mode::Grep, KeyCode::Right) => {
                            self.cursor_position = self.cursor_position.min(self.input.len());
                        }
                        (Mode::FindFiles | Mode::FindDirs, KeyCode::Enter) => {
                            // run `find`:
                            let mut cmd = Command::new("find");
                            cmd.arg(".");
                            if self.mode == Mode::FindDirs {
                                cmd.arg("-type").arg("d");
                            }
                            if !self.input.is_empty() {
                                cmd.arg("-name").arg(format!("*{}*", self.input));
                            }
                            let output = cmd.output().expect("find failed");
                            self.results = if output.status.success() {
                                let paths = String::from_utf8_lossy(&output.stdout);
                                let mut results = Vec::new();
                                
                                for path in paths.lines() {
                                    let path = path.trim();
                                    if path.is_empty() {
                                        continue;
                                    }
                                    
                                    let size_output = if self.mode == Mode::FindDirs {
                                        Command::new("du")
                                            .arg("-sh")
                                            .arg(path)
                                            .output()
                                    } else {
                                        Command::new("ls")
                                            .arg("-lh")
                                            .arg(path)
                                            .output()
                                    };
                                    
                                    match size_output {
                                        Ok(size_output) if size_output.status.success() => {
                                            let output_str = String::from_utf8_lossy(&size_output.stdout);
                                            let size_str = output_str
                                                .trim()
                                                .split_whitespace()
                                                .next()
                                                .unwrap_or("?");
                                            results.push(format!("{} {}", size_str, path));
                                        }
                                        _ => {
                                            results.push(path.to_string());
                                        }
                                    }
                                }
                                
                                results.join("\n")
                            } else {
                                format!("Error: {}", String::from_utf8_lossy(&output.stderr))
                            };
                            self.mode = Mode::Normal;
                            self.input.clear();
                            self.cursor_position = 0;
                        }
                        (Mode::Grep, KeyCode::Enter) => {
                            // run `grep`:
                            let output = Command::new("grep")
                                .arg("-rn")
                                .arg("--binary-files=without-match")
                                .arg(format!(".*{}.*", self.input))
                                .arg(".")
                                .output()
                                .expect("grep failed");
                            self.results = if output.status.success() {
                                String::from_utf8_lossy(&output.stdout).to_string()
                            } else {
                                format!("Error: {}", String::from_utf8_lossy(&output.stderr))
                            };
                            self.mode = Mode::Normal;
                            self.input.clear();
                            self.cursor_position = 0;
                        }

                        // Esc in a popup returns to Normal
                        (Mode::FindFiles, KeyCode::Esc)
                        | (Mode::FindDirs, KeyCode::Esc)
                        | (Mode::Grep, KeyCode::Esc) => {
                            self.mode = Mode::Normal;
                            self.input.clear();
                            self.cursor_position = 0;
                        }

                        // Any other key in normal mode is ignored
                        _ => {}
                    }
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let vertical = Layout::vertical([Constraint::Percentage(20), Constraint::Percentage(80)]);
        let [instructions, content] = vertical.areas(area);

        let text = "> Press Space-f(-d) for find files(-directories)\n\
                    > Press Space-g for grep command";
        frame.render_widget(
            Paragraph::new(text).block(Block::default()).centered(),
            instructions,
        );

        frame.render_widget(
            Paragraph::new(Text::from(self.results.as_str()))
                .block(Block::bordered().title("Content").on_blue())
                .wrap(Wrap { trim: true }),
            content,
        );

        if let Mode::FindFiles | Mode::FindDirs | Mode::Grep = self.mode {
            let area = popup_area(area, 60);
            frame.render_widget(Clear, area);
            frame.render_widget(Block::bordered(), area);

            let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1), Constraint::Min(0)])
                .split(area);
            let popup_type = match self.mode {
                Mode::FindFiles => "Find Files",
                Mode::FindDirs  => "Find Directories",
                Mode::Grep      => "Grep",
                _ => unreachable!(),
            };
            let input_text = format!(
                "{}: {}{}",
                popup_type,
                self.input,
                if self.cursor_position == self.input.len() { "█" } else { "" }
            );
            frame.render_widget(Paragraph::new(input_text), chunks[1]);
        }
    }
}

// unchanged helper
fn popup_area(area: Rect, percent_x: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(10), Constraint::Length(3)]);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]);
    let [_, area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}