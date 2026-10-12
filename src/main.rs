use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use prowly::app::App;
use prowly::config::Config;
use prowly::event::EventHandler;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::panic;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut config = Config::load();

    // CLI overrides
    if args.iter().any(|arg| arg == "--no-nerd-font") {
        config.use_nerd_font = false;
    }
    if let Some(lang_pos) = args.iter().position(|arg| arg == "--lang") {
        if let Some(lang_code) = args.get(lang_pos + 1) {
            if let Some(l) = prowly::i18n::Language::from_code(lang_code) {
                config.language = l;
            }
        }
    }
    if let Some(theme_pos) = args.iter().position(|arg| arg == "--theme") {
        if let Some(theme_name) = args.get(theme_pos + 1) {
            if let Some(t) = prowly::ui::ThemeId::from_name(theme_name) {
                config.theme = t;
            }
        }
    }

    // Positional directory path argument: prowly [path]
    let mut skip_next = false;
    for arg in args.iter().skip(1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--lang" || arg == "--theme" {
            skip_next = true;
            continue;
        }
        if !arg.starts_with('-') {
            let p = std::path::PathBuf::from(arg);
            if p.is_dir() {
                let _ = std::env::set_current_dir(&p);
                break;
            }
        }
    }

    // Diagnostic/Information mode without taking over terminal
    if args
        .iter()
        .any(|arg| arg == "--info" || arg == "--version" || arg == "-v")
    {
        let theme = prowly::ui::Theme::get(config.theme);
        let icons = prowly::ui::Icons::new(config.use_nerd_font);
        println!("{} Prowly v0.1.0", icons.branch());
        println!("Theme: {} (dark={})", theme.name, theme.is_dark);
        println!("Language: {}", config.language.display_name());
        println!("Nerd Font: {}", config.use_nerd_font);
        println!("AI Provider: {:?}", config.ai.provider);
        return Ok(());
    }

    // Set panic hook to restore terminal gracefully
    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_panic(panic_info);
    }));

    // Setup terminal with alternate screen and mouse tracking
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut events = EventHandler::new(Duration::from_millis(50));
    let mut app = App::new(config);
    app.set_event_sender(events.sender());
    app.refresh_github();

    // Run TUI event loop
    while !app.should_quit {
        if app.needs_clear {
            terminal.clear()?;
            app.needs_clear = false;
        }
        terminal.draw(|f| app.render(f))?;

        if let Some(event) = events.next().await {
            app.handle_event(event);
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
