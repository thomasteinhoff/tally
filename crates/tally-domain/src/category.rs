/// Tipo de categoria na Phase 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CategoryKind {
    Expense,
    Income,
}

impl CategoryKind {
    pub const ALL: [CategoryKind; 2] = [CategoryKind::Expense, CategoryKind::Income];

    pub const fn as_str(self) -> &'static str {
        match self {
            CategoryKind::Expense => "expense",
            CategoryKind::Income => "income",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "expense" => Some(CategoryKind::Expense),
            "income" => Some(CategoryKind::Income),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CategoryKind;

    #[test]
    fn round_trips_every_category_kind() {
        for kind in CategoryKind::ALL {
            assert_eq!(CategoryKind::parse(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn rejects_unknown_category_kind() {
        assert_eq!(CategoryKind::parse("transfer"), None);
        assert_eq!(CategoryKind::parse(""), None);
    }
}
