// ============================================================================
// This is a RATATUI Application written in Rust.
// 🍅 RETRO POMODORO TIMER - A cool terminal Productivity Tool [PT].
// Version 1.3 - I had to rewrite various parts of the program. 
// This is the last iteration 1.3
// ============================================================================
// 
// This application demonstrates the beauty and power of Rust programming using RATATUI:
// - Memory safety with zero-cost abstractions
// - Expressive type system with pattern matching
// - Modern terminal UI development
// - Clean, modular architecture
// ============================================================================

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use chrono::Local;

// ============================================================================
// ENUMS AND DATA STRUCTURES
// ============================================================================

/// TimerState represents all possible states of our Pomodoro timer.
/// 
/// Rust enums are powerful - they can hold data and provide type safety.
/// The #[derive] macro automatically implements common traits:
/// - Debug: for printing and debugging
/// - Clone: for creating copies
/// - Copy: for efficient copying (no heap allocation)
/// - PartialEq: for comparing states (self.state != TimerState::Paused)
#[derive(Debug, Clone, Copy, PartialEq)]
enum TimerState {
    Work,       // Active work session (25 minutes)
    ShortBreak, // Short break after work (5 minutes)
    LongBreak,  // Long break after 4 sessions (15 minutes)
    Paused,     // Timer is paused
}

/// Main application state struct.
/// 
/// This struct holds all the data needed for our timer application.
/// Rust's struct provides a clean way to group related data together.
struct App {
    state: TimerState,              // Current timer state
    work_duration: Duration,         // Length of work sessions
    short_break_duration: Duration,  // Length of short breaks
    long_break_duration: Duration,   // Length of long breaks
    current_duration: Duration,       // Length of current session
    elapsed: Duration,              // Time elapsed in current session
    sessions_completed: u32,         // Number of completed work sessions
    animation_frame: usize,          // Current animation frame (0-3)
    last_update: Instant,           // Last time we updated the timer
}

// ============================================================================
// APP IMPLEMENTATION
// ============================================================================

impl App {
    /// Creates a new App instance with default Pomodoro settings.
    /// 
    /// This is Rust's constructor pattern - we create a "new" function
    /// that returns a fully initialized instance of our struct.
    fn new() -> Self {
        Self {
            state: TimerState::Work,
            // Standard Pomodoro intervals
            work_duration: Duration::from_secs(25 * 60), // 25 minutes
            short_break_duration: Duration::from_secs(5 * 60), // 5 minutes
            long_break_duration: Duration::from_secs(15 * 60), // 15 minutes
            current_duration: Duration::from_secs(25 * 60),
            elapsed: Duration::from_secs(0),
            sessions_completed: 0,
            animation_frame: 0,
            last_update: Instant::now(),
        }
    }

    /// Updates the timer state and animations.
    /// 
    /// This method is called every frame to:
    /// 1. Update elapsed time if not paused
    /// 2. Update animation frames
    /// 3. Check if session is complete
    /// 
    /// Rust's &mut self allows us to modify the struct safely.
    fn update(&mut self) {
        // Only update time if we're not paused
        if self.state != TimerState::Paused {
            let now = Instant::now();
            let delta = now - self.last_update;
            self.last_update = now;
            self.elapsed += delta;
            
            // Animate through 4 different emoji frames
            self.animation_frame = (self.animation_frame + 1) % 4;

            // Check if current session is complete
            if self.elapsed >= self.current_duration {
                self.next_session();
            }
        }
    }

    /// Transitions to the next session based on Pomodoro rules.
    /// 
    /// This demonstrates Rust's powerful pattern matching:
    /// - Work → Short Break (or Long Break after 4 sessions)
    /// - Short/Long Break → Work
    /// - Paused → no change
    fn next_session(&mut self) {
        match self.state {
            TimerState::Work => {
                // Complete a work session
                self.sessions_completed += 1;
                
                // Every 4th session gets a long break
                if self.sessions_completed % 4 == 0 {
                    self.state = TimerState::LongBreak;
                    self.current_duration = self.long_break_duration;
                } else {
                    self.state = TimerState::ShortBreak;
                    self.current_duration = self.short_break_duration;
                }
            }
            TimerState::ShortBreak | TimerState::LongBreak => {
                // After any break, go back to work
                self.state = TimerState::Work;
                self.current_duration = self.work_duration;
            }
            TimerState::Paused => {
                // Don't transition if paused
            }
        }
        
        // Reset elapsed time for new session
        self.elapsed = Duration::from_secs(0);
    }

    /// Toggles between paused and running states.
    /// 
    /// Demonstrates state mutation and conditional logic.
    fn toggle_pause(&mut self) {
        match self.state {
            TimerState::Paused => {
                // Resume: go back to work state
                self.state = TimerState::Work;
                self.last_update = Instant::now(); // Reset timing
            }
            _ => {
                // Pause any non-paused state
                self.state = TimerState::Paused;
            }
        }
    }

    /// Resets the timer to initial state.
    /// 
    /// Simple method to restart the entire Pomodoro cycle.
    fn reset(&mut self) {
        self.state = TimerState::Work;
        self.current_duration = self.work_duration;
        self.elapsed = Duration::from_secs(0);
        self.sessions_completed = 0;
        self.last_update = Instant::now();
    }

    /// Calculates progress as a percentage (0.0 to 1.0).
    /// 
    /// Returns a floating-point value that can be used for progress bars.
    /// The .min(1.0) ensures we never exceed 100%.
    fn progress(&self) -> f32 {
        if self.current_duration.as_secs() == 0 {
            0.0
        } else {
            (self.elapsed.as_secs() as f32 / self.current_duration.as_secs() as f32).min(1.0)
        }
    }

    /// Calculates remaining time in current session.
    /// 
    /// Uses saturating_sub to prevent negative durations.
    fn remaining(&self) -> Duration {
        self.current_duration.saturating_sub(self.elapsed)
    }
}

// ============================================================================
// UI RENDERING FUNCTIONS
// ============================================================================

/// Renders the main animated timer display.
/// 
/// This function creates the visual centerpiece of our application:
/// - Large countdown timer with animated emojis
/// - Color-coded based on current state
/// - Styled borders and titles
fn draw_animated_timer(f: &mut Frame, area: Rect, app: &App) {
    let remaining = app.remaining();
    let minutes = remaining.as_secs() / 60;
    let seconds = remaining.as_secs() % 60;
    
    // Animated emojis that cycle every frame
    let animation_chars = ["⚡", "🔥", "✨", "💫"];
    let anim_char = animation_chars[app.animation_frame];
    
    // Format: "⚡ 24:54 ⚡"
    let timer_text = format!(
        "{} {}:{:02} {}",
        anim_char,
        minutes,
        seconds,
        anim_char
    );

    // Choose colors and text based on current state
    // This demonstrates Rust's pattern matching for data transformation
    let (state_color, state_text) = match app.state {
        TimerState::Work => (Color::Red, "🍅 WORK TIME"),
        TimerState::ShortBreak => (Color::Green, "☕ SHORT BREAK"),
        TimerState::LongBreak => (Color::Blue, "🌟 LONG BREAK"),
        TimerState::Paused => (Color::Yellow, "⏸️ PAUSED"),
    };

    // Create the timer widget with styling
    let timer_widget = Paragraph::new(timer_text)
        .style(Style::default()
            .fg(state_color)
            .add_modifier(Modifier::BOLD)) // Make text bold
        .block(Block::default()
            .title(state_text)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(state_color))
            .title_style(Style::default().fg(state_color).add_modifier(Modifier::BOLD)));

    f.render_widget(timer_widget, area);
}

/// Renders a pixel-style progress bar.
/// 
/// Creates a retro progress bar using block characters:
/// - █ for filled portions
/// - ░ for empty portions
/// 
/// This demonstrates string manipulation and visual design in code.
fn draw_pixel_progress(f: &mut Frame, area: Rect, progress: f32, label: &str) {
    // Calculate how many blocks to fill (out of 20 total)
    let filled_blocks = (progress * 20.0) as usize;
    let empty_blocks = 20 - filled_blocks;
    
    // Create the visual progress bar string
    let progress_bar = "█".repeat(filled_blocks) + &"░".repeat(empty_blocks);
    
    // Use ratatui's Gauge widget for the progress bar
    let gauge = Gauge::default()
        .block(Block::default()
            .title(label)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)))
        .gauge_style(Style::default()
            .fg(Color::Green)    // Filled portion color
            .bg(Color::Black))   // Background color
        .percent((progress * 100.0) as u16)
        .label(format!("{} {:.0}%", progress_bar, progress * 100.0));

    f.render_widget(gauge, area);
}

/// Renders the statistics panel.
/// 
/// Shows:
/// - Number of completed sessions
/// - Current state description
/// - Current system time
fn draw_stats(f: &mut Frame, area: Rect, app: &App) {
    // Create styled text lines using Span for different colors
    let stats_text = vec![
        Line::from(vec![
            Span::styled("Sessions: ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{}", app.sessions_completed), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("State: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                match app.state {
                    TimerState::Work => "Working",
                    TimerState::ShortBreak => "Short Break",
                    TimerState::LongBreak => "Long Break",
                    TimerState::Paused => "Paused",
                },
                Style::default().fg(Color::White)
            ),
        ]),
        Line::from(vec![
            Span::styled("Time: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                Local::now().format("%H:%M:%S").to_string(),
                Style::default().fg(Color::Yellow)
            ),
        ]),
    ];

    let stats_widget = Paragraph::new(stats_text)
        .block(Block::default()
            .title("📊 STATS")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta)));

    f.render_widget(stats_widget, area);
}

/// Renders the help/controls panel.
/// 
/// Shows keyboard controls and Pomodoro technique information.
fn draw_controls(f: &mut Frame, area: Rect) {
    let controls_text = vec![
        Line::from("🎮 CONTROLS:"),
        Line::from(""),
        Line::from("Space - Pause/Resume"),
        Line::from("r     - Reset Timer"),
        Line::from("q     - Quit"),
        Line::from(""),
        Line::from("🍅 Pomodoro Technique:"),
        Line::from("25min work → 5min break"),
        Line::from("After 4 sessions → 15min break"),
    ];

    let controls_widget = Paragraph::new(controls_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default()
            .title("ℹ️ HELP")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)));

    f.render_widget(controls_widget, area);
}

/// Main rendering function that orchestrates all UI components.
/// 
/// Uses ratatui's Layout system to divide the screen into sections:
/// - Timer display (top)
/// - Progress bar
/// - Statistics
/// - Help/controls (bottom)
fn render_app(f: &mut Frame, app: &App) {
    // Create vertical layout with fixed heights
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Timer section
            Constraint::Length(8),  // Progress section
            Constraint::Length(8),  // Stats section
            Constraint::Min(0),     // Controls section (takes remaining space)
        ])
        .split(f.area());

    // Render each section in its designated area
    draw_animated_timer(f, chunks[0], app);
    draw_pixel_progress(f, chunks[1], app.progress(), "⏰ PROGRESS");
    draw_stats(f, chunks[2], app);
    draw_controls(f, chunks[3]);
}

// ============================================================================
// MAIN APPLICATION LOOP
// ============================================================================

/// Main entry point of the application.
/// 
/// This function:
/// 1. Sets up the terminal for TUI rendering
/// 2. Creates the application state
/// 3. Runs the main event loop
/// 4. Cleans up terminal state on exit
/// 
/// Returns Result for proper error handling - a Rust best practice.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ============================================================================
    // TERMINAL SETUP
    // ============================================================================
    
    // Enable raw mode for direct keyboard input
    enable_raw_mode()?;
    
    // Set up terminal for full-screen TUI
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
    // Create the terminal backend
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ============================================================================
    // APPLICATION INITIALIZATION
    // ============================================================================
    
    // Create our app instance with default Pomodoro settings
    let mut app = App::new();
    
    // Update every 100ms for smooth animations
    let tick_rate = Duration::from_millis(100);

    // ============================================================================
    // MAIN EVENT LOOP
    // ============================================================================
    
    loop {
        // Update application state (timer, animations)
        app.update();

        // Render the UI
        terminal.draw(|f| render_app(f, &app))?;

        // Handle keyboard input events
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                // Only respond to key press events, not release
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,           // Quit application
                        KeyCode::Char(' ') => app.toggle_pause(), // Toggle pause
                        KeyCode::Char('r') => app.reset(),      // Reset timer
                        _ => {} // Ignore other keys
                    }
                }
            }
        }
    }

    // ============================================================================
    // CLEANUP
    // ============================================================================
    
    // Restore terminal to original state
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
