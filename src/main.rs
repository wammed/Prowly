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

    // Help option without taking over terminal
    if args
        .iter()
        .any(|arg| arg == "--help" || arg == "-h" || arg == "help")
    {
        print_help(&config);
        return Ok(());
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
    app.trigger_background_fetch();

    // Run TUI event loop
    while !app.should_quit {
        if app.needs_clear {
            terminal.clear()?;
            app.needs_clear = false;
        }
        terminal.draw(|f| app.render(f))?;

        if let Some(event) = events.next().await {
            app.handle_event(event);
            while let Ok(next_event) = events.try_recv() {
                app.handle_event(next_event);
            }
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

fn print_help(config: &Config) {
    let icons = prowly::ui::Icons::new(config.use_nerd_font);
    if config.language == prowly::i18n::Language::Ja {
        println!("{} Prowly v0.1.0", icons.branch());
        println!("全面統合AIコパイロット搭載 ターミナル Git & GitHub クライアント\n");
        println!("使用方法:");
        println!("    prowly [オプション] [リポジトリパス]\n");
        println!("引数:");
        println!("    <PATH>            開く Git リポジトリのディレクトリパス [デフォルト: .]\n");
        println!("オプション:");
        println!("    -h, --help        このヘルプメッセージを表示して終了");
        println!("    -v, --version     バージョン情報および設定サマリーを表示して終了");
        println!("        --info        詳細な診断・設定情報を表示して終了");
        println!("        --theme <NAME>    UIカラーテーマを指定 (例: tokyo_night, catppuccin_mocha, dracula 等)");
        println!("        --lang <CODE>     UI表示言語を指定 (en, ja)");
        println!("        --no-nerd-font    Nerd Font グリフを無効化 (ASCII/Unicode フォールバックを使用)");
    } else {
        println!("{} Prowly v0.1.0", icons.branch());
        println!("Terminal Git & GitHub client with Omnipresent AI\n");
        println!("USAGE:");
        println!("    prowly [OPTIONS] [PATH]\n");
        println!("ARGS:");
        println!("    <PATH>            Path to Git repository directory [default: .]\n");
        println!("OPTIONS:");
        println!("    -h, --help        Print this help message and exit");
        println!("    -v, --version     Print version and configuration summary and exit");
        println!("        --info        Print diagnostic configuration information and exit");
        println!("        --theme <NAME>    Set UI color theme (e.g., tokyo_night, catppuccin_mocha, dracula)");
        println!("        --lang <CODE>     Set UI language (en, ja)");
        println!("        --no-nerd-font    Disable Nerd Font glyphs (use ASCII/Unicode fallback)");
    }
}
