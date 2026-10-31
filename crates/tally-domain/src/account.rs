/// Tipos de conta disponíveis na Phase 1.
///
/// Cartão de crédito é liability e entra na Phase 2; por isso não está
/// listado aqui e `create_account` o rejeita por enquanto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountType {
    Bank,
    Cash,
    Savings,
    Investment,
    DigitalWallet,
    OtherAsset,
}

impl AccountType {
    pub const ALL: [AccountType; 6] = [
        AccountType::Bank,
        AccountType::Cash,
        AccountType::Savings,
        AccountType::Investment,
        AccountType::DigitalWallet,
        AccountType::OtherAsset,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            AccountType::Bank => "bank",
            AccountType::Cash => "cash",
            AccountType::Savings => "savings",
            AccountType::Investment => "investment",
            AccountType::DigitalWallet => "digital_wallet",
            AccountType::OtherAsset => "other_asset",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "bank" => Some(AccountType::Bank),
            "cash" => Some(AccountType::Cash),
            "savings" => Some(AccountType::Savings),
            "investment" => Some(AccountType::Investment),
            "digital_wallet" => Some(AccountType::DigitalWallet),
            "other_asset" => Some(AccountType::OtherAsset),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AccountType;

    #[test]
    fn round_trips_every_account_type() {
        for account_type in AccountType::ALL {
            assert_eq!(
                AccountType::parse(account_type.as_str()),
                Some(account_type)
            );
        }
    }

    #[test]
    fn rejects_types_outside_phase_one() {
        assert_eq!(AccountType::parse("credit_card"), None);
        assert_eq!(AccountType::parse(""), None);
    }
}
