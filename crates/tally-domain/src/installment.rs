use crate::money::Money;
use chrono::{Months, NaiveDate};

/// Cronograma exato de um parcelamento.
///
/// A soma das parcelas é sempre igual ao total da compra. O resto de centavos
/// fica nas primeiras parcelas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallmentSchedule {
    pub amounts: Vec<Money>,
    pub due_dates: Vec<NaiveDate>,
}

/// Divide um total positivo em `count` parcelas de centavos inteiros.
///
/// `count` fica entre 1 e 60 e não pode exceder o total em centavos, pois cada
/// parcela precisa ter pelo menos 1 centavo.
pub fn split_amount(total_cents: i64, count: u32) -> Result<Vec<Money>, &'static str> {
    if total_cents <= 0 {
        return Err("o total da compra deve ser maior que zero");
    }
    if !(1..=60).contains(&count) {
        return Err("a quantidade de parcelas deve ficar entre 1 e 60");
    }
    if i64::from(count) > total_cents {
        return Err("cada parcela precisa ter pelo menos um centavo");
    }

    let base = total_cents / i64::from(count);
    let remainder = total_cents % i64::from(count);

    Ok((0..count)
        .map(|index| {
            let extra = i64::from(index) < remainder;
            Money::from_cents(base + i64::from(extra))
        })
        .collect())
}

/// Datas mensais a partir da primeira, preservando o dia e limitando ao fim do
/// mês quando o dia não existe. Ex.: 31/01 → 28/02 → 31/03.
pub fn due_dates(first_due_on: NaiveDate, count: u32) -> Result<Vec<NaiveDate>, &'static str> {
    if !(1..=60).contains(&count) {
        return Err("a quantidade de parcelas deve ficar entre 1 e 60");
    }

    (0..count)
        .map(|index| {
            first_due_on
                .checked_add_months(Months::new(index))
                .ok_or("a data das parcelas excedeu o limite representável")
        })
        .collect()
}

pub fn build_schedule(
    total_cents: i64,
    count: u32,
    first_due_on: NaiveDate,
) -> Result<InstallmentSchedule, &'static str> {
    Ok(InstallmentSchedule {
        amounts: split_amount(total_cents, count)?,
        due_dates: due_dates(first_due_on, count)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_schedule, due_dates, split_amount};
    use chrono::NaiveDate;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
    }

    fn cents(values: &[i64]) -> Vec<i64> {
        values.to_vec()
    }

    #[test]
    fn splits_900_reais_into_three_equal_installments() {
        let amounts = split_amount(90_000, 3).expect("split");
        assert_eq!(
            amounts
                .iter()
                .map(|money| money.cents())
                .collect::<Vec<_>>(),
            cents(&[30_000, 30_000, 30_000])
        );
    }

    #[test]
    fn puts_the_remainder_in_the_first_installments() {
        let amounts = split_amount(10_000, 3).expect("split");
        assert_eq!(
            amounts
                .iter()
                .map(|money| money.cents())
                .collect::<Vec<_>>(),
            cents(&[3_334, 3_333, 3_333])
        );
        assert_eq!(
            amounts.iter().map(|money| money.cents()).sum::<i64>(),
            10_000
        );
    }

    #[test]
    fn supports_single_and_sixty_installments() {
        assert_eq!(split_amount(1, 1).expect("one").len(), 1);
        let amounts = split_amount(60, 60).expect("sixty");
        assert!(amounts.iter().all(|money| money.cents() == 1));
    }

    #[test]
    fn rejects_invalid_splits() {
        assert!(split_amount(0, 1).is_err());
        assert!(split_amount(100, 0).is_err());
        assert!(split_amount(100, 61).is_err());
        assert!(split_amount(2, 3).is_err());
    }

    #[test]
    fn clamps_month_end_and_returns_to_the_original_day() {
        let dates = due_dates(date(2025, 1, 31), 4).expect("dates");
        assert_eq!(
            dates,
            vec![
                date(2025, 1, 31),
                date(2025, 2, 28),
                date(2025, 3, 31),
                date(2025, 4, 30),
            ]
        );
    }

    #[test]
    fn handles_leap_year_february() {
        let dates = due_dates(date(2024, 1, 31), 2).expect("dates");
        assert_eq!(dates, vec![date(2024, 1, 31), date(2024, 2, 29)]);
    }

    #[test]
    fn builds_amounts_and_dates_together() {
        let schedule = build_schedule(90_000, 3, date(2026, 1, 31)).expect("schedule");
        assert_eq!(schedule.amounts.len(), 3);
        assert_eq!(schedule.due_dates.len(), 3);
        assert_eq!(schedule.due_dates[1], date(2026, 2, 28));
    }
}
