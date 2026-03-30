use chrono::{Days, NaiveDate};

pub fn next_streak(last_checkin: Option<NaiveDate>, today: NaiveDate, current_streak: i32) -> i32 {
    match last_checkin.and_then(|date| date.checked_add_days(Days::new(1))) {
        Some(next_day) if next_day == today => current_streak + 1,
        _ => 1,
    }
}

pub fn reward_for_streak_day(streak_day: i32, reward_sequence: &[i32]) -> Option<i32> {
    if streak_day <= 0 || reward_sequence.is_empty() {
        return None;
    }

    let index = usize::try_from(streak_day.saturating_sub(1)).ok()?;
    reward_sequence
        .get(index)
        .copied()
        .or_else(|| reward_sequence.last().copied())
}

pub fn reward_sum_for_streak_range(
    start_day: i32,
    end_day: i32,
    reward_sequence: &[i32],
) -> Option<i32> {
    if end_day < start_day {
        return Some(0);
    }

    (start_day..=end_day).try_fold(0_i32, |acc, streak_day| {
        let reward = reward_for_streak_day(streak_day, reward_sequence)?;
        acc.checked_add(reward)
    })
}

pub fn missed_days_since_last_checkin(last_checkin: Option<NaiveDate>, today: NaiveDate) -> i64 {
    match last_checkin {
        Some(last_checkin) => (today - last_checkin).num_days().saturating_sub(1),
        None => 0,
    }
}

pub fn makeup_cost(days: i64, gold_per_day: i32, diamond_cost: i32) -> Option<(i32, i32)> {
    let days = i32::try_from(days).ok()?;
    let gold_cost = days.checked_mul(gold_per_day)?;

    Some((gold_cost, diamond_cost))
}

pub fn makeup_dates(last_checkin: NaiveDate, today: NaiveDate) -> Vec<NaiveDate> {
    let missed_days = missed_days_since_last_checkin(Some(last_checkin), today);

    (1..=missed_days)
        .filter_map(|offset| last_checkin.checked_add_days(Days::new(offset as u64)))
        .collect()
}
