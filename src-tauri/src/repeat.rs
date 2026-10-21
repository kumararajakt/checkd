//! Occurrence generation for repeat rules.
//!
//! A rule produces an infinite (or finite) series of dates. The rules that
//! matter:
//!
//! - **day** — every `interval` days from the start date.
//! - **week** — every `interval` weeks (ISO weeks, Monday-based), on the
//!   selected weekdays. An empty weekday list means "the start date's
//!   weekday".
//! - **month** — every `interval` months, anchored to a day of the month
//!   (clamped to the month's length), the last day, or the nth/last weekday.
//! - **year** — every `interval` years, on the start date's month and day
//!   (Feb 29 becomes Feb 28 in non-leap years).

use chrono::{Datelike, Duration, NaiveDate, Weekday};

use crate::db::{MonthRepeat, MonthWeek, Repeat, RepeatUnit};

/// Parses a `YYYY-MM-DD` date.
pub fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

/// Formats a date as `YYYY-MM-DD`.
pub fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// 0 = Sunday … 6 = Saturday (the convention used by the TS model).
fn weekday_index(date: NaiveDate) -> u32 {
    match date.weekday() {
        Weekday::Sun => 0,
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
    }
}

/// The offset of a weekday index inside an ISO week (Monday = 0).
fn offset_in_week(index: u32) -> i64 {
    ((index + 6) % 7) as i64
}

/// The Monday of the week containing `date`.
fn week_start(date: NaiveDate) -> NaiveDate {
    date - Duration::days(offset_in_week(weekday_index(date)) as i64)
}

/// The number of days in a month.
fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid month");
    let next = NaiveDate::from_ymd_opt(next_year, next_month, 1).expect("valid month");
    (next - first).num_days() as u32
}

/// Adds `delta` months to a year/month pair.
fn add_months(year: i32, month: u32, delta: i64) -> (i32, u32) {
    let total = year as i64 * 12 + (month as i64 - 1) + delta;
    let year = total.div_euclid(12) as i32;
    let month = total.rem_euclid(12) as u32 + 1;
    (year, month)
}

/// The nth occurrence of a weekday in a month, or the last one.
fn nth_weekday(year: i32, month: u32, weekday: u32, nth: Option<u32>) -> Option<NaiveDate> {
    let length = days_in_month(year, month);
    let mut matches = Vec::new();
    for day in 1..=length {
        let date = NaiveDate::from_ymd_opt(year, month, day)?;
        if weekday_index(date) == weekday {
            matches.push(date);
        }
    }
    match nth {
        Some(n) => matches.get(n as usize - 1).copied(),
        None => matches.last().copied(),
    }
}

/// The date of one "period" of the rule (a day, a week, a month or a year).
fn dates_for_period(rule: &Repeat, period: i64) -> Vec<NaiveDate> {
    let Some(start) = parse_date(&rule.start) else {
        return Vec::new();
    };
    let interval = rule.interval.max(1) as i64;

    match rule.unit {
        RepeatUnit::Day => {
            let date = start + Duration::days(period * interval);
            vec![date]
        }
        RepeatUnit::Week => {
            let anchor = week_start(start);
            let week = anchor + Duration::weeks(period * interval);
            let days: Vec<u32> = if rule.weekdays.is_empty() {
                vec![weekday_index(start)]
            } else {
                let mut days: Vec<u32> = rule
                    .weekdays
                    .iter()
                    .map(|day| (*day).clamp(0, 6) as u32)
                    .collect();
                days.sort_unstable();
                days.dedup();
                days
            };
            days
                .into_iter()
                .map(|day| week + Duration::days(offset_in_week(day)))
                .collect()
        }
        RepeatUnit::Month => {
            let (year, month) = add_months(start.year(), start.month(), period * interval);
            let date = match &rule.month_repeat {
                Some(MonthRepeat::Day { day }) => {
                    let day = (*day).clamp(1, days_in_month(year, month) as i64) as u32;
                    NaiveDate::from_ymd_opt(year, month, day)
                }
                Some(MonthRepeat::LastDay) => {
                    let day = days_in_month(year, month);
                    NaiveDate::from_ymd_opt(year, month, day)
                }
                Some(MonthRepeat::Weekday { weekday, week }) => nth_weekday(
                    year,
                    month,
                    *weekday as u32,
                    match week {
                        MonthWeek::Index(n) => Some(*n as u32),
                        MonthWeek::Last => None,
                    },
                ),
                None => {
                    let day = start.day().min(days_in_month(year, month));
                    NaiveDate::from_ymd_opt(year, month, day)
                }
            };
            date.into_iter().collect()
        }
        RepeatUnit::Year => {
            let year = start.year() + (period * interval) as i32;
            let month = start.month();
            let day = start.day().min(days_in_month(year, month));
            NaiveDate::from_ymd_opt(year, month, day).into_iter().collect()
        }
    }
}

/// The period indexes that could hold an occurrence near `date`.
fn candidate_periods(rule: &Repeat, date: NaiveDate) -> Vec<i64> {
    let Some(start) = parse_date(&rule.start) else {
        return Vec::new();
    };
    let interval = rule.interval.max(1) as i64;

    let estimate = match rule.unit {
        RepeatUnit::Day => (date - start).num_days().div_euclid(interval),
        RepeatUnit::Week => {
            let anchor = week_start(start);
            (date - anchor).num_days().div_euclid(7 * interval)
        }
        RepeatUnit::Month => {
            let months = (date.year() - start.year()) as i64 * 12
                + (date.month() as i64 - start.month() as i64);
            months.div_euclid(interval)
        }
        RepeatUnit::Year => ((date.year() - start.year()) as i64).div_euclid(interval),
    };

    // A period can hold several occurrences (a week with several weekdays), so
    // look at the neighbouring periods as well. Period 0 is the first one, so
    // the range never starts before it.
    let low = (estimate - 1).max(0);
    let high = (estimate + 1).max(low);
    (low..=high).collect()
}

/// The first occurrence on or after `from`.
///
/// Occurrences before the rule's start date are never produced.
pub fn first_at_or_after(rule: &Repeat, from: NaiveDate) -> Option<NaiveDate> {
    let start = parse_date(&rule.start)?;

    for period in candidate_periods(rule, from) {
        for date in dates_for_period(rule, period) {
            if date >= start && date >= from && within_end(rule, date) {
                return Some(date);
            }
        }
    }
    None
}

/// The first occurrence strictly after `after`, or `None` when the series has
/// ended.
pub fn next_after(rule: &Repeat, after: NaiveDate) -> Option<NaiveDate> {
    let start = parse_date(&rule.start)?;

    let mut candidates: Vec<NaiveDate> = Vec::new();
    for period in candidate_periods(rule, after) {
        candidates.extend(dates_for_period(rule, period));
    }
    candidates.sort_unstable();
    candidates.dedup();

    candidates
        .into_iter()
        .find(|date| *date > after && *date >= start && within_end(rule, *date))
}

/// Whether the series has ended by `date` ("never" / "on date" ends).
fn within_end(rule: &Repeat, date: NaiveDate) -> bool {
    match &rule.end {
        crate::db::RepeatEnd::Never => true,
        crate::db::RepeatEnd::On { date: end } => match parse_date(end) {
            Some(end) => date <= end,
            None => true,
        },
        crate::db::RepeatEnd::After { .. } => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{MonthRepeat, MonthWeek, RepeatEnd, RepeatUnit};

    fn rule(unit: RepeatUnit, start: &str, interval: i64) -> Repeat {
        Repeat {
            unit,
            interval,
            start: start.to_string(),
            time: None,
            weekdays: Vec::new(),
            month_repeat: None,
            end: RepeatEnd::Never,
        }
    }

    fn date(value: &str) -> NaiveDate {
        parse_date(value).unwrap()
    }

    #[test]
    fn daily_rule_advances_by_the_interval() {
        let rule = rule(RepeatUnit::Day, "2026-10-21", 3);
        assert_eq!(first_at_or_after(&rule, date("2026-10-01")), Some(date("2026-10-21")));
        assert_eq!(next_after(&rule, date("2026-10-21")), Some(date("2026-10-24")));
        assert_eq!(next_after(&rule, date("2026-10-22")), Some(date("2026-10-24")));
    }

    #[test]
    fn daily_rule_skips_missed_days() {
        let rule = rule(RepeatUnit::Day, "2026-10-01", 1);
        assert_eq!(first_at_or_after(&rule, date("2026-10-20")), Some(date("2026-10-20")));
    }

    #[test]
    fn weekly_rule_without_weekdays_uses_the_start_weekday() {
        // 2026-10-21 is a Wednesday.
        let rule = rule(RepeatUnit::Week, "2026-10-21", 1);
        assert_eq!(next_after(&rule, date("2026-10-21")), Some(date("2026-10-28")));
    }

    #[test]
    fn weekly_rule_with_several_weekdays() {
        let mut rule = rule(RepeatUnit::Week, "2026-10-19", 1); // Monday
        rule.weekdays = vec![1, 3, 5]; // Mon, Wed, Fri
        assert_eq!(first_at_or_after(&rule, date("2026-10-19")), Some(date("2026-10-19")));
        assert_eq!(next_after(&rule, date("2026-10-19")), Some(date("2026-10-21")));
        assert_eq!(next_after(&rule, date("2026-10-21")), Some(date("2026-10-23")));
        assert_eq!(next_after(&rule, date("2026-10-23")), Some(date("2026-10-26")));
    }

    #[test]
    fn weekly_rule_every_two_weeks() {
        let mut rule = rule(RepeatUnit::Week, "2026-10-19", 2); // Monday
        rule.weekdays = vec![1];
        assert_eq!(next_after(&rule, date("2026-10-19")), Some(date("2026-11-02")));
        assert_eq!(next_after(&rule, date("2026-11-02")), Some(date("2026-11-16")));
    }

    #[test]
    fn monthly_rule_clamps_to_the_month_length() {
        let mut rule = rule(RepeatUnit::Month, "2026-01-31", 1);
        rule.month_repeat = Some(MonthRepeat::Day { day: 31 });
        assert_eq!(next_after(&rule, date("2026-01-31")), Some(date("2026-02-28")));
        assert_eq!(next_after(&rule, date("2026-02-28")), Some(date("2026-03-31")));
    }

    #[test]
    fn monthly_rule_last_day() {
        let mut rule = rule(RepeatUnit::Month, "2026-01-31", 1);
        rule.month_repeat = Some(MonthRepeat::LastDay);
        assert_eq!(next_after(&rule, date("2026-01-31")), Some(date("2026-02-28")));
    }

    #[test]
    fn monthly_rule_nth_and_last_weekday() {
        let mut rule = rule(RepeatUnit::Month, "2026-10-21", 1);
        rule.month_repeat = Some(MonthRepeat::Weekday {
            weekday: 3, // Wednesday
            week: MonthWeek::Index(3),
        });
        // Third Wednesday of October 2026 is the 21st.
        assert_eq!(next_after(&rule, date("2026-10-20")), Some(date("2026-10-21")));

        rule.month_repeat = Some(MonthRepeat::Weekday {
            weekday: 3,
            week: MonthWeek::Last,
        });
        // Last Wednesday of October 2026 is the 28th.
        assert_eq!(next_after(&rule, date("2026-10-21")), Some(date("2026-10-28")));
    }

    #[test]
    fn yearly_rule_clamps_leap_day() {
        let rule = rule(RepeatUnit::Year, "2024-02-29", 1);
        assert_eq!(next_after(&rule, date("2024-02-29")), Some(date("2025-02-28")));
    }

    #[test]
    fn end_on_date_stops_the_series() {
        let mut rule = rule(RepeatUnit::Day, "2026-10-21", 1);
        rule.end = RepeatEnd::On {
            date: "2026-10-23".to_string(),
        };
        assert_eq!(next_after(&rule, date("2026-10-22")), Some(date("2026-10-23")));
        assert_eq!(next_after(&rule, date("2026-10-23")), None);
    }
}
