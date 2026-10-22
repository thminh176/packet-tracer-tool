//! Normalization, validation and interpretation layer for network values,
//! CIDR notation, subnet masks, wildcard masks, static routes and OSPF statements.
//!
//! Handles Packet Tracer-specific encodings (such as `100.0.0.0-8` or
//! `172.16.128.0-17-172.18.192.1-0-1`), dotted-decimal masks, wildcard masks,
//! and standard CIDR formats while preserving raw values and reporting
//! normalization confidence and diagnostic explanations.

use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};

/// Confidence in the normalization of a network value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    /// Explicit, unambiguous notation (e.g. `100.0.0.0/8` or `100.0.0.0 255.0.0.0`).
    High,
    /// Inferred or sanitized representation (e.g. `100.0.0.0-8` where host bits are zero).
    Medium,
    /// Ambiguous encoding or normalized with host-bit truncation.
    Low,
}

/// Normalization summary for an IPv4 network, subnet and wildcard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedNetwork {
    /// The original raw input string.
    pub raw: String,
    /// Canonical network address (host bits zeroed).
    pub network: Ipv4Addr,
    /// CIDR prefix length in range `0..=32`.
    pub prefix: u8,
    /// Contiguous dotted-decimal subnet mask.
    pub subnet_mask: Ipv4Addr,
    /// Inverted wildcard mask corresponding to the subnet mask.
    pub wildcard_mask: Ipv4Addr,
    /// Normalization confidence.
    pub confidence: Confidence,
    /// Human-readable explanation of how the value was normalized.
    pub explanation: String,
}

impl NormalizedNetwork {
    /// Return the canonical CIDR notation (e.g. `100.0.0.0/8`).
    pub fn cidr(&self) -> String {
        format!("{}/{}", self.network, self.prefix)
    }

    /// Return the network address and dotted subnet mask (e.g. `100.0.0.0 255.0.0.0`).
    pub fn network_and_mask(&self) -> String {
        format!("{} {}", self.network, self.subnet_mask)
    }

    /// Return the network address and wildcard mask (e.g. `100.0.0.0 0.255.255.255`).
    pub fn network_and_wildcard(&self) -> String {
        format!("{} {}", self.network, self.wildcard_mask)
    }

    /// Normalize an IPv4 address and prefix length into a `NormalizedNetwork`.
    pub fn from_ip_and_prefix(ip: Ipv4Addr, prefix: u8, raw: String) -> Option<Self> {
        if prefix > 32 {
            return None;
        }
        let subnet_mask = prefix_to_subnet(prefix)?;
        let wildcard_mask = prefix_to_wildcard(prefix)?;
        let ip_u32 = u32::from(ip);
        let mask_u32 = u32::from(subnet_mask);
        let net_u32 = ip_u32 & mask_u32;
        let network = Ipv4Addr::from(net_u32);

        let had_host_bits = ip_u32 != net_u32;
        let confidence = if had_host_bits {
            Confidence::Medium
        } else if raw.contains('/') || raw.contains(' ') {
            Confidence::High
        } else {
            Confidence::Medium
        };

        let explanation = if had_host_bits {
            format!("Host address {ip} clamped to network address {network} with prefix /{prefix}")
        } else {
            format!("Valid network {network}/{prefix} (mask {subnet_mask}, wildcard {wildcard_mask})")
        };

        Some(Self {
            raw,
            network,
            prefix,
            subnet_mask,
            wildcard_mask,
            confidence,
            explanation,
        })
    }

    /// Parse a network string in any supported format:
    /// - `100.0.0.0/8` (CIDR)
    /// - `100.0.0.0-8` (Packet Tracer hyphenated prefix)
    /// - `100.0.0.0 255.0.0.0` (Dotted subnet mask)
    /// - `100.0.0.0 0.255.255.255` (Wildcard mask)
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() || is_sentinel(trimmed) {
            return None;
        }

        // Case 1: CIDR slash notation "100.0.0.0/8"
        if let Some((addr_part, prefix_part)) = trimmed.split_once('/') {
            let addr: Ipv4Addr = addr_part.trim().parse().ok()?;
            let prefix: u8 = prefix_part.trim().parse().ok()?;
            return Self::from_ip_and_prefix(addr, prefix, trimmed.to_string());
        }

        // Case 2: Packet Tracer hyphenated notation "100.0.0.0-8"
        // Note: Can also appear as "100.0.0.0-8-..." in static routes; if extra parts exist,
        // take the first two segments.
        if let Some((addr_part, rest)) = trimmed.split_once('-') {
            if let Ok(addr) = addr_part.trim().parse::<Ipv4Addr>() {
                // If rest starts with a valid prefix 0..=32
                let prefix_candidate = rest.split('-').next().unwrap_or(rest).trim();
                if let Ok(prefix) = prefix_candidate.parse::<u8>() {
                    if prefix <= 32 {
                        let mut norm = Self::from_ip_and_prefix(addr, prefix, trimmed.to_string())?;
                        norm.explanation = format!(
                            "Packet Tracer hyphenated encoding normalized to {}/{} (mask {}, wildcard {})",
                            norm.network, norm.prefix, norm.subnet_mask, norm.wildcard_mask
                        );
                        return Some(norm);
                    }
                }
            }
        }

        // Case 3: Space-separated "100.0.0.0 255.0.0.0" or "100.0.0.0 0.255.255.255"
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.len() == 2 {
            let addr: Ipv4Addr = tokens[0].parse().ok()?;
            if let Ok(mask) = tokens[1].parse::<Ipv4Addr>() {
                if is_contiguous_mask(mask) {
                    let prefix = subnet_to_prefix(mask)?;
                    return Self::from_ip_and_prefix(addr, prefix, trimmed.to_string());
                } else if is_contiguous_wildcard(mask) {
                    let prefix = wildcard_to_prefix(mask)?;
                    return Self::from_ip_and_prefix(addr, prefix, trimmed.to_string());
                }
            }
        }

        // Case 4: Plain IP without mask - determine default classful prefix if requested,
        // but here report classful with Medium confidence.
        if let Ok(addr) = trimmed.parse::<Ipv4Addr>() {
            let prefix = classful_prefix(addr);
            let mut norm = Self::from_ip_and_prefix(addr, prefix, trimmed.to_string())?;
            norm.confidence = Confidence::Medium;
            norm.explanation = format!(
                "Classful default /{prefix} inferred for IPv4 address {addr} (mask {})",
                norm.subnet_mask
            );
            return Some(norm);
        }

        None
    }
}

/// Convert a CIDR prefix (0..=32) to contiguous IPv4 subnet mask.
pub fn prefix_to_subnet(prefix: u8) -> Option<Ipv4Addr> {
    if prefix > 32 {
        return None;
    }
    if prefix == 0 {
        return Some(Ipv4Addr::new(0, 0, 0, 0));
    }
    let mask_u32 = u32::MAX << (32 - prefix);
    Some(Ipv4Addr::from(mask_u32))
}

/// Convert a CIDR prefix (0..=32) to IPv4 wildcard mask.
pub fn prefix_to_wildcard(prefix: u8) -> Option<Ipv4Addr> {
    if prefix > 32 {
        return None;
    }
    if prefix == 0 {
        return Some(Ipv4Addr::new(255, 255, 255, 255));
    }
    let mask = prefix_to_subnet(prefix)?;
    Some(Ipv4Addr::from(!u32::from(mask)))
}

/// Check if a subnet mask is contiguous.
pub fn is_contiguous_mask(mask: Ipv4Addr) -> bool {
    let n = u32::from(mask);
    if n == 0 {
        return true;
    }
    let inverse = !n;
    inverse.wrapping_add(1) & inverse == 0
}

/// Check if a wildcard mask is contiguous.
pub fn is_contiguous_wildcard(wildcard: Ipv4Addr) -> bool {
    let n = u32::from(wildcard);
    if n == u32::MAX {
        return true;
    }
    n.wrapping_add(1) & n == 0
}

/// Convert contiguous subnet mask to prefix length (0..=32).
pub fn subnet_to_prefix(mask: Ipv4Addr) -> Option<u8> {
    if !is_contiguous_mask(mask) {
        return None;
    }
    Some(u32::from(mask).count_ones() as u8)
}

/// Convert contiguous wildcard mask to prefix length (0..=32).
pub fn wildcard_to_prefix(wildcard: Ipv4Addr) -> Option<u8> {
    if !is_contiguous_wildcard(wildcard) {
        return None;
    }
    Some((32 - u32::from(wildcard).count_ones()) as u8)
}

/// Invert a contiguous subnet mask to wildcard mask.
pub fn subnet_to_wildcard(mask: Ipv4Addr) -> Option<Ipv4Addr> {
    if !is_contiguous_mask(mask) {
        return None;
    }
    Some(Ipv4Addr::from(!u32::from(mask)))
}

/// Invert a contiguous wildcard mask to subnet mask.
pub fn wildcard_to_subnet(wildcard: Ipv4Addr) -> Option<Ipv4Addr> {
    if !is_contiguous_wildcard(wildcard) {
        return None;
    }
    Some(Ipv4Addr::from(!u32::from(wildcard)))
}

/// Determine standard classful prefix length for an IPv4 address.
pub fn classful_prefix(ip: Ipv4Addr) -> u8 {
    match ip.octets()[0] {
        0..=127 => 8,
        128..=191 => 16,
        192..=223 => 24,
        _ => 24,
    }
}

/// Check if a value is a sentinel, null, or empty string.
pub fn is_sentinel(s: &str) -> bool {
    let lower = s.trim().to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "" | "--"
            | "none"
            | "null"
            | "nil"
            | "unknown"
            | "unset"
            | "n/a"
            | "<not set>"
            | "not set"
            | "-1"
            | "65535"
            | "4294967295"
            | "18446744073709551615"
    ) || s.contains("${")
}

// ---------------------------------------------------------------------------
// Static Route Parsing & Interpretation
// ---------------------------------------------------------------------------

/// Parsed IPv4 static route specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedStaticRoute {
    /// Raw unparsed string.
    pub raw: String,
    /// Normalized destination network.
    pub destination: NormalizedNetwork,
    /// Next-hop IPv4 address, if known.
    pub next_hop: Option<Ipv4Addr>,
    /// Exit interface name, if known.
    pub exit_interface: Option<String>,
    /// Administrative distance (1..=255), if specified.
    pub distance: Option<u8>,
    /// True if default route (`0.0.0.0/0`).
    pub is_default: bool,
    /// True if all parameters required to generate a valid Cisco command are present.
    pub is_complete: bool,
    /// Missing required parameters.
    pub missing_fields: Vec<String>,
}

impl ParsedStaticRoute {
    /// Parse an IPv4 static route string in any Packet Tracer or Cisco format:
    /// - Hyphenated: `172.16.128.0-17-172.18.192.1-0-1`
    /// - Hyphenated exit iface: `0.0.0.0-0-Serial0/1/0-0-1`
    /// - Short hyphenated: `100.0.0.0-8`
    /// - CLI format: `192.168.1.0 255.255.255.0 10.0.0.1`
    /// - CLI with distance: `192.168.1.0 255.255.255.0 10.0.0.1 5`
    /// - Default route: `0.0.0.0 0.0.0.0 Serial0/1/0`
    pub fn parse(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() || is_sentinel(trimmed) {
            return Err("Empty or sentinel static route specification".into());
        }

        // Check for hyphenated format with host-bit inconsistency
        let hyphen_parts: Vec<&str> = trimmed.split('-').collect();
        if hyphen_parts.len() >= 2 {
            if let (Ok(net_addr), Ok(prefix)) = (
                hyphen_parts[0].trim().parse::<Ipv4Addr>(),
                hyphen_parts[1].trim().parse::<u8>(),
            ) {
                if prefix <= 32 {
                    if let Some(mask) = prefix_to_subnet(prefix) {
                        if u32::from(net_addr) & !u32::from(mask) != 0 {
                            return Err(format!(
                                "Inconsistent destination address {net_addr} and prefix /{prefix} (host bits are set)"
                            ));
                        }
                    }
                }
            }
        }

        // Check for space-separated CLI format with host-bit inconsistency
        let tokens: Vec<&str> = trimmed
            .strip_prefix("ip route ")
            .unwrap_or(trimmed)
            .split_whitespace()
            .collect();
        if tokens.len() >= 3 {
            if let (Ok(net_addr), Ok(mask)) = (
                tokens[0].parse::<Ipv4Addr>(),
                tokens[1].parse::<Ipv4Addr>(),
            ) {
                if is_contiguous_mask(mask) && u32::from(net_addr) & !u32::from(mask) != 0 {
                    return Err(format!(
                        "Inconsistent destination address {net_addr} and subnet mask {mask} (host bits are set)"
                    ));
                }
            }
        }

        // Try Packet Tracer hyphenated format:
        // E.g. "172.16.128.0-17-172.18.192.1-0-1" or "0.0.0.0-0-Serial0/1/0-0-1" or "100.0.0.0-8"
        if let Some(route) = Self::parse_hyphenated(trimmed) {
            return Ok(route);
        }

        // Try space-separated CLI format:
        if let Some(route) = Self::parse_space_separated(trimmed) {
            return Ok(route);
        }

        // Fallback: try parsing input as a network only (e.g. "100.0.0.0/8" or "100.0.0.0-8")
        if hyphen_parts.len() <= 2 && tokens.len() <= 2 {
            if let Some(net) = NormalizedNetwork::parse(trimmed) {
                let is_default = net.network == Ipv4Addr::new(0, 0, 0, 0) && net.prefix == 0;
                return Ok(Self {
                    raw: trimmed.to_string(),
                    destination: net,
                    next_hop: None,
                    exit_interface: None,
                    distance: None,
                    is_default,
                    is_complete: false,
                    missing_fields: vec!["next-hop or exit-interface".into()],
                });
            }
        }

        Err(format!("Unrecognized static route format: '{trimmed}'"))
    }

    fn parse_hyphenated(input: &str) -> Option<Self> {
        // Hyphenated format: <net>-<prefix>[-<target>[-<metric>[-<distance>]]]
        let parts: Vec<&str> = input.split('-').collect();
        if parts.len() < 2 {
            return None;
        }

        let net_addr: Ipv4Addr = parts[0].trim().parse().ok()?;
        let prefix: u8 = parts[1].trim().parse().ok()?;
        if prefix > 32 {
            return None;
        }

        let destination =
            NormalizedNetwork::from_ip_and_prefix(net_addr, prefix, format!("{net_addr}/{prefix}"))?;
        if net_addr != destination.network {
            // Destination address has non-zero host bits for prefix length
            return None;
        }
        let is_default = net_addr == Ipv4Addr::new(0, 0, 0, 0) && prefix == 0;

        let mut next_hop = None;
        let mut exit_interface = None;
        let mut distance = None;
        let mut missing_fields = Vec::new();

        if parts.len() >= 3 {
            let target = parts[2].trim();
            if let Ok(ip) = target.parse::<Ipv4Addr>() {
                next_hop = Some(ip);
            } else if is_valid_interface_name(target) {
                exit_interface = Some(target.to_string());
            }
        }

        if parts.len() >= 5 {
            if let Ok(dist) = parts[4].trim().parse::<u8>() {
                if (1..=255).contains(&dist) {
                    distance = Some(dist);
                }
            }
        }

        let is_complete = next_hop.is_some() || exit_interface.is_some();
        if !is_complete {
            missing_fields.push("next-hop or exit-interface".into());
        }

        Some(Self {
            raw: input.to_string(),
            destination,
            next_hop,
            exit_interface,
            distance,
            is_default,
            is_complete,
            missing_fields,
        })
    }

    fn parse_space_separated(input: &str) -> Option<Self> {
        let tokens: Vec<&str> = input
            .strip_prefix("ip route ")
            .unwrap_or(input)
            .split_whitespace()
            .collect();

        if tokens.len() < 3 {
            return None;
        }

        let net_addr: Ipv4Addr = tokens[0].parse().ok()?;
        let mask: Ipv4Addr = tokens[1].parse().ok()?;
        let prefix = subnet_to_prefix(mask)?;
        let destination =
            NormalizedNetwork::from_ip_and_prefix(net_addr, prefix, format!("{net_addr} {mask}"))?;
        if net_addr != destination.network {
            // Destination address has non-zero host bits for subnet mask
            return None;
        }
        let is_default = net_addr == Ipv4Addr::new(0, 0, 0, 0) && prefix == 0;

        let mut next_hop = None;
        let mut exit_interface = None;
        let mut distance = None;

        let target = tokens[2];
        if let Ok(ip) = target.parse::<Ipv4Addr>() {
            next_hop = Some(ip);
        } else if is_valid_interface_name(target) {
            exit_interface = Some(target.to_string());
        } else {
            return None;
        }

        if tokens.len() >= 4 {
            if let Ok(dist) = tokens[3].parse::<u8>() {
                if (1..=255).contains(&dist) {
                    distance = Some(dist);
                }
            }
        }

        Some(Self {
            raw: input.to_string(),
            destination,
            next_hop,
            exit_interface,
            distance,
            is_default,
            is_complete: true,
            missing_fields: Vec::new(),
        })
    }

    /// Generate the Cisco IOS command if the static route is complete.
    /// Format: `ip route <network> <subnet-mask> <next-hop-or-interface> [distance]`
    /// Distance is omitted if default (1).
    pub fn to_cisco_command(&self) -> Option<String> {
        if !self.is_complete {
            return None;
        }
        let target = if let Some(ip) = self.next_hop {
            ip.to_string()
        } else if let Some(ref iface) = self.exit_interface {
            iface.clone()
        } else {
            return None;
        };

        let mut cmd = format!(
            "ip route {} {} {}",
            self.destination.network, self.destination.subnet_mask, target
        );

        if let Some(dist) = self.distance {
            if dist > 1 {
                cmd.push_str(&format!(" {dist}"));
            }
        }

        Some(cmd)
    }
}

/// Parsed IPv6 static route specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedIpv6StaticRoute {
    pub raw: String,
    pub destination: Ipv6Addr,
    pub prefix: u8,
    pub next_hop: Option<Ipv6Addr>,
    pub exit_interface: Option<String>,
    pub distance: Option<u8>,
    pub is_default: bool,
    pub is_complete: bool,
}

impl ParsedIpv6StaticRoute {
    /// Parse an IPv6 static route (e.g. `2001:DB8:ACAD:3::-64-2001:DB8:ACAD:2::2-0-1`
    /// or `::-0-Serial0/1/1-0-1`).
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() || is_sentinel(trimmed) {
            return None;
        }

        let parts: Vec<&str> = trimmed.split('-').collect();
        if parts.len() < 2 {
            return None;
        }

        let dest: Ipv6Addr = parts[0].trim().parse().ok()?;
        let prefix: u8 = parts[1].trim().parse().ok()?;
        if prefix > 128 {
            return None;
        }

        let is_default = dest == Ipv6Addr::UNSPECIFIED && prefix == 0;
        let mut next_hop = None;
        let mut exit_interface = None;
        let mut distance = None;

        if parts.len() >= 3 {
            let target = parts[2].trim();
            if let Ok(ip) = target.parse::<Ipv6Addr>() {
                next_hop = Some(ip);
            } else if is_valid_interface_name(target) {
                exit_interface = Some(target.to_string());
            }
        }

        if parts.len() >= 5 {
            if let Ok(dist) = parts[4].trim().parse::<u8>() {
                if (1..=255).contains(&dist) {
                    distance = Some(dist);
                }
            }
        }

        let is_complete = next_hop.is_some() || exit_interface.is_some();

        Some(Self {
            raw: trimmed.to_string(),
            destination: dest,
            prefix,
            next_hop,
            exit_interface,
            distance,
            is_default,
            is_complete,
        })
    }

    /// Generate Cisco IOS `ipv6 route <prefix> <next-hop|iface> [distance]`.
    pub fn to_cisco_command(&self) -> Option<String> {
        if !self.is_complete {
            return None;
        }
        let target = if let Some(ip) = self.next_hop {
            ip.to_string()
        } else if let Some(ref iface) = self.exit_interface {
            iface.clone()
        } else {
            return None;
        };

        let mut cmd = format!("ipv6 route {}/{} {}", self.destination, self.prefix, target);
        if let Some(dist) = self.distance {
            if dist > 1 {
                cmd.push_str(&format!(" {dist}"));
            }
        }
        Some(cmd)
    }
}

// ---------------------------------------------------------------------------
// OSPF Network Parsing & Interpretation
// ---------------------------------------------------------------------------

/// Parsed OSPF network statement specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedOspfNetwork {
    /// Raw unparsed input string.
    pub raw: String,
    /// OSPF Process ID (1..=65535), if known.
    pub process_id: Option<u32>,
    /// Network address.
    pub network: Ipv4Addr,
    /// Contiguous or explicit wildcard mask.
    pub wildcard_mask: Ipv4Addr,
    /// OSPF Area ID (e.g. "0" or "0.0.0.0"), if specified.
    pub area: Option<String>,
    /// Whether the wildcard mask was derived from a CIDR prefix length.
    pub derived_from_prefix: bool,
    /// True if all fields needed to generate `network <net> <wildcard> area <area>` are present.
    pub is_complete: bool,
    /// List of missing required fields.
    pub missing_fields: Vec<String>,
}

impl ParsedOspfNetwork {
    /// Parse an OSPF network statement from various encodings:
    /// - `network 10.0.0.0 0.0.0.3 area 0`
    /// - `10.0.0.0 0.0.0.3 0`
    /// - `1 10.0.0.0 0.0.0.3 0` (process 1)
    /// - `100.0.0.0-8` (normalized with wildcard 0.255.255.255, missing area/process)
    /// - `192.168.1.0/24` (normalized with wildcard 0.0.0.255, missing area/process)
    pub fn parse(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() || is_sentinel(trimmed) {
            return Err("Empty or sentinel OSPF specification".into());
        }

        // Case 1: CLI format "network <net> <wildcard> area <area>"
        if let Some(cmd) = trimmed.strip_prefix("network ") {
            let tokens: Vec<&str> = cmd.split_whitespace().collect();
            if tokens.len() == 4 && tokens[2] == "area" {
                let net: Ipv4Addr = tokens[0].parse().map_err(|e| format!("{e}"))?;
                let wildcard: Ipv4Addr = tokens[1].parse().map_err(|e| format!("{e}"))?;
                let area = tokens[3].to_string();
                if is_sentinel(&area) || area == "4294967295" {
                    return Err("Sentinel or invalid area in OSPF specification".into());
                }
                return Ok(Self {
                    raw: trimmed.to_string(),
                    process_id: None,
                    network: net,
                    wildcard_mask: wildcard,
                    area: Some(area),
                    derived_from_prefix: false,
                    is_complete: false,
                    missing_fields: vec!["OSPF process ID".into()],
                });
            }
        }

        // Case 2: Assessment format "<net> <wildcard> <area>" e.g. "10.0.0.0 0.0.0.3 0"
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.len() == 3 {
            if let (Ok(net), Ok(wildcard)) = (tokens[0].parse::<Ipv4Addr>(), tokens[1].parse::<Ipv4Addr>()) {
                let area = tokens[2].to_string();
                if is_sentinel(&area) || area == "4294967295" {
                    return Err("Sentinel or invalid area in OSPF specification".into());
                }
                return Ok(Self {
                    raw: trimmed.to_string(),
                    process_id: None,
                    network: net,
                    wildcard_mask: wildcard,
                    area: Some(area),
                    derived_from_prefix: false,
                    is_complete: false,
                    missing_fields: vec!["OSPF process ID".into()],
                });
            }
        }

        // Case 3: Assessment with process ID "<process> <net> <wildcard> <area>"
        if tokens.len() == 4 {
            if let (Ok(pid), Ok(net), Ok(wildcard)) = (
                tokens[0].parse::<u32>(),
                tokens[1].parse::<Ipv4Addr>(),
                tokens[2].parse::<Ipv4Addr>(),
            ) {
                let area = tokens[3].to_string();
                if pid == 0 || pid >= 65535 || is_sentinel(&area) || area == "4294967295" {
                    return Err("Sentinel or invalid process/area in OSPF specification".into());
                }
                return Ok(Self {
                    raw: trimmed.to_string(),
                    process_id: Some(pid),
                    network: net,
                    wildcard_mask: wildcard,
                    area: Some(area),
                    derived_from_prefix: false,
                    is_complete: true,
                    missing_fields: Vec::new(),
                });
            }
        }

        // Case 4: CIDR or hyphenated network encoding (e.g. "100.0.0.0-8" or "100.0.0.0/8")
        if let Some(norm) = NormalizedNetwork::parse(trimmed) {
            return Ok(Self {
                raw: trimmed.to_string(),
                process_id: None,
                network: norm.network,
                wildcard_mask: norm.wildcard_mask,
                area: None,
                derived_from_prefix: true,
                is_complete: false,
                missing_fields: vec!["OSPF process ID".into(), "OSPF area ID".into()],
            });
        }

        Err(format!("Unrecognized OSPF network statement: '{trimmed}'"))
    }

    /// Set or supply the OSPF process ID.
    pub fn with_process_id(mut self, pid: u32) -> Self {
        self.process_id = Some(pid);
        self.missing_fields.retain(|f| f != "OSPF process ID");
        self.is_complete = self.area.is_some();
        self
    }

    /// Set or supply the OSPF area ID.
    pub fn with_area(mut self, area: impl Into<String>) -> Self {
        self.area = Some(area.into());
        self.missing_fields.retain(|f| f != "OSPF area ID");
        self.is_complete = true;
        self
    }

    /// Generate Cisco IOS OSPF statement: `network <address> <wildcard-mask> area <area-id>`
    pub fn to_cisco_command(&self) -> Option<String> {
        let area = self.area.as_ref()?;
        Some(format!(
            "network {} {} area {}",
            self.network, self.wildcard_mask, area
        ))
    }
}

/// Validate if an interface name matches Cisco IOS naming conventions.
pub fn is_valid_interface_name(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    let prefixes = [
        "gigabitethernet",
        "fastethernet",
        "tengigabitethernet",
        "ethernet",
        "serial",
        "loopback",
        "vlan",
        "port-channel",
        "gi",
        "fa",
        "eth",
        "se",
        "lo",
        "te",
        "po",
    ];

    for p in prefixes {
        if let Some(tail) = lower.strip_prefix(p) {
            if tail.starts_with(|c: char| c.is_ascii_digit())
                && tail
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '/' || c == '.')
            {
                return true;
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_hyphenated_prefix() {
        let n = NormalizedNetwork::parse("100.0.0.0-8").expect("should parse 100.0.0.0-8");
        assert_eq!(n.network, Ipv4Addr::new(100, 0, 0, 0));
        assert_eq!(n.prefix, 8);
        assert_eq!(n.subnet_mask, Ipv4Addr::new(255, 0, 0, 0));
        assert_eq!(n.wildcard_mask, Ipv4Addr::new(0, 255, 255, 255));
        assert_eq!(n.cidr(), "100.0.0.0/8");
        assert_eq!(n.confidence, Confidence::Medium);
    }

    #[test]
    fn test_normalize_standard_cidr() {
        let n = NormalizedNetwork::parse("192.168.1.0/24").expect("192.168.1.0/24");
        assert_eq!(n.network, Ipv4Addr::new(192, 168, 1, 0));
        assert_eq!(n.prefix, 24);
        assert_eq!(n.subnet_mask, Ipv4Addr::new(255, 255, 255, 0));
        assert_eq!(n.wildcard_mask, Ipv4Addr::new(0, 0, 0, 255));
        assert_eq!(n.confidence, Confidence::High);

        let n16 = NormalizedNetwork::parse("172.16.0.0/16").unwrap();
        assert_eq!(n16.subnet_mask, Ipv4Addr::new(255, 255, 0, 0));
        assert_eq!(n16.wildcard_mask, Ipv4Addr::new(0, 0, 255, 255));

        let n30 = NormalizedNetwork::parse("10.0.0.0/30").unwrap();
        assert_eq!(n30.subnet_mask, Ipv4Addr::new(255, 255, 255, 252));
        assert_eq!(n30.wildcard_mask, Ipv4Addr::new(0, 0, 0, 3));

        let n0 = NormalizedNetwork::parse("0.0.0.0/0").unwrap();
        assert_eq!(n0.subnet_mask, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(n0.wildcard_mask, Ipv4Addr::new(255, 255, 255, 255));

        let n32 = NormalizedNetwork::parse("192.168.1.1/32").unwrap();
        assert_eq!(n32.subnet_mask, Ipv4Addr::new(255, 255, 255, 255));
        assert_eq!(n32.wildcard_mask, Ipv4Addr::new(0, 0, 0, 0));
    }

    #[test]
    fn test_reject_invalid_prefixes() {
        assert!(NormalizedNetwork::parse("100.0.0.0-33").is_none());
        assert!(NormalizedNetwork::parse("100.0.0.0/35").is_none());
        assert!(NormalizedNetwork::parse("100.0.0.0--1").is_none());
        assert!(NormalizedNetwork::parse("invalid-8").is_none());
        assert!(NormalizedNetwork::parse("").is_none());
        assert!(NormalizedNetwork::parse("none").is_none());
    }

    #[test]
    fn test_static_route_parsing_complete() {
        let r = ParsedStaticRoute::parse("172.16.128.0-17-172.18.192.1-0-1").unwrap();
        assert_eq!(r.destination.network, Ipv4Addr::new(172, 16, 128, 0));
        assert_eq!(r.destination.prefix, 17);
        assert_eq!(r.destination.subnet_mask, Ipv4Addr::new(255, 255, 128, 0));
        assert_eq!(r.next_hop, Some(Ipv4Addr::new(172, 18, 192, 1)));
        assert_eq!(r.exit_interface, None);
        assert_eq!(r.distance, Some(1));
        assert!(r.is_complete);
        assert_eq!(
            r.to_cisco_command().unwrap(),
            "ip route 172.16.128.0 255.255.128.0 172.18.192.1"
        );
    }

    #[test]
    fn test_static_route_exit_interface_and_default() {
        let r = ParsedStaticRoute::parse("0.0.0.0-0-Serial0/1/0-0-1").unwrap();
        assert!(r.is_default);
        assert_eq!(r.destination.network, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(r.destination.subnet_mask, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(r.exit_interface, Some("Serial0/1/0".into()));
        assert_eq!(r.next_hop, None);
        assert!(r.is_complete);
        assert_eq!(
            r.to_cisco_command().unwrap(),
            "ip route 0.0.0.0 0.0.0.0 Serial0/1/0"
        );
    }

    #[test]
    fn test_static_route_floating_distance() {
        let r = ParsedStaticRoute::parse("10.0.0.0 255.0.0.0 192.168.1.1 5").unwrap();
        assert_eq!(r.distance, Some(5));
        assert_eq!(
            r.to_cisco_command().unwrap(),
            "ip route 10.0.0.0 255.0.0.0 192.168.1.1 5"
        );
    }

    #[test]
    fn test_static_route_missing_next_hop() {
        let r = ParsedStaticRoute::parse("100.0.0.0-8").unwrap();
        assert!(!r.is_complete);
        assert!(r.to_cisco_command().is_none());
        assert_eq!(r.missing_fields, vec!["next-hop or exit-interface"]);
        assert_eq!(r.destination.cidr(), "100.0.0.0/8");
    }

    #[test]
    fn test_ipv6_static_route() {
        let r =
            ParsedIpv6StaticRoute::parse("2001:DB8:ACAD:3::-64-2001:DB8:ACAD:2::2-0-1").unwrap();
        assert_eq!(r.prefix, 64);
        assert_eq!(
            r.next_hop,
            Some("2001:db8:acad:2::2".parse::<Ipv6Addr>().unwrap())
        );
        assert_eq!(
            r.to_cisco_command().unwrap(),
            "ipv6 route 2001:db8:acad:3::/64 2001:db8:acad:2::2"
        );

        let def = ParsedIpv6StaticRoute::parse("::-0-Serial0/1/1-0-1").unwrap();
        assert!(def.is_default);
        assert_eq!(
            def.to_cisco_command().unwrap(),
            "ipv6 route ::/0 Serial0/1/1"
        );
    }

    #[test]
    fn test_ospf_network_parsing() {
        // Complete 4-token statement with process ID
        let o_with_pid = ParsedOspfNetwork::parse("1 10.0.0.0 0.0.0.3 0").unwrap();
        assert_eq!(o_with_pid.process_id, Some(1));
        assert_eq!(o_with_pid.network, Ipv4Addr::new(10, 0, 0, 0));
        assert_eq!(o_with_pid.wildcard_mask, Ipv4Addr::new(0, 0, 0, 3));
        assert_eq!(o_with_pid.area, Some("0".into()));
        assert!(o_with_pid.is_complete);
        assert_eq!(
            o_with_pid.to_cisco_command().unwrap(),
            "network 10.0.0.0 0.0.0.3 area 0"
        );

        // 3-token statement without process ID: needs process ID before execution
        let o = ParsedOspfNetwork::parse("10.0.0.0 0.0.0.3 0").unwrap();
        assert_eq!(o.network, Ipv4Addr::new(10, 0, 0, 0));
        assert_eq!(o.wildcard_mask, Ipv4Addr::new(0, 0, 0, 3));
        assert_eq!(o.area, Some("0".into()));
        assert!(!o.is_complete);
        assert_eq!(o.missing_fields, vec!["OSPF process ID"]);

        let o_completed = o.with_process_id(1);
        assert!(o_completed.is_complete);
        assert_eq!(
            o_completed.to_cisco_command().unwrap(),
            "network 10.0.0.0 0.0.0.3 area 0"
        );

        // From hyphenated CIDR encoding: "100.0.0.0-8"
        let o2 = ParsedOspfNetwork::parse("100.0.0.0-8").unwrap();
        assert_eq!(o2.wildcard_mask, Ipv4Addr::new(0, 255, 255, 255));
        assert!(!o2.is_complete);
        assert!(o2.to_cisco_command().is_none());

        // Completing with area
        let o2_complete = o2.with_area("0").with_process_id(1);
        assert!(o2_complete.is_complete);
        assert_eq!(
            o2_complete.to_cisco_command().unwrap(),
            "network 100.0.0.0 0.255.255.255 area 0"
        );
    }
}
