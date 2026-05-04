use chrono::{Datelike, Duration, Local, NaiveDate, Utc};
use colored::*;

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

fn print_month(cal: &MonthCalendar, today: NaiveDate) {
    let month_str = month_name(cal.month);

    // Each panel is 36 chars: " GPS WK" (7) + "  " (1) + 7 * " xxx" (28) = 36
    let panel_width = 36;
    let gap = "   ";

    // Title
    let dom_title = month_str.to_string();
    let mid_label = format!("{}", cal.year);
    let doy_title = format!("{} DOY", month_str);

    let left_pad_dom = (panel_width - dom_title.len()) / 2;
    let right_pad_dom = panel_width - left_pad_dom - dom_title.len();
    let left_pad_doy = (panel_width - doy_title.len()) / 2;

    println!(
        "{}{}{}{}{}{}",
        " ".repeat(left_pad_dom),
        dom_title.white().bold(),
        " ".repeat(right_pad_dom),
        format!("{:^3}", mid_label).bright_black(),
        " ".repeat(left_pad_doy),
        doy_title.white().bold(),
    );

    // Column headers
    let day_hdr = " GPS WK  Sun Mon Tue Wed Thu Fri Sat";
    println!("{}{}{}", day_hdr.bright_black(), gap, day_hdr.bright_black());

    // Rows
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
        println!(
            " {} {}{} {} {}",
            fmt_gps_week(*gw, week_is_future),
            dom_cells,
            gap,
            fmt_gps_week(*gw, week_is_future),
            doy_cells
        );
    }
}

fn main() {
    let now_utc = Utc::now();
    let now_local = now_utc.with_timezone(&Local);
    let today = now_utc.date_naive();

    let utc_time = format!("{:<26}", now_utc.format("%Y-%m-%d %H:%M:%S UTC"));
    let local_time = format!("{:<26}", now_local.format("%Y-%m-%d %H:%M:%S %Z"));
    println!(
        "{} {}  {}{}",
        "Now (UTC):  ".white().bold(),
        utc_time.bright_cyan(),
        "DOY ".white().bold(),
        format!("{:03}", now_utc.ordinal()).bright_cyan()
    );
    println!(
        "{} {}  {}{}",
        "Now (Local):".white().bold(),
        local_time.bright_cyan(),
        "DOY ".white().bold(),
        format!("{:03}", now_local.ordinal()).bright_cyan()
    );
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
