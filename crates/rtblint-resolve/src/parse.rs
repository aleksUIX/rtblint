//! sellers.json and ads.txt / app-ads.txt parsers.

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SellersJson {
    pub(crate) seller_ids: Vec<String>,
}

impl SellersJson {
    pub(crate) fn contains_sid(&self, sid: &str) -> bool {
        self.seller_ids.iter().any(|id| id == sid)
    }
}

#[derive(Deserialize)]
struct SellersDocument {
    #[serde(default)]
    sellers: Vec<Seller>,
}

#[derive(Deserialize)]
struct Seller {
    seller_id: SellerId,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SellerId {
    String(String),
    Signed(i64),
    Unsigned(u64),
}

impl SellerId {
    fn as_key(&self) -> String {
        match self {
            Self::String(value) => value.clone(),
            Self::Signed(value) => value.to_string(),
            Self::Unsigned(value) => value.to_string(),
        }
    }
}

pub(crate) fn parse_sellers_json(raw: &str) -> Result<SellersJson, String> {
    let document: SellersDocument = serde_json::from_str(raw)
        .map_err(|error| format!("sellers.json is not usable: {error}"))?;
    Ok(SellersJson {
        seller_ids: document
            .sellers
            .into_iter()
            .map(|seller| seller.seller_id.as_key())
            .collect(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdsTxt {
    pub(crate) records: Vec<AdsRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdsRecord {
    pub(crate) domain: String,
    pub(crate) account_id: String,
}

impl AdsTxt {
    pub(crate) fn authorizes(&self, asi: &str, sid: &str) -> bool {
        let asi = asi.to_ascii_lowercase();
        self.records
            .iter()
            .any(|record| record.domain == asi && record.account_id == sid)
    }
}

pub(crate) fn parse_ads_txt(raw: &str) -> AdsTxt {
    let mut records = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !line.contains(',') {
            continue;
        }
        let parts: Vec<&str> = line.split(',').map(str::trim).collect();
        if parts.len() < 3 {
            continue;
        }
        let relationship = parts[2].to_ascii_uppercase();
        if relationship != "DIRECT" && relationship != "RESELLER" {
            continue;
        }
        if parts[0].is_empty() || parts[1].is_empty() {
            continue;
        }
        records.push(AdsRecord {
            domain: parts[0].to_ascii_lowercase(),
            account_id: String::from(parts[1]),
        });
    }
    AdsTxt { records }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sellers_json_accepts_string_and_numeric_ids() {
        let parsed =
            parse_sellers_json(r#"{ "sellers": [ { "seller_id": "abc" }, { "seller_id": 42 } ] }"#)
                .expect("parse");
        assert!(parsed.contains_sid("abc"));
        assert!(parsed.contains_sid("42"));
        assert!(!parsed.contains_sid("missing"));
    }

    #[test]
    fn ads_txt_skips_comments_variables_and_non_relationship_rows() {
        let parsed = parse_ads_txt(
            "# comment\n\
             CONTACT=ads@publisher.example\n\
             exchange.example, seller-1, DIRECT\n\
             other.example, seller-2, reseller\n\
             inventorypartnerdomain=partner.example\n\
             broken, only-two\n",
        );
        assert_eq!(parsed.records.len(), 2);
        assert!(parsed.authorizes("Exchange.example", "seller-1"));
        assert!(parsed.authorizes("other.example", "seller-2"));
        assert!(!parsed.authorizes("exchange.example", "seller-2"));
    }
}
