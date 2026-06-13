use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpNetworkLikeCpp {
    address: IpAddr,
    prefix: u8,
}

impl IpNetworkLikeCpp {
    pub fn new(address: IpAddr, prefix: u8) -> Self {
        let max_prefix = match address {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        Self {
            address,
            prefix: prefix.min(max_prefix),
        }
    }

    pub fn contains_like_cpp(&self, client_address: IpAddr) -> bool {
        match (self.address, client_address) {
            (IpAddr::V4(network), IpAddr::V4(client)) => {
                Ipv4NetworkLikeCpp::new(network, self.prefix).contains_like_cpp(client)
            }
            (IpAddr::V6(network), IpAddr::V6(client)) => {
                if client == network {
                    return true;
                }
                let prefix = u32::from(self.prefix);
                let mask = if prefix == 0 {
                    0
                } else {
                    u128::MAX << (128 - prefix)
                };
                (u128::from(network) & mask) == (u128::from(client) & mask)
            }
            _ => false,
        }
    }
}

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

fn is_ip_in_local_network_like_cpp(address: IpAddr, local_networks: &[IpNetworkLikeCpp]) -> bool {
    local_networks
        .iter()
        .any(|network| network.contains_like_cpp(address))
}

fn ipv4_prefix_from_netmask_like_cpp(netmask: Ipv4Addr) -> u8 {
    netmask
        .octets()
        .iter()
        .map(|octet| octet.leading_ones() as u8)
        .sum()
}

fn ipv6_prefix_from_netmask_like_cpp(netmask: Ipv6Addr) -> u8 {
    netmask
        .octets()
        .iter()
        .map(|octet| octet.leading_ones() as u8)
        .sum()
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn sockaddr_ipv4_like_cpp(sockaddr: *const libc::sockaddr) -> Option<Ipv4Addr> {
    if sockaddr.is_null() {
        return None;
    }

    // SAFETY: caller passes a non-null sockaddr pointer from getifaddrs. The
    // family check ensures the layout is sockaddr_in before the cast.
    unsafe {
        if (*sockaddr).sa_family as libc::c_int != libc::AF_INET {
            return None;
        }
        let ipv4 = &*(sockaddr as *const libc::sockaddr_in);
        Some(Ipv4Addr::from(ipv4.sin_addr.s_addr.to_ne_bytes()))
    }
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn sockaddr_ipv6_like_cpp(sockaddr: *const libc::sockaddr) -> Option<Ipv6Addr> {
    if sockaddr.is_null() {
        return None;
    }

    // SAFETY: caller passes a non-null sockaddr pointer from getifaddrs. The
    // family check ensures the layout is sockaddr_in6 before the cast.
    unsafe {
        if (*sockaddr).sa_family as libc::c_int != libc::AF_INET6 {
            return None;
        }
        let ipv6 = &*(sockaddr as *const libc::sockaddr_in6);
        Some(Ipv6Addr::from(ipv6.sin6_addr.s6_addr))
    }
}

#[cfg(unix)]
#[allow(unsafe_code)]
pub fn scan_local_ipv4_networks_like_cpp() -> Vec<Ipv4NetworkLikeCpp> {
    scan_local_ip_networks_like_cpp()
        .into_iter()
        .filter_map(|network| match network.address {
            IpAddr::V4(address) => Some(Ipv4NetworkLikeCpp::new(address, network.prefix)),
            IpAddr::V6(_) => None,
        })
        .collect()
}

#[cfg(unix)]
#[allow(unsafe_code)]
pub fn scan_local_ip_networks_like_cpp() -> Vec<IpNetworkLikeCpp> {
    let mut networks = Vec::new();
    let mut addresses: *mut libc::ifaddrs = std::ptr::null_mut();

    // SAFETY: getifaddrs initializes a linked list owned by libc on success;
    // every successful call is paired with freeifaddrs before returning.
    let result = unsafe { libc::getifaddrs(&mut addresses) };
    if result == -1 {
        return networks;
    }

    let mut current = addresses;
    while !current.is_null() {
        // SAFETY: current traverses the getifaddrs-owned linked list until null.
        let ifaddr = unsafe { &*current };
        if let Some(address) = sockaddr_ipv4_like_cpp(ifaddr.ifa_addr) {
            if !(address.is_unspecified()
                || address.is_loopback()
                || address.is_multicast()
                || address == Ipv4Addr::BROADCAST)
            {
                let prefix = sockaddr_ipv4_like_cpp(ifaddr.ifa_netmask)
                    .map(ipv4_prefix_from_netmask_like_cpp)
                    .unwrap_or(32);
                networks.push(IpNetworkLikeCpp::new(IpAddr::V4(address), prefix));
            }
        }
        if let Some(address) = sockaddr_ipv6_like_cpp(ifaddr.ifa_addr) {
            if !(address.is_unspecified() || address.is_loopback() || address.is_multicast()) {
                let prefix = sockaddr_ipv6_like_cpp(ifaddr.ifa_netmask)
                    .map(ipv6_prefix_from_netmask_like_cpp)
                    .unwrap_or(128);
                networks.push(IpNetworkLikeCpp::new(IpAddr::V6(address), prefix));
            }
        }

        current = ifaddr.ifa_next;
    }

    // SAFETY: addresses was returned by a successful getifaddrs call above.
    unsafe {
        libc::freeifaddrs(addresses);
    }
    networks
}

#[cfg(not(unix))]
pub fn scan_local_ipv4_networks_like_cpp() -> Vec<Ipv4NetworkLikeCpp> {
    Vec::new()
}

#[cfg(not(unix))]
pub fn scan_local_ip_networks_like_cpp() -> Vec<IpNetworkLikeCpp> {
    Vec::new()
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

pub fn select_ip_address_for_client_like_cpp(
    client_address: IpAddr,
    addresses: &[IpAddr],
    local_networks: &[IpNetworkLikeCpp],
) -> Option<usize> {
    let mut local_ipv6_index = None;
    let mut external_ipv6_index = None;
    let mut loopback_ipv6_index = None;
    let mut local_ipv4_index = None;
    let mut external_ipv4_index = None;
    let mut loopback_ipv4_index = None;

    for (index, address) in addresses.iter().copied().enumerate() {
        if address.is_loopback() {
            match address {
                IpAddr::V6(_) => {
                    loopback_ipv6_index.get_or_insert(index);
                }
                IpAddr::V4(_) => {
                    loopback_ipv4_index.get_or_insert(index);
                }
            }
        } else if is_ip_in_local_network_like_cpp(address, local_networks) {
            match address {
                IpAddr::V6(_) => {
                    local_ipv6_index.get_or_insert(index);
                }
                IpAddr::V4(_) => {
                    local_ipv4_index.get_or_insert(index);
                }
            }
        } else {
            match address {
                IpAddr::V6(_) => {
                    external_ipv6_index.get_or_insert(index);
                }
                IpAddr::V4(_) => {
                    external_ipv4_index.get_or_insert(index);
                }
            }
        }
    }

    if is_ip_in_local_network_like_cpp(client_address, local_networks)
        || client_address.is_loopback()
    {
        if matches!(client_address, IpAddr::V6(_)) {
            if let Some(index) = local_ipv6_index {
                return Some(index);
            }
        }

        if let Some(index) = local_ipv4_index {
            return Some(index);
        }
    }

    if client_address.is_loopback() {
        if matches!(client_address, IpAddr::V6(_)) {
            if let Some(index) = loopback_ipv6_index {
                return Some(index);
            }
        }

        if let Some(index) = loopback_ipv4_index {
            return Some(index);
        }
    }

    if matches!(client_address, IpAddr::V6(_)) {
        if let Some(index) = external_ipv6_index {
            return Some(index);
        }
    }

    external_ipv4_index
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

    #[test]
    fn ipv4_prefix_from_netmask_counts_leading_one_bits_like_cpp() {
        assert_eq!(
            ipv4_prefix_from_netmask_like_cpp(Ipv4Addr::new(255, 255, 0, 0)),
            16
        );
        assert_eq!(
            ipv4_prefix_from_netmask_like_cpp(Ipv4Addr::new(255, 255, 254, 0)),
            23
        );
        assert_eq!(
            ipv4_prefix_from_netmask_like_cpp(Ipv4Addr::new(255, 255, 255, 255)),
            32
        );
    }

    #[test]
    fn select_ip_prefers_local_ipv6_for_local_ipv6_client_like_cpp() {
        let addresses = [
            IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 10)),
            IpAddr::V6("2001:db8::10".parse().unwrap()),
            IpAddr::V6("fd00::10".parse().unwrap()),
        ];
        let local_networks = [
            IpNetworkLikeCpp::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 24),
            IpNetworkLikeCpp::new("fd00::1".parse().unwrap(), 64),
        ];

        assert_eq!(
            select_ip_address_for_client_like_cpp(
                "fd00::42".parse().unwrap(),
                &addresses,
                &local_networks
            ),
            Some(3)
        );
    }

    #[test]
    fn select_ip_falls_back_to_local_ipv4_for_local_ipv6_client_like_cpp() {
        let addresses = [
            IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 10)),
            IpAddr::V6("2001:db8::10".parse().unwrap()),
        ];
        let local_networks = [
            IpNetworkLikeCpp::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 24),
            IpNetworkLikeCpp::new("fd00::1".parse().unwrap(), 64),
        ];

        assert_eq!(
            select_ip_address_for_client_like_cpp(
                "fd00::42".parse().unwrap(),
                &addresses,
                &local_networks
            ),
            Some(1)
        );
    }

    #[test]
    fn select_ip_prefers_loopback_ipv6_for_loopback_ipv6_client_like_cpp() {
        let addresses = [
            IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10)),
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
        ];

        assert_eq!(
            select_ip_address_for_client_like_cpp(IpAddr::V6(Ipv6Addr::LOCALHOST), &addresses, &[]),
            Some(2)
        );
    }

    #[test]
    fn select_ip_prefers_external_ipv6_for_external_ipv6_client_like_cpp() {
        let addresses = [
            IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 10)),
            IpAddr::V6("2001:db8::10".parse().unwrap()),
        ];
        let local_networks = [IpNetworkLikeCpp::new(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            24,
        )];

        assert_eq!(
            select_ip_address_for_client_like_cpp(
                "2001:db8:ffff::42".parse().unwrap(),
                &addresses,
                &local_networks
            ),
            Some(2)
        );
    }
}
