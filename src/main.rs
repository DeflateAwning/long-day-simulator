use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use clap::Parser;
use icalendar::{Calendar, Component, Event, EventLike};
use std::fs;
use std::path::PathBuf;

/// Simulate a non-24-hour sleep schedule and export it to an ICS calendar file.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Bedtime on the start day (HH:MM, 24h format)
    #[arg(long, default_value = "01:30")]
    bedtime: String,

    /// Start date (YYYY-MM-DD). Defaults to today.
    #[arg(long)]
    start_date: Option<String>,

    /// Sleep duration in hours (can be fractional, e.g. 8.5)
    #[arg(long, default_value_t = 8.0)]
    sleep_hours: f64,

    /// Total day length in hours (e.g. 25 for a 25-hour day)
    #[arg(long, default_value_t = 25.0)]
    day_length_hours: f64,

    /// Number of days to simulate
    #[arg(long, default_value_t = 30)]
    days: i64,

    /// Output ICS file path
    #[arg(long, short, default_value = "long_day_schedule.ics")]
    output: PathBuf,

    /// Event periods to generate: sleep, awake, or both (comma-separated)
    #[arg(long, default_value = "sleep", value_delimiter = ',')]
    include: Vec<String>,
}

#[derive(Debug, Clone)]
struct Period {
    start: NaiveDateTime,
    end: NaiveDateTime,
}

impl Period {
    fn duration(&self) -> Duration {
        self.end - self.start
    }
}

fn local_naive_to_utc(dt: NaiveDateTime) -> DateTime<Utc> {
    Local
        .from_local_datetime(&dt)
        .single()
        .expect("Ambiguous or invalid local time")
        .with_timezone(&Utc)
}

fn run_sim(cli: &Cli) -> (Vec<Period>, Vec<Period>) {
    let bedtime = NaiveTime::parse_from_str(&cli.bedtime, "%H:%M")
        .unwrap_or_else(|_| panic!("Invalid bedtime format '{}', expected HH:MM", cli.bedtime));

    let start_date = match &cli.start_date {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .unwrap_or_else(|_| panic!("Invalid date format '{}', expected YYYY-MM-DD", s)),
        None => Local::now().date_naive(),
    };

    let sleep_duration = Duration::seconds((cli.sleep_hours * 3600.0) as i64);
    let day_length = Duration::seconds((cli.day_length_hours * 3600.0) as i64);
    let awake_duration = day_length - sleep_duration;

    let stop_dt = Local::now().naive_local() + Duration::days(cli.days);

    let mut sleep_periods: Vec<Period> = Vec::new();
    let mut awake_periods: Vec<Period> = Vec::new();

    let mut cur_dt = NaiveDateTime::new(start_date, bedtime);
    let mut last_sleep_start = cur_dt;

    println!("Go to sleep at {}", cur_dt);

    while cur_dt <= stop_dt {
        let next_awake_start = cur_dt + sleep_duration;
        println!("Wake up at {}", next_awake_start);

        let next_sleep_start = next_awake_start + awake_duration;
        println!("Go to sleep at {}", next_sleep_start);

        sleep_periods.push(Period {
            start: last_sleep_start,
            end: next_awake_start,
        });
        awake_periods.push(Period {
            start: next_awake_start,
            end: next_sleep_start,
        });

        last_sleep_start = next_sleep_start;
        cur_dt = next_sleep_start;
    }

    (sleep_periods, awake_periods)
}

fn write_to_ics(
    sleep_periods: &[Period],
    awake_periods: &[Period],
    include: &[String],
    output: &PathBuf,
) {
    let mut calendar = Calendar::new();

    if include.iter().any(|s| s == "sleep") {
        for period in sleep_periods {
            let event = Event::new()
                .summary("Sleep")
                .starts(local_naive_to_utc(period.start))
                .ends(local_naive_to_utc(period.end))
                .done();
            calendar.push(event);
        }
    }

    if include.iter().any(|s| s == "awake") {
        for period in awake_periods {
            let event = Event::new()
                .summary("Awake")
                .starts(local_naive_to_utc(period.start))
                .ends(local_naive_to_utc(period.end))
                .done();
            calendar.push(event);
        }
    }

    let ics_string = calendar.to_string();
    fs::write(output, ics_string)
        .unwrap_or_else(|e| panic!("Failed to write ICS file '{}': {}", output.display(), e));
}

fn main() {
    let cli = Cli::parse();

    let (sleep_periods, awake_periods) = run_sim(&cli);

    println!("\nSleep periods ({}):", sleep_periods.len());
    for p in &sleep_periods {
        println!(
            "  {} -> {} ({:.1}h)",
            p.start,
            p.end,
            p.duration().num_minutes() as f64 / 60.0
        );
    }
    println!("Awake periods ({}):", awake_periods.len());
    for p in &awake_periods {
        println!(
            "  {} -> {} ({:.1}h)",
            p.start,
            p.end,
            p.duration().num_minutes() as f64 / 60.0
        );
    }

    write_to_ics(&sleep_periods, &awake_periods, &cli.include, &cli.output);

    println!(
        "\nWrote {} sleep periods and {} awake periods to '{}'.",
        sleep_periods.len(),
        awake_periods.len(),
        cli.output.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_default_config_generates_ics_file() {
        let output = temp_dir().join("long_day_simulator_test.ics");

        let cli = Cli {
            bedtime: "01:30".to_string(),
            start_date: Some("2026-01-01".to_string()),
            sleep_hours: 8.0,
            day_length_hours: 25.0,
            days: 30,
            output: output.clone(),
            include: vec!["sleep".to_string()],
        };

        let (sleep_periods, awake_periods) = run_sim(&cli);
        write_to_ics(&sleep_periods, &awake_periods, &cli.include, &cli.output);

        assert!(output.exists(), "ICS file was not created");
        assert!(output.metadata().unwrap().len() > 0, "ICS file is empty");

        let contents = std::fs::read_to_string(&output).unwrap();
        assert!(contents.contains("BEGIN:VCALENDAR"));
        assert!(contents.contains("SUMMARY:Sleep"));
        assert!(!contents.contains("SUMMARY:Awake"));

        // 30-day sim with a 25h day: expect roughly 28-29 sleep periods
        assert!(!sleep_periods.is_empty());
        assert_eq!(awake_periods.len(), sleep_periods.len());
        for p in &sleep_periods {
            assert_eq!(
                p.duration(),
                Duration::hours(8),
                "each sleep period should be exactly 8h"
            );
        }

        std::fs::remove_file(&output).ok();
    }
}
