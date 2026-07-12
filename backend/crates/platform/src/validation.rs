use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};

pub(crate) fn reason(value: &str) -> AppResult<&str> {
    let value = value.trim();
    if !(3..=500).contains(&value.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    Ok(value)
}

pub(crate) fn optional_text(
    value: Option<&str>,
    max_chars: usize,
    field: &str,
) -> AppResult<Option<String>> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    if value.is_some_and(|value| value.chars().count() > max_chars) {
        return Err(AppError::BadRequest(format!(
            "{field} must be at most {max_chars} characters"
        )));
    }
    Ok(value.map(str::to_owned))
}

pub(crate) fn required_text(value: &str, max_chars: usize, field: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_chars {
        return Err(AppError::BadRequest(format!("{field} must be 1–{max_chars} characters")));
    }
    Ok(value.to_owned())
}

pub(crate) fn timestamp(value: Option<i64>, field: &str) -> AppResult<Option<DateTime<Utc>>> {
    value
        .map(|value| {
            DateTime::from_timestamp(value, 0)
                .ok_or_else(|| AppError::BadRequest(format!("{field} is out of range")))
        })
        .transpose()
}

pub(crate) fn schedule(
    status: &str,
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    if starts_at.zip(ends_at).is_some_and(|(starts_at, ends_at)| ends_at <= starts_at) {
        return Err(AppError::BadRequest("endsAt must be later than startsAt".into()));
    }
    if status == "scheduled" && starts_at.is_none_or(|starts_at| starts_at <= now) {
        return Err(AppError::BadRequest("scheduled content requires a future startsAt".into()));
    }
    if status == "published" && starts_at.is_some_and(|starts_at| starts_at > now) {
        return Err(AppError::BadRequest("future content must use scheduled status".into()));
    }
    if matches!(status, "published" | "scheduled") && ends_at.is_some_and(|ends_at| ends_at <= now)
    {
        return Err(AppError::BadRequest("active content cannot end in the past".into()));
    }
    Ok(())
}

pub(crate) fn parse_id(value: &str, field: &str) -> AppResult<i64> {
    value
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::BadRequest(format!("invalid {field}")))
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::schedule;

    #[test]
    fn rejects_scheduled_content_without_a_future_start() {
        let now = Utc::now();
        assert!(schedule("scheduled", None, None, now).is_err());
        assert!(schedule("scheduled", Some(now - Duration::seconds(1)), None, now).is_err());
    }

    #[test]
    fn accepts_a_bounded_future_schedule() {
        let now = Utc::now();
        assert!(schedule(
            "scheduled",
            Some(now + Duration::hours(1)),
            Some(now + Duration::hours(2)),
            now,
        )
        .is_ok());
    }
}
