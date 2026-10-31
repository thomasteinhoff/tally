/// Tipos de movimentação simples disponíveis na Phase 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MovementKind {
    Expense,
    Income,
    Transfer,
    Adjustment,
}

impl MovementKind {
    pub const ALL: [MovementKind; 4] = [
        MovementKind::Expense,
        MovementKind::Income,
        MovementKind::Transfer,
        MovementKind::Adjustment,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            MovementKind::Expense => "expense",
            MovementKind::Income => "income",
            MovementKind::Transfer => "transfer",
            MovementKind::Adjustment => "adjustment",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "expense" => Some(MovementKind::Expense),
            "income" => Some(MovementKind::Income),
            "transfer" => Some(MovementKind::Transfer),
            "adjustment" => Some(MovementKind::Adjustment),
            _ => None,
        }
    }

    pub const fn is_simple(self) -> bool {
        matches!(
            self,
            MovementKind::Expense
                | MovementKind::Income
                | MovementKind::Transfer
                | MovementKind::Adjustment
        )
    }
}

/// Método de pagamento. Cartão existe no modelo, mas a Phase 1 só aceita
/// Pix, débito e dinheiro nas movimentações simples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaymentMethod {
    Pix,
    Debit,
    Credit,
    SubscriptionCredit,
    Cash,
}

impl PaymentMethod {
    pub const ALL: [PaymentMethod; 5] = [
        PaymentMethod::Pix,
        PaymentMethod::Debit,
        PaymentMethod::Credit,
        PaymentMethod::SubscriptionCredit,
        PaymentMethod::Cash,
    ];

    pub const PHASE_ONE: [PaymentMethod; 3] = [
        PaymentMethod::Pix,
        PaymentMethod::Debit,
        PaymentMethod::Cash,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            PaymentMethod::Pix => "pix",
            PaymentMethod::Debit => "debit",
            PaymentMethod::Credit => "credit",
            PaymentMethod::SubscriptionCredit => "subscription_credit",
            PaymentMethod::Cash => "cash",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pix" => Some(PaymentMethod::Pix),
            "debit" => Some(PaymentMethod::Debit),
            "credit" => Some(PaymentMethod::Credit),
            "subscription_credit" => Some(PaymentMethod::SubscriptionCredit),
            "cash" => Some(PaymentMethod::Cash),
            _ => None,
        }
    }

    pub fn parse_phase_one(value: &str) -> Option<Self> {
        Self::parse(value).filter(|method| Self::PHASE_ONE.contains(method))
    }
}

#[cfg(test)]
mod tests {
    use super::{MovementKind, PaymentMethod};

    #[test]
    fn round_trips_movement_kinds() {
        for kind in MovementKind::ALL {
            assert_eq!(MovementKind::parse(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn phase_one_rejects_card_payment_methods() {
        assert_eq!(
            PaymentMethod::parse_phase_one("pix"),
            Some(PaymentMethod::Pix)
        );
        assert_eq!(
            PaymentMethod::parse_phase_one("cash"),
            Some(PaymentMethod::Cash)
        );
        assert_eq!(PaymentMethod::parse_phase_one("credit"), None);
        assert_eq!(PaymentMethod::parse_phase_one("subscription_credit"), None);
    }

    #[test]
    fn parses_all_payment_methods_for_reading_history() {
        for method in PaymentMethod::ALL {
            assert_eq!(PaymentMethod::parse(method.as_str()), Some(method));
        }
    }
}
