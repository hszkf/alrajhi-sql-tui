//! Layout management

use crate::app::{App, ActivePanel, SPINNER_FRAMES};
use crate::ui::{AlrajhiTheme, draw_query_editor, draw_results_table, draw_schema_explorer, draw_history_panel};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Clear};

/// Draw the main layout
pub fn draw_layout(f: &mut Frame, app: &mut App, area: Rect) {
    // Main vertical layout: header, content, status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Header
            Constraint::Min(10),     // Content
            Constraint::Length(1),   // Status bar
        ])
        .split(area);

    // Draw header
    draw_header(f, app, chunks[0]);

    // Draw main content (horizontal split)
    draw_content(f, app, chunks[1]);

    // Draw status bar
    draw_status_bar(f, app, chunks[2]);
}

/// Draw the header with Alrajhi Bank branding
fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(40),  // Logo/title
            Constraint::Min(20),     // Connection info
            Constraint::Length(25),  // Quick hints
        ])
        .split(area);

    // Logo/Title
    let logo = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("╔═══════════════════════════════════╗", AlrajhiTheme::title()),
        ]),
        Line::from(vec![
            Span::styled("║ ", AlrajhiTheme::title()),
            Span::styled("🏦 ALRAJHI BANK ", Style::default().fg(AlrajhiTheme::GOLD).add_modifier(Modifier::BOLD)),
            Span::styled("SQL Studio ", Style::default().fg(AlrajhiTheme::TEXT)),
            Span::styled("║", AlrajhiTheme::title()),
        ]),
        Line::from(vec![
            Span::styled("╚═══════════════════════════════════╝", AlrajhiTheme::title()),
        ]),
    ])
    .style(AlrajhiTheme::header());
    f.render_widget(logo, header_chunks[0]);

    // Connection info
    let conn_info = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("● ", AlrajhiTheme::success()),
            Span::styled(&app.db.config.database, AlrajhiTheme::normal_text()),
            Span::styled(" @ ", AlrajhiTheme::dim_text()),
            Span::styled(&app.db.config.host, AlrajhiTheme::dim_text()),
        ]),
        Line::from(""),
    ])
    .style(AlrajhiTheme::header());
    f.render_widget(conn_info, header_chunks[1]);

    // Quick hints (instead of mode indicator)
    let hints = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", AlrajhiTheme::info()),
            Span::styled(":Run ", AlrajhiTheme::dim_text()),
            Span::styled("F1", AlrajhiTheme::info()),
            Span::styled(":Help ", AlrajhiTheme::dim_text()),
        ]),
        Line::from(""),
    ])
    .style(AlrajhiTheme::header())
    .alignment(Alignment::Right);
    f.render_widget(hints, header_chunks[2]);
}

/// Draw main content area
fn draw_content(f: &mut Frame, app: &mut App, area: Rect) {
    // Horizontal split: left (query + results), right (schema + history)
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),  // Main area
            Constraint::Percentage(30),  // Side panels
        ])
        .split(area);

    // Left side: Query editor + Results
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),  // Query editor
            Constraint::Percentage(65),  // Results
        ])
        .split(h_chunks[0]);

    // Right side: Schema explorer + History
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),  // Schema explorer
            Constraint::Percentage(40),  // History
        ])
        .split(h_chunks[1]);

    // Draw each panel - query editor needs mutable access for scroll updates
    let is_query_active = app.active_panel == ActivePanel::QueryEditor;
    let is_results_active = app.active_panel == ActivePanel::Results;
    let is_schema_active = app.active_panel == ActivePanel::SchemaExplorer;
    let is_history_active = app.active_panel == ActivePanel::History;

    draw_query_editor(f, app, left_chunks[0], is_query_active);
    draw_results_table(f, app, left_chunks[1], is_results_active);
    draw_schema_explorer(f, app, right_chunks[0], is_schema_active);
    draw_history_panel(f, app, right_chunks[1], is_history_active);
}

/// Draw the status bar
fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),      // Messages
            Constraint::Length(50),   // Status info
            Constraint::Length(40),   // Keyboard hints
        ])
        .split(area);

    // Messages (error or success)
    let message = if let Some(ref err) = app.error {
        Paragraph::new(Span::styled(
            format!("❌ {}", err),
            AlrajhiTheme::error(),
        ))
    } else if let Some(ref msg) = app.message {
        Paragraph::new(Span::styled(
            format!("✓ {}", msg),
            AlrajhiTheme::success(),
        ))
    } else if app.is_loading {
        let spinner = SPINNER_FRAMES[app.spinner_frame];
        Paragraph::new(Span::styled(
            format!("{} Executing query...", spinner),
            AlrajhiTheme::warning(),
        ))
    } else {
        Paragraph::new(Span::styled("Type query, press Enter to run", AlrajhiTheme::dim_text()))
    };

    f.render_widget(message.style(AlrajhiTheme::status_bar()), chunks[0]);

    // Status info
    let status_info = format!(
        " {} | Rows: {} | History: {} ",
        app.status,
        app.result.row_count,
        app.history.len()
    );
    let status = Paragraph::new(status_info)
        .style(AlrajhiTheme::status_bar())
        .alignment(Alignment::Center);
    f.render_widget(status, chunks[1]);

    // Simplified keyboard hints
    let hints = "Enter:Run  Shift+Enter:Newline  Ctrl+F:Format  Tab:Indent";
    let hints_widget = Paragraph::new(hints)
        .style(AlrajhiTheme::status_bar())
        .alignment(Alignment::Right);
    f.render_widget(hints_widget, chunks[2]);
}

/// Draw help popup
pub fn draw_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 60, area);

    // Clear the area
    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled("🏦 ALRAJHI SQL STUDIO - KEYBOARD SHORTCUTS", AlrajhiTheme::title())),
        Line::from(""),
        Line::from(Span::styled("═══ QUERY EDITOR ═══", AlrajhiTheme::info())),
        Line::from("  Enter           Run query"),
        Line::from("  Shift+Enter     New line in query"),
        Line::from("  Tab             Insert indentation (4 spaces)"),
        Line::from("  Ctrl+F          Format SQL (beautify)"),
        Line::from("  F5              Run query"),
        Line::from("  Esc             Clear query"),
        Line::from("  ←/→/↑/↓         Move cursor"),
        Line::from("  Home/End        Jump to start/end"),
        Line::from(""),
        Line::from(Span::styled("═══ RESULTS TABLE ═══", AlrajhiTheme::info())),
        Line::from("  ↑/↓ or j/k      Navigate rows"),
        Line::from("  ←/→ or h/l      Navigate columns"),
        Line::from("  PageUp/Down     Fast scroll (20 rows)"),
        Line::from("  Home/End        First/Last row"),
        Line::from("  Ctrl+Y          Copy cell value"),
        Line::from("  Ctrl+E          Export to CSV"),
        Line::from("  Ctrl+S          Export to JSON"),
        Line::from("  Ctrl+I          Copy row as INSERT"),
        Line::from("  Enter/Esc       Back to query"),
        Line::from(""),
        Line::from(Span::styled("═══ PANELS ═══", AlrajhiTheme::info())),
        Line::from("  Ctrl+Tab        Next panel"),
        Line::from("  Shift+Tab       Previous panel"),
        Line::from("  Schema: Enter   Expand/Insert table"),
        Line::from("  History: Enter  Load query"),
        Line::from(""),
        Line::from(Span::styled("═══ GLOBAL ═══", AlrajhiTheme::info())),
        Line::from("  Ctrl+Q          Quit application"),
        Line::from("  F1              Toggle this help"),
        Line::from(""),
        Line::from(Span::styled("Press Esc or F1 to close", AlrajhiTheme::dim_text())),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(AlrajhiTheme::popup_border())
                .title(Span::styled(" Help ", AlrajhiTheme::title()))
                .style(AlrajhiTheme::popup()),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(help, popup_area);
}

/// Helper to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
