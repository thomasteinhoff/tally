use crate::money::Money;

/// Partida de um lançamento em partidas dobradas.
///
/// O sinal segue a convenção de débito positivo: valor positivo é débito,
/// valor negativo é crédito. Um lançamento válido soma zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posting {
    pub id: String,
    pub account_id: String,
    pub amount: Money,
}

impl Posting {
    pub fn new(id: impl Into<String>, account_id: impl Into<String>, amount: Money) -> Self {
        Self {
            id: id.into(),
            account_id: account_id.into(),
            amount,
        }
    }
}

/// Valida a invariante estrutural do ledger antes de persistir.
pub fn validate_postings(postings: &[Posting]) -> Result<(), &'static str> {
    if postings.len() < 2 {
        return Err("um lançamento exige pelo menos duas partidas");
    }

    let mut total = Money::ZERO;

    for posting in postings {
        if posting.amount.is_zero() {
            return Err("partidas não podem ter valor zero");
        }

        total = total
            .checked_add(posting.amount)
            .ok_or("o total das partidas excedeu o limite representável")?;
    }

    if !total.is_zero() {
        return Err("as partidas não somam zero");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Posting, validate_postings};
    use crate::money::Money;

    fn posting(id: &str, cents: i64) -> Posting {
        Posting::new(id, "account", Money::from_cents(cents))
    }

    #[test]
    fn accepts_two_postings_that_sum_zero() {
        assert!(validate_postings(&[posting("a", 100), posting("b", -100)]).is_ok());
    }

    #[test]
    fn rejects_fewer_than_two_postings() {
        assert!(validate_postings(&[posting("a", 0)]).is_err());
    }

    #[test]
    fn rejects_unbalanced_postings() {
        assert!(validate_postings(&[posting("a", 100), posting("b", -90)]).is_err());
    }

    #[test]
    fn rejects_zero_postings() {
        assert!(
            validate_postings(&[posting("a", 100), posting("b", -100), posting("c", 0)]).is_err()
        );
    }
}
