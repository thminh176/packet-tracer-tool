#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeviceProfile {
    #[default]
    Unknown,
    GenericRouter,
    GenericSwitch,
    Catalyst2960,
    Catalyst3560,
    Router1941,
    Router2911,
    Isr4321,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportState {
    VerifiedPacketTracer,
    DocumentedNotTested,
    Unsupported,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Basic,
    Switching,
    Routing,
    Multilayer,
    Serial,
    DynamicRouting,
    Ipv6Routing,
    Security,
    Unknown,
}
impl DeviceProfile {
    pub const ALL: [Self; 8] = [
        Self::Unknown,
        Self::GenericRouter,
        Self::GenericSwitch,
        Self::Catalyst2960,
        Self::Catalyst3560,
        Self::Router1941,
        Self::Router2911,
        Self::Isr4321,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown IOS",
            Self::GenericRouter => "Generic router",
            Self::GenericSwitch => "Generic switch",
            Self::Catalyst2960 => "Catalyst 2960",
            Self::Catalyst3560 => "Catalyst 3560",
            Self::Router1941 => "Cisco 1941",
            Self::Router2911 => "Cisco 2911",
            Self::Isr4321 => "ISR 4321 (IOS XE)",
        }
    }
    pub fn from_evidence(model: &str, kind: &str) -> Self {
        let m = model.trim().to_ascii_lowercase();
        match m.as_str() {
            "2960" | "2960-24tt" | "catalyst2960" => Self::Catalyst2960,
            "3560" | "3560-24ps" | "catalyst3560" => Self::Catalyst3560,
            "1941" | "cisco1941" => Self::Router1941,
            "2911" | "cisco2911" => Self::Router2911,
            "4321" | "isr4321" => Self::Isr4321,
            _ => match kind.to_ascii_lowercase().as_str() {
                "router" => Self::GenericRouter,
                "switch" => Self::GenericSwitch,
                _ => Self::Unknown,
            },
        }
    }
    pub fn support(self, feature: Capability) -> SupportState {
        use Capability::*;
        use DeviceProfile::*;
        match (self, feature) {
            (Catalyst2960, Routing | Multilayer | DynamicRouting | Ipv6Routing | Serial) => {
                SupportState::Unsupported
            }
            (Router1941 | Router2911 | Isr4321 | GenericRouter, Switching) => {
                SupportState::Unsupported
            }
            (Router1941 | Router2911 | Isr4321 | GenericRouter, Multilayer) => {
                SupportState::Unsupported
            }
            (Catalyst3560, Serial) => SupportState::Unsupported,
            (Catalyst3560, Multilayer) => SupportState::DocumentedNotTested,
            (Catalyst2960 | Catalyst3560, Basic | Switching | Security) => {
                SupportState::DocumentedNotTested
            }
            _ => SupportState::Unknown,
        }
    }
}
