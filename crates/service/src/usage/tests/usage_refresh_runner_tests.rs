use super::*;
use chrono::{TimeZone, Timelike};

#[test]
fn warmup_cron_accepts_five_field_expression() {
    let after = Local
        .with_ymd_and_hms(2026, 5, 23, 10, 15, 30)
        .single()
        .expect("local timestamp");

    let next = next_cron_after("0 */4 * * *", after).expect("next run");

    assert_eq!(next.hour(), 12);
    assert_eq!(next.minute(), 0);
    assert_eq!(next.second(), 0);
}

#[test]
fn warmup_cron_accepts_six_field_expression() {
    let after = Local
        .with_ymd_and_hms(2026, 5, 23, 10, 15, 30)
        .single()
        .expect("local timestamp");

    let next = next_cron_after("45 15 10 * * *", after).expect("next run");

    assert_eq!(next.hour(), 10);
    assert_eq!(next.minute(), 15);
    assert_eq!(next.second(), 45);
}

#[test]
fn warmup_cron_uses_earliest_pipe_separated_schedule() {
    let after = Local
        .with_ymd_and_hms(2026, 5, 23, 10, 15, 30)
        .single()
        .expect("local timestamp");

    let next = next_cron_after("0 18 * * *|30 10 * * *", after).expect("next run");

    assert_eq!(next.hour(), 10);
    assert_eq!(next.minute(), 30);
}
