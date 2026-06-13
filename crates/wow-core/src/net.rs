use std::net::Ipv4Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4NetworkLikeCpp {
    address: Ipv4Addr,
    prefix: u8,
}

impl Ipv4NetworkLikeCpp {
    pub fn new(address: Ipv4Addr, prefix: u8) -> Self {
        Self {
            address,
            prefix: prefix.min(32),
        }
    }

    pub fn contains_like_cpp(&self, client_address: Ipv4Addr) -> bool {
        if client_address == self.address {
            return true;
        }

        let prefix = u32::from(self.prefix);
        let mask = if prefix == 0 {
            0
        } else {
            u32::MAX << (32 - prefix)
        };
        let network = u32::from(self.address) & mask;
        let client = u32::from(client_address) & mask;
        network == client
    }
}

fn is_in_local_network_like_cpp(address: Ipv4Addr, local_networks: &[Ipv4NetworkLikeCpp]) -> bool {
    local_networks
        .iter()
        .any(|network| network.contains_like_cpp(address))
}

/// IPv4 subset of Trinity::Net::SelectAddressForClient.
///
/// C++ classifies configured addresses as loopback, local-interface, or
/// external, then picks by client locality. The actual interface scan lives in
/// Trinity::Net::ScanLocalNetworks; callers provide that scanned network set.
pub fn select_ipv4_address_for_client_like_cpp(
    client_address: Ipv4Addr,
    addresses: &[Ipv4Addr],
    local_networks: &[Ipv4NetworkLikeCpp],
) -> Option<usize> {
    let mut loopback_index = None;
    let mut local_index = None;
    let mut external_index = None;

    for (index, address) in addresses.iter().copied().enumerate() {
        if address.is_loopback() {
            loopback_index.get_or_insert(index);
        } else if is_in_local_network_like_cpp(address, local_networks) {
            local_index.get_or_insert(index);
        } else {
            external_index.get_or_insert(index);
        }
    }

    if is_in_local_network_like_cpp(client_address, local_networks) || client_address.is_loopback()
    {
        if let Some(index) = local_index {
            return Some(index);
        }
    }

    if client_address.is_loopback() {
        if let Some(index) = loopback_index {
            return Some(index);
        }
    }

    external_index
}

pub fn realm_ipv4_address_for_client_like_cpp(
    client_address: Option<Ipv4Addr>,
    external_address: Ipv4Addr,
    local_address: Ipv4Addr,
    local_networks: &[Ipv4NetworkLikeCpp],
) -> Ipv4Addr {
    let Some(client_address) = client_address else {
        return external_address;
    };

    let addresses = [external_address, local_address];
    if let Some(index) =
        select_ipv4_address_for_client_like_cpp(client_address, &addresses, local_networks)
    {
        return addresses[index];
    }

    if client_address.is_loopback() {
        return local_address;
    }

    external_address
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_address_prefers_local_for_clients_in_scanned_local_network_like_cpp() {
        let external = Ipv4Addr::new(198, 51, 100, 10);
        let local = Ipv4Addr::new(10, 0, 4, 10);
        let local_networks = [Ipv4NetworkLikeCpp::new(Ipv4Addr::new(10, 0, 0, 1), 16)];

        assert_eq!(
            realm_ipv4_address_for_client_like_cpp(
                Some(Ipv4Addr::new(10, 0, 99, 42)),
                external,
                local,
                &local_networks,
            ),
            local
        );
    }

    #[test]
    fn select_address_prefers_external_for_non_local_clients_like_cpp() {
        let external = Ipv4Addr::new(198, 51, 100, 10);
        let local = Ipv4Addr::new(10, 0, 4, 10);
        let local_networks = [Ipv4NetworkLikeCpp::new(Ipv4Addr::new(10, 0, 0, 1), 16)];

        assert_eq!(
            realm_ipv4_address_for_client_like_cpp(
                Some(Ipv4Addr::new(203, 0, 113, 42)),
                external,
                local,
                &local_networks,
            ),
            external
        );
    }

    #[test]
    fn select_address_falls_back_to_loopback_for_loopback_client_like_cpp() {
        let external = Ipv4Addr::new(198, 51, 100, 10);
        let local = Ipv4Addr::new(127, 0, 0, 1);

        assert_eq!(
            realm_ipv4_address_for_client_like_cpp(
                Some(Ipv4Addr::new(127, 0, 0, 1)),
                external,
                local,
                &[],
            ),
            local
        );
    }
}
