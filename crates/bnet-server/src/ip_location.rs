//! TrinityCore-compatible IP location range store.

use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpLocationRecord {
    pub ip_from: u32,
    pub ip_to: u32,
    pub country_code: String,
    pub country_name: String,
}

#[derive(Debug, Clone, Default)]
pub struct IpLocationStore {
    records: Vec<IpLocationRecord>,
}

impl IpLocationStore {
    pub fn from_csv_like_cpp(contents: &str) -> Self {
        let mut records = Vec::new();

        for line in contents.lines() {
            let mut fields = line.splitn(4, ',').map(strip_cpp_csv_field_like_cpp);
            let Some(ip_from) = fields.next() else {
                continue;
            };
            let Some(ip_to) = fields.next() else {
                continue;
            };
            let Some(country_code) = fields.next() else {
                continue;
            };
            let Some(country_name) = fields.next() else {
                continue;
            };

            let Ok(ip_from) = ip_from.parse::<u32>() else {
                continue;
            };
            let Ok(ip_to) = ip_to.parse::<u32>() else {
                continue;
            };

            records.push(IpLocationRecord {
                ip_from,
                ip_to,
                country_code: country_code.to_ascii_lowercase(),
                country_name,
            });
        }

        records.sort_by_key(|record| record.ip_from);
        Self { records }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn country_for_ip_like_cpp(&self, ip_address: &str) -> Option<&str> {
        let ip = match ip_address.parse::<IpAddr>().ok()? {
            IpAddr::V4(address) => u32::from(address),
            IpAddr::V6(_) => return None,
        };

        self.country_for_ipv4_value_like_cpp(ip)
    }

    #[cfg(test)]
    pub fn country_for_ipv4_like_cpp(&self, ip_address: std::net::Ipv4Addr) -> Option<&str> {
        self.country_for_ipv4_value_like_cpp(u32::from(ip_address))
    }

    fn country_for_ipv4_value_like_cpp(&self, ip: u32) -> Option<&str> {
        let index = self.records.partition_point(|record| ip >= record.ip_to);
        let record = self.records.get(index)?;
        if ip < record.ip_from {
            return None;
        }

        Some(record.country_code.as_str())
    }
}

fn strip_cpp_csv_field_like_cpp(field: &str) -> String {
    field
        .trim_end_matches(['\r', '\n'])
        .chars()
        .filter(|c| *c != '"')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::IpLocationStore;
    use std::net::Ipv4Addr;

    #[test]
    fn ip_location_loader_strips_quotes_and_lowercases_country_like_cpp() {
        let store = IpLocationStore::from_csv_like_cpp(
            "\"167772160\",\"167772416\",\"US\",\"United States\"\r\nbad,row\n",
        );

        assert_eq!(store.len(), 1);
        assert_eq!(
            store.country_for_ipv4_like_cpp(Ipv4Addr::new(10, 0, 0, 1)),
            Some("us")
        );
    }

    #[test]
    fn ip_location_lookup_uses_cpp_half_open_upper_bound_semantics() {
        let store = IpLocationStore::from_csv_like_cpp(
            "\"167772160\",\"167772416\",\"US\",\"United States\"\n\
             \"167772416\",\"167772672\",\"CA\",\"Canada\"\n",
        );

        assert_eq!(
            store.country_for_ipv4_like_cpp(Ipv4Addr::new(10, 0, 0, 0)),
            Some("us")
        );
        assert_eq!(
            store.country_for_ipv4_like_cpp(Ipv4Addr::new(10, 0, 0, 255)),
            Some("us")
        );
        assert_eq!(
            store.country_for_ipv4_like_cpp(Ipv4Addr::new(10, 0, 1, 0)),
            Some("ca")
        );
    }

    #[test]
    fn ip_location_lookup_rejects_non_ipv4_like_cpp() {
        let store = IpLocationStore::from_csv_like_cpp(
            "\"167772160\",\"167772416\",\"US\",\"United States\"\n",
        );

        assert_eq!(store.country_for_ip_like_cpp("not-an-ip"), None);
        assert_eq!(store.country_for_ip_like_cpp("::1"), None);
    }
}
