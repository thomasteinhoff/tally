use std::fmt;

/// Dinheiro é sempre representado em centavos inteiros.
///
/// Nenhuma regra financeira pode usar ponto flutuante. O newtype evita
/// misturar centavos com outras grandezas numéricas por acidente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Money(i64);

impl Money {
    pub const ZERO: Self = Self(0);

    pub const fn from_cents(cents: i64) -> Self {
        Self(cents)
    }

    pub const fn cents(self) -> i64 {
        self.0
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    pub fn checked_neg(self) -> Option<Self> {
        self.0.checked_neg().map(Self)
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let absolute = self.0.unsigned_abs();
        write!(f, "{sign}{},{:02}", absolute / 100, absolute % 100)
    }
}

#[cfg(test)]
mod tests {
    use super::Money;

    #[test]
    fn formats_cents_without_floating_point() {
        assert_eq!(Money::from_cents(0).to_string(), "0,00");
        assert_eq!(Money::from_cents(1).to_string(), "0,01");
        assert_eq!(Money::from_cents(123_456).to_string(), "1234,56");
        assert_eq!(Money::from_cents(-1_999).to_string(), "-19,99");
    }

    #[test]
    fn checked_arithmetic_detects_overflow() {
        assert_eq!(
            Money::from_cents(i64::MAX).checked_add(Money::from_cents(1)),
            None
        );
        assert_eq!(
            Money::from_cents(i64::MIN).checked_sub(Money::from_cents(1)),
            None
        );
    }

    #[test]
    fn checked_negation_detects_overflow() {
        assert_eq!(
            Money::from_cents(1_000).checked_neg(),
            Some(Money::from_cents(-1_000))
        );
        assert_eq!(Money::from_cents(i64::MIN).checked_neg(), None);
    }
}
