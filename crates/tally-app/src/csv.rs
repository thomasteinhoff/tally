use crate::dto::MovementDto;

/// Gera o CSV de movimentações com BOM UTF-8 e separador `;`, formato que o
/// Excel em português abre sem reclamar de acentos.
pub fn movements_csv(movements: &[MovementDto]) -> String {
    let mut output = String::from(
        "\u{feff}Data;Hora;Periodo;Tipo;Descricao;Conta;Contraparte;Categoria;Forma de pagamento;Valor (R$);Centavos;ID\r\n",
    );

    for movement in movements {
        let description = movement.description.as_deref().unwrap_or("");
        let counterparty = movement.counterparty_account_name.as_deref().unwrap_or("");
        let category = movement.category_name.as_deref().unwrap_or("");
        let payment = movement.payment_method.as_deref().unwrap_or("");
        let period = movement.period.as_deref().unwrap_or("");

        let fields = [
            movement.occurred_on.clone(),
            movement.occurred_time.clone().unwrap_or_default(),
            period.to_owned(),
            kind_label(&movement.kind).to_owned(),
            description.to_owned(),
            movement.primary_account_name.clone(),
            counterparty.to_owned(),
            category.to_owned(),
            payment.to_owned(),
            format_brl(movement.amount_cents),
            movement.amount_cents.to_string(),
            movement.id.clone(),
        ];

        output.push_str(
            &fields
                .iter()
                .map(|field| escape(field))
                .collect::<Vec<_>>()
                .join(";"),
        );
        output.push_str("\r\n");
    }

    output
}

/// Rótulos estáveis no arquivo, independentes do texto da interface.
fn kind_label(kind: &str) -> &str {
    match kind {
        "savings_yield" => "Rendimento de cofrinho",
        "savings_yield_reversal" => "Estorno de rendimento",
        other => other,
    }
}

fn escape(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ';' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn format_brl(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let absolute = cents.unsigned_abs();
    let reais = (absolute / 100).to_string();
    let decimals = absolute % 100;

    let mut grouped = String::with_capacity(reais.len() + reais.len() / 3);
    for (index, digit) in reais.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(digit);
    }
    let grouped: String = grouped.chars().rev().collect();

    format!("{sign}R$ {grouped},{decimals:02}")
}

#[cfg(test)]
mod tests {
    use super::movements_csv;
    use crate::dto::MovementDto;

    fn movement(description: &str) -> MovementDto {
        MovementDto {
            id: "entry-1".to_owned(),
            kind: "expense".to_owned(),
            occurred_on: "2026-11-03".to_owned(),
            occurred_time: Some("12:05".to_owned()),
            manual_period: None,
            period: Some("tarde".to_owned()),
            description: Some(description.to_owned()),
            primary_account_id: "account-1".to_owned(),
            primary_account_name: "Banco".to_owned(),
            primary_account_is_savings: false,
            counterparty_account_id: None,
            counterparty_account_name: None,
            counterparty_account_is_savings: false,
            category_id: Some("category-1".to_owned()),
            category_name: Some("Mercado".to_owned()),
            payment_method: Some("pix".to_owned()),
            amount_cents: -123_456,
            created_at: "2026-11-03T12:05:00Z".to_owned(),
            installment_plan_id: None,
            installment_count: None,
            first_installment_cents: None,
        }
    }

    #[test]
    fn writes_bom_header_and_escapes_semicolons_quotes_and_newlines() {
        let csv = movements_csv(&[movement("Mercado; \"extra\"\nsegunda linha")]);

        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.contains("Data;Hora;Periodo;Tipo;Descricao;Conta"));
        assert!(csv.contains("\"Mercado; \"\"extra\"\"\nsegunda linha\""));
        assert!(csv.contains("-R$ 1.234,56;-123456;entry-1"));
        assert!(csv.ends_with("\r\n"));
    }

    #[test]
    fn uses_stable_labels_for_savings_kinds() {
        let mut yield_movement = movement("Rendimento");
        yield_movement.kind = "savings_yield".to_owned();
        let mut reversal = movement("Estorno");
        reversal.kind = "savings_yield_reversal".to_owned();
        reversal.id = "entry-2".to_owned();

        let csv = movements_csv(&[yield_movement, reversal]);

        assert!(csv.contains(";Rendimento de cofrinho;"));
        assert!(csv.contains(";Estorno de rendimento;"));
    }
}
