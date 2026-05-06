use chrono::{Datelike, Duration, Local, NaiveDate, Utc};
use colored::*;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::Print,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

/// GPS epoch: January 6, 1980
fn gps_epoch() -> NaiveDate {
    NaiveDate::from_ymd_opt(1980, 1, 6).unwrap()
}

/// Returns (gps_week, day_of_week) where day_of_week is 0=Sun..6=Sat
fn gps_week_day(date: NaiveDate) -> (i64, u32) {
    let days_since_epoch = (date - gps_epoch()).num_days();
    let gps_week = days_since_epoch / 7;
    let dow = days_since_epoch.rem_euclid(7) as u32;
    (gps_week, dow)
}

/// Get the Sunday that starts a given GPS week
fn gps_week_start(week: i64) -> NaiveDate {
    gps_epoch() + Duration::days(week * 7)
}

struct MonthCalendar {
    year: i32,
    month: u32,
    weeks: Vec<(i64, [Option<NaiveDate>; 7])>,
}

impl MonthCalendar {
    fn new(year: i32, month: u32) -> Self {
        let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let last = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - Duration::days(1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - Duration::days(1)
        };

        let mut weeks: Vec<(i64, [Option<NaiveDate>; 7])> = Vec::new();
        let mut d = first;
        while d <= last {
            let (gw, dow) = gps_week_day(d);
            let row = if let Some(r) = weeks.iter_mut().find(|(w, _)| *w == gw) {
                r
            } else {
                weeks.push((gw, [None; 7]));
                weeks.last_mut().unwrap()
            };
            row.1[dow as usize] = Some(d);
            d += Duration::days(1);
        }

        MonthCalendar { year, month, weeks }
    }
}

fn month_name(m: u32) -> &'static str {
    match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

fn fmt_day(val: u32, width: usize, is_today: bool, is_future: bool) -> String {
    let s = format!("{:>width$}", val, width = width);
    if is_today {
        format!("{}", s.black().on_white().bold())
    } else if is_future {
        format!("{}", s.bright_black())
    } else {
        s
    }
}

fn fmt_gps_week(week: i64, is_future: bool) -> String {
    let s = format!("{:>6}", week);
    if is_future {
        format!("{}", s.bright_black())
    } else {
        s
    }
}

fn format_month(cal: &MonthCalendar, today: NaiveDate) -> String {
    let mut out = String::new();
    let month_str = month_name(cal.month);

    // Each panel is 36 chars: " GPS WK" (7) + "  " (1) + 7 * " xxx" (28) = 36
    let panel_width = 36;
    let gap = "   ";

    let dom_title = month_str.to_string();
    let mid_label = format!("{}", cal.year);
    let doy_title = format!("{} DOY", month_str);

    let left_pad_dom = (panel_width - dom_title.len()) / 2;
    let right_pad_dom = panel_width - left_pad_dom - dom_title.len();
    let left_pad_doy = (panel_width - doy_title.len()) / 2;

    out.push_str(&format!(
        "{}{}{}{}{}{}\n",
        " ".repeat(left_pad_dom),
        dom_title.white().bold(),
        " ".repeat(right_pad_dom),
        format!("{:^3}", mid_label).bright_black(),
        " ".repeat(left_pad_doy),
        doy_title.white().bold(),
    ));

    let day_hdr = " GPS WK  Sun Mon Tue Wed Thu Fri Sat";
    out.push_str(&format!(
        "{}{}{}\n",
        day_hdr.bright_black(),
        gap,
        day_hdr.bright_black()
    ));

    for (gw, days) in &cal.weeks {
        let mut dom_cells = String::new();
        let mut doy_cells = String::new();

        for i in 0..7 {
            if let Some(date) = days[i] {
                let is_today = date == today;
                let is_future = date > today;
                dom_cells.push_str(&format!(" {}", fmt_day(date.day(), 3, is_today, is_future)));
                doy_cells
                    .push_str(&format!(" {}", fmt_day(date.ordinal(), 3, is_today, is_future)));
            } else {
                dom_cells.push_str("    ");
                doy_cells.push_str("    ");
            }
        }

        let week_is_future = gps_week_start(*gw) > today;
        out.push_str(&format!(
            " {} {}{} {} {}\n",
            fmt_gps_week(*gw, week_is_future),
            dom_cells,
            gap,
            fmt_gps_week(*gw, week_is_future),
            doy_cells
        ));
    }
    out
}

fn print_month(cal: &MonthCalendar, today: NaiveDate) {
    print!("{}", format_month(cal, today));
}

fn shift_month(date: NaiveDate, months: i32) -> NaiveDate {
    let total = date.year() * 12 + (date.month() as i32 - 1) + months;
    let y = total.div_euclid(12);
    let m = total.rem_euclid(12) as u32 + 1;
    NaiveDate::from_ymd_opt(y, m, 1).unwrap()
}

fn format_now_header(now_utc: chrono::DateTime<Utc>, now_local: chrono::DateTime<Local>) -> String {
    let utc_time = format!("{:<26}", now_utc.format("%Y-%m-%d %H:%M:%S UTC"));
    let local_time = format!("{:<26}", now_local.format("%Y-%m-%d %H:%M:%S %Z"));
    let mut s = String::new();
    s.push_str(&format!(
        "{} {}  {}{}\n",
        "Now (UTC):  ".white().bold(),
        utc_time.bright_cyan(),
        "DOY ".white().bold(),
        format!("{:03}", now_utc.ordinal()).bright_cyan()
    ));
    s.push_str(&format!(
        "{} {}  {}{}\n",
        "Now (Local):".white().bold(),
        local_time.bright_cyan(),
        "DOY ".white().bold(),
        format!("{:03}", now_local.ordinal()).bright_cyan()
    ));
    s
}

fn run_interactive(today: NaiveDate) -> io::Result<()> {
    let mut view = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let mut stdout = io::stdout();

    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show);
        panic_hook(info);
    }));

    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    let result = (|| -> io::Result<()> {
        loop {
            let now_utc = Utc::now();
            let now_local = now_utc.with_timezone(&Local);
            let prev = shift_month(view, -1);
            let next = shift_month(view, 1);

            let mut body = String::new();
            body.push_str(&format_now_header(now_utc, now_local));
            body.push_str(&format!(
                "{} {}\n\n",
                "View:       ".white().bold(),
                format!("{} {}", month_name(view.month()), view.year()).bright_yellow()
            ));
            body.push_str(&format_month(
                &MonthCalendar::new(prev.year(), prev.month()),
                today,
            ));
            body.push('\n');
            body.push_str(&format_month(
                &MonthCalendar::new(view.year(), view.month()),
                today,
            ));
            body.push('\n');
            body.push_str(&format_month(
                &MonthCalendar::new(next.year(), next.month()),
                today,
            ));
            body.push('\n');
            body.push_str(&format!(
                "{}",
                "↑ ↓  month     ← →  year     t  today     q  quit".bright_black()
            ));

            queue!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
            for line in body.split('\n') {
                queue!(stdout, Print(line), cursor::MoveToNextLine(1))?;
            }
            stdout.flush()?;

            match event::read()? {
                Event::Key(k) => match (k.code, k.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => break,
                    (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => break,
                    (KeyCode::Up, _)
                    | (KeyCode::Char('k'), _)
                    | (KeyCode::PageUp, _) => view = shift_month(view, -1),
                    (KeyCode::Down, _)
                    | (KeyCode::Char('j'), _)
                    | (KeyCode::PageDown, _) => view = shift_month(view, 1),
                    (KeyCode::Left, _) | (KeyCode::Char('h'), _) => view = shift_month(view, -12),
                    (KeyCode::Right, _) | (KeyCode::Char('l'), _) => view = shift_month(view, 12),
                    (KeyCode::Char('t'), _) | (KeyCode::Home, _) => {
                        view = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(())
    })();

    execute!(stdout, cursor::Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    result
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let interactive = args.iter().any(|a| a == "-i" || a == "--interactive");

    let now_utc = Utc::now();
    let now_local = now_utc.with_timezone(&Local);
    let today = now_utc.date_naive();

    if interactive {
        if let Err(e) = run_interactive(today) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }

    print!("{}", format_now_header(now_utc, now_local));
    println!();

    // Previous month
    let prev_month_date = if today.month() == 1 {
        NaiveDate::from_ymd_opt(today.year() - 1, 12, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(today.year(), today.month() - 1, 1).unwrap()
    };

    // Check if next GPS week spills into a following month
    let (current_gps_week, _) = gps_week_day(today);
    let next_week_end = gps_week_start(current_gps_week + 1) + Duration::days(6);
    let next_month_needed =
        next_week_end.month() != today.month() || next_week_end.year() != today.year();

    // Previous month
    let prev_cal = MonthCalendar::new(prev_month_date.year(), prev_month_date.month());
    print_month(&prev_cal, today);
    println!();

    // Current month
    let cur_cal = MonthCalendar::new(today.year(), today.month());
    print_month(&cur_cal, today);

    // Next month if next week spills over
    if next_month_needed {
        println!();
        let next_date = if today.month() == 12 {
            NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1).unwrap()
        };
        let next_cal = MonthCalendar::new(next_date.year(), next_date.month());
        print_month(&next_cal, today);
    }

    println!();
}
