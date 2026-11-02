/// Tipos de conta disponíveis na V1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountType {
    Bank,
    Cash,
    Savings,
    Investment,
    DigitalWallet,
    OtherAsset,
    CreditCard,
}

impl AccountType {
    pub const ALL: [AccountType; 7] = [
        AccountType::Bank,
        AccountType::Cash,
        AccountType::Savings,
        AccountType::Investment,
        AccountType::DigitalWallet,
        AccountType::OtherAsset,
        AccountType::CreditCard,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            AccountType::Bank => "bank",
            AccountType::Cash => "cash",
            AccountType::Savings => "savings",
            AccountType::Investment => "investment",
            AccountType::DigitalWallet => "digital_wallet",
            AccountType::OtherAsset => "other_asset",
            AccountType::CreditCard => "credit_card",
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
            "credit_card" => Some(AccountType::CreditCard),
            _ => None,
        }
    }

    /// Classe contábil da conta. Cartão é passivo; o restante é ativo.
    pub const fn class(self) -> &'static str {
        match self {
            AccountType::CreditCard => "liability",
            _ => "asset",
        }
    }

    pub const fn is_credit_card(self) -> bool {
        matches!(self, AccountType::CreditCard)
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
    fn only_credit_card_is_a_liability() {
        for account_type in AccountType::ALL {
            let expected = if account_type == AccountType::CreditCard {
                "liability"
            } else {
                "asset"
            };
            assert_eq!(account_type.class(), expected);
        }
    }

    #[test]
    fn rejects_unknown_types() {
        assert_eq!(AccountType::parse(""), None);
        assert_eq!(AccountType::parse("credit"), None);
    }
}
