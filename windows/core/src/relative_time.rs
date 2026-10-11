pub fn time_left(reset: i64, now: i64) -> String {
    let seconds = reset.saturating_sub(now);
    if seconds <= 0 {
        return "resetting…".into();
    }
    if seconds < 60 {
        return format!("in {seconds}s");
    }
    let minutes = seconds / 60;
    let d = minutes / 1440;
    let h = minutes % 1440 / 60;
    let m = minutes % 60;
    let mut parts = Vec::new();
    if d > 0 {
        parts.push(format!("{d}d"));
    }
    if h > 0 {
        parts.push(format!("{h}h"));
    }
    if m > 0 {
        parts.push(format!("{m}m"));
    }
    format!("in {}", parts.join(" "))
}
pub fn parse_date(s: &str) -> Option<i64> {
    if !s.is_ascii() || s.len() < 20 {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: i64 = s.get(5..7)?.parse().ok()?;
    let day: i64 = s.get(8..10)?.parse().ok()?;
    if s.get(4..5)? != "-"
        || s.get(7..8)? != "-"
        || !matches!(s.get(10..11)?, "T" | "t")
        || s.get(13..14)? != ":"
        || s.get(16..17)? != ":"
    {
        return None;
    }
    let hour: i64 = s[11..13].parse().ok()?;
    let min: i64 = s[14..16].parse().ok()?;
    let sec: i64 = s[17..19].parse().ok()?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let md = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&month)
        || day < 1
        || day > md[(month - 1) as usize]
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&min)
        || !(0..=59).contains(&sec)
    {
        return None;
    }
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yo = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let days = era * 146097 + yo * 365 + yo / 4 - yo / 100 + (153 * mp + 2) / 5 + day - 1 - 719468;
    let suffix = &s[19..];
    let suffix = if suffix.starts_with('.') {
        let cut = suffix.find(['Z', 'z', '+', '-'])?;
        if cut <= 1 || !suffix[1..cut].bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        &suffix[cut..]
    } else {
        suffix
    };
    let offset = if suffix == "Z" || suffix == "z" {
        0
    } else {
        if suffix.len() != 6 || !matches!(&suffix[0..1], "+" | "-") || &suffix[3..4] != ":" {
            return None;
        }
        let h: i64 = suffix[1..3].parse().ok()?;
        let m: i64 = suffix[4..6].parse().ok()?;
        if !(0..=23).contains(&h) || !(0..=59).contains(&m) {
            return None;
        }
        (h * 3600 + m * 60) * if suffix.starts_with('-') { -1 } else { 1 }
    };
    Some(days * 86400 + hour * 3600 + min * 60 + sec - offset)
}
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
