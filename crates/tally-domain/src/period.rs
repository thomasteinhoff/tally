use std::fmt;

/// Período do dia usado para agrupar movimentações.
///
/// O período automático é derivado do horário. `Custom` existe apenas como
/// valor manual; ele nunca é produzido por `Period::from_time`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Period {
    Madrugada,
    Manha,
    Tarde,
    Noite,
    Custom,
}

impl Period {
    pub const ALL: [Period; 5] = [
        Period::Madrugada,
        Period::Manha,
        Period::Tarde,
        Period::Noite,
        Period::Custom,
    ];

    /// Deriva o período de um horário local no formato `HH:MM`.
    ///
    /// Limites: 00:00–04:59 madrugada, 05:00–11:59 manhã,
    /// 12:00–18:59 tarde, 19:00–23:59 noite.
    pub fn from_time(value: &str) -> Option<Self> {
        let (hour, minute) = parse_hhmm(value)?;
        let minutes = hour * 60 + minute;

        Some(match minutes {
            0..=299 => Period::Madrugada,
            300..=719 => Period::Manha,
            720..=1139 => Period::Tarde,
            1140..=1439 => Period::Noite,
            _ => return None,
        })
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Period::Madrugada => "madrugada",
            Period::Manha => "manha",
            Period::Tarde => "tarde",
            Period::Noite => "noite",
            Period::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "madrugada" => Some(Period::Madrugada),
            "manha" => Some(Period::Manha),
            "tarde" => Some(Period::Tarde),
            "noite" => Some(Period::Noite),
            "custom" => Some(Period::Custom),
            _ => None,
        }
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn parse_hhmm(value: &str) -> Option<(u32, u32)> {
    let bytes = value.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return None;
    }

    if !bytes[0].is_ascii_digit()
        || !bytes[1].is_ascii_digit()
        || !bytes[3].is_ascii_digit()
        || !bytes[4].is_ascii_digit()
    {
        return None;
    }

    let hour = u32::from(bytes[0] - b'0') * 10 + u32::from(bytes[1] - b'0');
    let minute = u32::from(bytes[3] - b'0') * 10 + u32::from(bytes[4] - b'0');

    if hour > 23 || minute > 59 {
        return None;
    }

    Some((hour, minute))
}

#[cfg(test)]
mod tests {
    use super::Period;

    #[test]
    fn derives_period_at_exact_boundaries() {
        assert_eq!(Period::from_time("00:00"), Some(Period::Madrugada));
        assert_eq!(Period::from_time("04:59"), Some(Period::Madrugada));
        assert_eq!(Period::from_time("05:00"), Some(Period::Manha));
        assert_eq!(Period::from_time("11:59"), Some(Period::Manha));
        assert_eq!(Period::from_time("12:00"), Some(Period::Tarde));
        assert_eq!(Period::from_time("18:59"), Some(Period::Tarde));
        assert_eq!(Period::from_time("19:00"), Some(Period::Noite));
        assert_eq!(Period::from_time("23:59"), Some(Period::Noite));
    }

    #[test]
    fn rejects_invalid_or_malformed_times() {
        assert_eq!(Period::from_time("24:00"), None);
        assert_eq!(Period::from_time("12:60"), None);
        assert_eq!(Period::from_time("9:00"), None);
        assert_eq!(Period::from_time("12:0"), None);
        assert_eq!(Period::from_time("12-00"), None);
        assert_eq!(Period::from_time(""), None);
    }

    #[test]
    fn custom_is_manual_only() {
        assert_eq!(Period::parse("custom"), Some(Period::Custom));
        assert_eq!(Period::from_time("12:00"), Some(Period::Tarde));
    }
}
