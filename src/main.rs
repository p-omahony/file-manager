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

#[derive(Default)]
struct App {
    show_popup: bool,
    show_grep_popup: bool,
    input: String,
    cursor_position: usize,
    results: String,
    find_directories: bool,
    waiting_for_d: bool,
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut space_pressed = false;
    
        loop {
            terminal.draw(|frame| self.draw(frame))?;
    
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            if !self.show_popup && !self.show_grep_popup {
                                return Ok(());
                            } else {
                                self.input.push('q');
                                self.cursor_position += 1;
                            }
                        }
                        KeyCode::Char(' ') => {
                            if !self.show_popup && !self.show_grep_popup {
                                space_pressed = true;
                            } else {
                                self.input.push(' ');
                                self.cursor_position += 1;
                            }
                        }
                        KeyCode::Char('f') => {
                            if space_pressed && !self.show_popup && !self.show_grep_popup {
                                self.show_popup = true;
                                self.show_grep_popup = false;
                                self.find_directories = false;
                                self.waiting_for_d = true;
                            } else if self.show_popup || self.show_grep_popup {
                                self.input.push('f');
                                self.cursor_position += 1;
                            }
                            space_pressed = false;
                        }
                        KeyCode::Char('d') => {
                            if self.waiting_for_d {
                                self.find_directories = true;
                                self.waiting_for_d = false;
                            } else if self.show_popup || self.show_grep_popup {
                                self.input.push('d');
                                self.cursor_position += 1;
                            }
                            space_pressed = false;
                        }
                        KeyCode::Char('g') => {
                            if space_pressed && !self.show_popup && !self.show_grep_popup {
                                self.show_grep_popup = true;
                                self.show_popup = false;
                                self.find_directories = false;
                                self.waiting_for_d = false;
                            } else if self.show_popup || self.show_grep_popup {
                                self.input.push('g');
                                self.cursor_position += 1;
                            }
                            space_pressed = false;
                        }
                        KeyCode::Esc => {
                            if self.show_popup || self.show_grep_popup {
                                self.show_popup = false;
                                self.show_grep_popup = false;
                                self.find_directories = false;
                                self.waiting_for_d = false;
                                self.input.clear();
                                self.cursor_position = 0;
                            }
                        }
                        KeyCode::Char(c) => {
                            if self.show_popup || self.show_grep_popup {
                                self.waiting_for_d = false;
                                self.input.push(c);
                                self.cursor_position += 1;
                            }
                        }
                        KeyCode::Backspace => {
                            if (self.show_popup || self.show_grep_popup) && !self.input.is_empty() {
                                self.waiting_for_d = false;
                                self.input.pop();
                                self.cursor_position = self.cursor_position.saturating_sub(1);
                            }
                        }
                        KeyCode::Left => {
                            if self.show_popup || self.show_grep_popup {
                                self.waiting_for_d = false;
                                self.cursor_position = self.cursor_position.saturating_sub(1);
                            }
                        }
                        KeyCode::Right => {
                            if self.show_popup || self.show_grep_popup {
                                self.waiting_for_d = false;
                                self.cursor_position = self.cursor_position.min(self.input.len());
                            }
                        }
                        KeyCode::Enter => {
                            if self.show_popup {
                                let mut command = Command::new("find");
                                command.arg(".");
                                
                                if self.find_directories {
                                    command.arg("-type").arg("d");
                                }
                                
                                if !self.input.is_empty() {
                                    command.arg("-name").arg(format!("*{}*", self.input));
                                }
                                
                                let output = command
                                    .output()
                                    .expect("Failed to execute find command");
                                
                                if output.status.success() {
                                    self.results = String::from_utf8_lossy(&output.stdout).to_string();
                                } else {
                                    self.results = format!("Error: {}", String::from_utf8_lossy(&output.stderr));
                                }
                                
                                self.show_popup = false;
                                self.find_directories = false;
                                self.waiting_for_d = false;
                                self.input.clear();
                                self.cursor_position = 0;
                            } else if self.show_grep_popup {
                                let output = Command::new("grep")
                                    .arg("-rn")
                                    .arg(&self.input)
                                    .arg(".")
                                    .output()
                                    .expect("Failed to execute grep command");
                                
                                if output.status.success() {
                                    self.results = String::from_utf8_lossy(&output.stdout).to_string();
                                } else {
                                    self.results = format!("Error: {}", String::from_utf8_lossy(&output.stderr));
                                }
                                
                                self.show_grep_popup = false;
                                self.input.clear();
                                self.cursor_position = 0;
                            }
                        }
                        _ => {
                            space_pressed = false;
                            self.waiting_for_d = false;
                        }
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
        let paragraph = Paragraph::new(text)
            .block(Block::default())
            .centered();
        frame.render_widget(paragraph, instructions);

        let content_paragraph = Paragraph::new(Text::from(self.results.as_str()))
            .block(Block::bordered().title("Content").on_blue())
            .wrap(Wrap { trim: true });
        frame.render_widget(content_paragraph, content);

        if self.show_popup || self.show_grep_popup {
            let block = Block::bordered();
            let area = popup_area(area, 60);
            frame.render_widget(Clear, area); //this clears out the background
            frame.render_widget(block, area);

            // Create a centered area for the input text
            let chunks = Layout::vertical([
                Constraint::Min(0),  // Add small offset at top
                Constraint::Length(1),
                Constraint::Min(0),
            ]).split(area);

            // Render the input text in the middle chunk
            let popup_type = if self.show_popup {
                if self.find_directories { "Find Directories" } else { "Find Files" }
            } else {
                "Grep"
            };
            let input_text = format!("{}: {}{}", 
                popup_type,
                self.input, 
                if self.cursor_position == self.input.len() { "█" } else { "" }
            );
            let input = Paragraph::new(input_text);
            frame.render_widget(input, chunks[1]);
        }
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, percent_x: u16) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage(10),  // Add small offset at top
        Constraint::Length(3)
    ]);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]);
    let [_, area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}