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
    let title_content = Paragraph::new("↑↓: navigate | Enter: connect | i: edit | n: new | q: quit")
        .block(title);
    f.render_widget(title_content, chunks[0]);

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
    f.render_widget(list, chunks[1]);

    let footer_text = if let Some(status) = &app.status {
        status.clone()
    } else {
        "Ready".to_string()
    };

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL).title(" Status "));
    f.render_widget(footer, chunks[2]);
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
