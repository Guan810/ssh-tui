use crate::app::{App, AppState, FormField};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    match app.state {
        AppState::Normal => draw_normal(f, app),
        AppState::Edit | AppState::New => draw_form(f, app),
    }
}

fn draw_normal(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Block::default()
        .borders(Borders::ALL)
        .title(" SSH TUI ");
    let title_content = Paragraph::new("↑↓/jk: navigate | Enter: connect | i: edit | n: new | q/Esc: quit")
        .block(title);
    f.render_widget(title_content, chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    let items: Vec<ListItem> = app
        .hosts
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let style = if i == app.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let display = format!("{} ({})", entry.host, entry.hostname);
            ListItem::new(display).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Hosts "))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(list, main_chunks[0]);

    draw_details_pane(f, app, main_chunks[1]);

    let footer_text = if let Some(status) = &app.status {
        status.clone()
    } else {
        "Ready".to_string()
    };

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL).title(" Status "));
    f.render_widget(footer, chunks[2]);
}

fn draw_details_pane(f: &mut Frame, app: &App, area: Rect) {
    let details_block = Block::default()
        .borders(Borders::ALL)
        .title(" Details ");

    if let Some(entry) = app.selected_host() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Host: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(&entry.host),
            ]),
            Line::from(vec![
                Span::styled("HostName: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(&entry.hostname),
            ]),
        ];

        if !entry.user.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("User: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(&entry.user),
            ]));
        }

        if !entry.port.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Port: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(&entry.port),
            ]));
        }

        if !entry.identity_file.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("IdentityFile: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(&entry.identity_file),
            ]));
        }

        if !entry.extra.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Additional Config:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]));
            for extra_line in &entry.extra {
                if !extra_line.trim().is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", extra_line),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
        }

        let details = Paragraph::new(lines)
            .block(details_block);
        f.render_widget(details, area);
    } else {
        let empty_text = Paragraph::new("No host selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(details_block);
        f.render_widget(empty_text, area);
    }
}

fn draw_form(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title_text = match app.state {
        AppState::Edit => "Edit Host",
        AppState::New => "New Host",
        AppState::Normal => "Form",
    };

    let title = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title_text));
    let help_text = "Tab/Shift+Tab: navigate | Enter: save | Esc: cancel";
    let help = Paragraph::new(help_text).block(title);
    f.render_widget(help, chunks[0]);

    let form_area = chunks[1];
    draw_form_fields(f, app, form_area);

    let footer_text = if let Some(error) = &app.form_error {
        error.clone()
    } else {
        "Fill in the form and press Enter to save".to_string()
    };

    let footer_style = if app.form_error.is_some() {
        Style::default().fg(Color::Red)
    } else {
        Style::default()
    };

    let footer = Paragraph::new(footer_text)
        .style(footer_style)
        .block(Block::default().borders(Borders::ALL).title(" Status "));
    f.render_widget(footer, chunks[2]);
}

fn draw_form_fields(f: &mut Frame, app: &App, area: Rect) {
    let field_constraints = vec![
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(field_constraints)
        .margin(1)
        .split(area);

    draw_field(
        f,
        "Host (alias)",
        &app.form_entry.host,
        chunks[0],
        app.form_field == FormField::Host,
    );
    draw_field(
        f,
        "HostName (address)",
        &app.form_entry.hostname,
        chunks[1],
        app.form_field == FormField::HostName,
    );
    draw_field(
        f,
        "User",
        &app.form_entry.user,
        chunks[2],
        app.form_field == FormField::User,
    );
    draw_field(
        f,
        "Port",
        &app.form_entry.port,
        chunks[3],
        app.form_field == FormField::Port,
    );
    draw_field(
        f,
        "IdentityFile",
        &app.form_entry.identity_file,
        chunks[4],
        app.form_field == FormField::IdentityFile,
    );
}

fn draw_field(f: &mut Frame, label: &str, value: &str, area: Rect, focused: bool) {
    let style = if focused {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let border_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let full_label = format!(" {} ", label);

    let display_value = if value.is_empty() && !focused {
        Span::styled("<empty>", Style::default().fg(Color::DarkGray))
    } else if focused {
        Span::styled(format!("{}_", value), style)
    } else {
        Span::styled(value, style)
    };

    let content = Line::from(display_value);
    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(full_label),
        );

    f.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HostEntry;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn test_host(name: &str, hostname: &str) -> HostEntry {
        HostEntry {
            host: name.to_string(),
            hostname: hostname.to_string(),
            user: "testuser".to_string(),
            port: "22".to_string(),
            identity_file: "~/.ssh/id_rsa".to_string(),
            extra: vec![],
        }
    }

    #[test]
    fn test_draw_normal_mode() {
        let app = App::test_with_hosts(vec![
            test_host("server1", "192.168.1.1"),
            test_host("server2", "192.168.1.2"),
        ]);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| draw(f, &app))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer.content();

        let text: String = content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("SSH TUI"));
        assert!(text.contains("Hosts"));
        assert!(text.contains("Details"));
        assert!(text.contains("Status"));
        assert!(text.contains("server1"));
        assert!(text.contains("server2"));
    }

    #[test]
    fn test_draw_shows_selected_host_details() {
        let host = HostEntry {
            host: "myserver".to_string(),
            hostname: "example.com".to_string(),
            user: "admin".to_string(),
            port: "2222".to_string(),
            identity_file: "~/.ssh/custom_key".to_string(),
            extra: vec!["  ProxyCommand ssh jump".to_string()],
        };

        let app = App::test_with_hosts(vec![host]);

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| draw(f, &app))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(text.contains("myserver"));
        assert!(text.contains("example.com"));
        assert!(text.contains("admin"));
        assert!(text.contains("2222"));
        assert!(text.contains("custom_key"));
        assert!(text.contains("ProxyCommand"));
    }

    #[test]
    fn test_draw_empty_host_list() {
        let app = App::test_with_hosts(vec![]);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| draw(f, &app))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(text.contains("SSH TUI"));
        assert!(text.contains("Hosts"));
        assert!(text.contains("No host selected"));
    }

    #[test]
    fn test_draw_form_edit_mode() {
        let mut app = App::test_with_hosts(vec![test_host("server1", "192.168.1.1")]);
        app.enter_edit_mode();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| draw(f, &app))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(text.contains("Edit Host"));
        assert!(text.contains("Host (alias)"));
        assert!(text.contains("HostName (address)"));
        assert!(text.contains("User"));
        assert!(text.contains("Port"));
        assert!(text.contains("IdentityFile"));
    }

    #[test]
    fn test_draw_form_new_mode() {
        let mut app = App::test_with_hosts(vec![]);
        app.enter_new_mode();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| draw(f, &app))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(text.contains("New Host"));
        assert!(text.contains("Fill in the form"));
    }

    #[test]
    fn test_draw_with_status_message() {
        let mut app = App::test_with_hosts(vec![test_host("server1", "192.168.1.1")]);
        app.status = Some("Connected successfully".to_string());

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| draw(f, &app))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(text.contains("Connected successfully"));
    }

    #[test]
    fn test_draw_form_with_error() {
        let mut app = App::test_with_hosts(vec![]);
        app.enter_new_mode();
        app.form_error = Some("Host cannot be empty".to_string());

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| draw(f, &app))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(text.contains("Host cannot be empty"));
    }

    #[test]
    fn test_draw_key_hints_visible() {
        let app = App::test_with_hosts(vec![test_host("server1", "192.168.1.1")]);

        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| draw(f, &app))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(text.contains("navigate"));
        assert!(text.contains("connect"));
        assert!(text.contains("edit"));
        assert!(text.contains("quit"));
    }

    #[test]
    fn test_details_pane_shows_optional_fields() {
        let host_with_all_fields = HostEntry {
            host: "full".to_string(),
            hostname: "example.com".to_string(),
            user: "admin".to_string(),
            port: "2222".to_string(),
            identity_file: "~/.ssh/id_rsa".to_string(),
            extra: vec!["  ServerAliveInterval 60".to_string()],
        };

        let host_minimal = HostEntry {
            host: "minimal".to_string(),
            hostname: "example.org".to_string(),
            user: String::new(),
            port: String::new(),
            identity_file: String::new(),
            extra: vec![],
        };

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let app = App::test_with_hosts(vec![host_with_all_fields.clone()]);
        terminal.draw(|f| draw(f, &app)).unwrap();
        let buffer_full = terminal.backend().buffer().clone();
        let text_full: String = buffer_full.content().iter().map(|cell| cell.symbol()).collect();

        assert!(text_full.contains("admin"));
        assert!(text_full.contains("2222"));
        assert!(text_full.contains("id_rsa"));
        assert!(text_full.contains("ServerAliveInterval"));

        let app_minimal = App::test_with_hosts(vec![host_minimal]);
        terminal.draw(|f| draw(f, &app_minimal)).unwrap();
        let buffer_minimal = terminal.backend().buffer().clone();
        let text_minimal: String = buffer_minimal.content().iter().map(|cell| cell.symbol()).collect();

        assert!(text_minimal.contains("minimal"));
        assert!(text_minimal.contains("example.org"));
    }
}
