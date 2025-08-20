//! xx
//! version: 0.1.0

use get_if_addrs::{IfAddr, get_if_addrs};
use std::fmt;
use std::net::Ipv4Addr;

/*
   some notes
       Ipv4Addr::LOCALHOST == 127.0.0.1

       Ipv4Addr::Unspecified == 0.0.0.0
            - also known as `INADDR_ANY`
            - can be binded as local address
            - means to accept all traffic
            - cannot be used as remote
*/

#[derive(Debug)]
pub struct Netif {
    pub name: String,
    pub ip: Ipv4Addr,
    pub bc: Option<Ipv4Addr>,
}

impl fmt::Display for Netif {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(bc) = self.bc {
            write!(f, "{} - {} ( bc = {} )", self.name, self.ip, bc)
        } else {
            write!(f, "{} - {}", self.name, self.ip)
        }
    }
}

impl Netif {
    /// generate a ip address prefix for neighbours
    /// on the network, & operation with mask is just
    /// too much over head, we will return the first
    /// octets for now
    pub fn remote_ip_template(&self) -> String {
        if self.ip.is_loopback() {
            return self.ip.to_string();
        }

        if self.ip.is_unspecified() {
            return Ipv4Addr::LOCALHOST.to_string();
        }

        let octs = self.ip.octets();
        format!("{}.{}.{}.", octs[0], octs[1], octs[2])
    }

    /// netdev approach
    /// ### if_physical
    /// does not work for interfaces like awdl, llw and anpi
    /// even all of them has no valid ipv4 address, and most
    /// of them are is_up == true
    /// utun (VM) is recogonized as if_physical == false
    /// localhost is recogonized as if_physical == false
    pub fn get_local_netif() -> Vec<Netif> {
        let mut res = vec![];
        let ifaces = netdev::get_interfaces();

        for iface in ifaces {
            if !iface.ipv4.is_empty() {
                let ipv4 = iface.ipv4[0];

                // this is to filter utun4 on mac which is considered not physical
                // notice localhost is also not physical which we'd like to keep
                if !ipv4.addr.is_loopback() && !iface.is_physical() || !iface.is_up() {
                    continue;
                }

                // test code
                // dbg!(&iface.name);  // works for macos
                // dbg!(&iface.friendly_name); // works for windows but it is an option
                let name = {
                    #[cfg(target_os = "windows")]
                    {
                        match iface.friendly_name {
                            Some(n) => n,
                            None => iface.name,
                        }
                    }

                    #[cfg(not(target_os = "windows"))]
                    {
                        iface.name
                    }
                };

                res.push(Netif {
                    name,
                    ip: ipv4.addr,
                    bc: if ipv4.addr.is_loopback() {
                        None
                    } else {
                        Some(ipv4.broadcast())
                    },
                });
            }
        }

        // manually insert the INADDR_ANY (0.0.0.0)
        res.push(Netif {
            name: "INADDR_ANY".to_string(),
            ip: Ipv4Addr::UNSPECIFIED,
            bc: None,
        });
        res
    }

    /// get_if_addrs version
    /// netif names are GUIDs on windows (ok on mac) which is not friendly
    /// return empty vec in case of failure retrive
    #[allow(unused)]
    #[deprecated = "old approach"]
    fn get_local_netif_old1() -> Vec<Netif> {
        get_if_addrs()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|iface| {
                println!("iface = {:?}", iface);
                if let IfAddr::V4(ipv4_addr) = iface.addr {
                    let iface_name = iface.name.to_lowercase();

                    // Skip virtual interfaces
                    if Self::is_likely_virtual(&iface_name) {
                        return None;
                    }

                    Some(Netif {
                        name: iface_name,
                        ip: ipv4_addr.ip,
                        bc: ipv4_addr.broadcast,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// to be used with get_if_addr approach
    /// check if netif is virtual using some common known names
    /// tested working for mac, not sure windows
    fn is_likely_virtual(name: &str) -> bool {
        name.starts_with("veth") ||   // virtual ethernet 
        name.starts_with("docker") || // docker
        name.starts_with("vmnet") ||  // VMware
        name.starts_with("vbox") ||   // VirtualBox
        name.starts_with("utun") ||   // macOS/BSD tunnel
        name.starts_with("tun") ||    // tunnel
        name.starts_with("tap") ||    // TAP interface
        name.starts_with("ipsec") // IPsec tunnel
    }

    /// netdev approach
    /// the friendly_name is not working well for mac
    /// no name info for localhost, the name en0 is now "WI-F"
    #[allow(unused)]
    #[deprecated = "old approach using netdev"]
    fn get_local_netif_old2() -> Vec<Netif> {
        let mut res = vec![];
        let ifaces = netdev::get_interfaces();

        for iface in ifaces {
            if iface.is_loopback() || (iface.is_physical() && iface.is_running()) {
                let num_ipv4s = iface.ipv4.len();
                if num_ipv4s == 0 {
                    continue;
                } else {
                    if num_ipv4s > 1 {
                        println!(
                            "iface {:?} has multiple addresses:\n{:?}",
                            iface, iface.ipv4
                        );
                    }

                    let ipv4net = iface.ipv4[0];
                    let bc = if iface.is_broadcast() {
                        Some(ipv4net.broadcast())
                    } else {
                        None
                    };

                    let netif = Netif {
                        name: iface.name,
                        ip: ipv4net.addr,
                        bc,
                    };
                    res.push(netif);
                }
            }
        }
        res
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_netif() {
        let _netifs = Netif::get_local_netif();
    }
}
