//! Capability catalog shared by CLI/MCP.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilitySurface {
    Library,
    Cli,
    Mcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityKind {
    Io,
    Transform,
    Analytics,
    Advanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capability {
    pub name: &'static str,
    pub kind: CapabilityKind,
}

pub const CAPABILITIES: &[Capability] = &[
    // I/O
    Capability {
        name: "read",
        kind: CapabilityKind::Io,
    },
    Capability {
        name: "write",
        kind: CapabilityKind::Io,
    },
    Capability {
        name: "convert",
        kind: CapabilityKind::Io,
    },
    Capability {
        name: "sheets",
        kind: CapabilityKind::Io,
    },
    Capability {
        name: "read_all",
        kind: CapabilityKind::Io,
    },
    // Transforms
    Capability {
        name: "sort",
        kind: CapabilityKind::Transform,
    },
    Capability {
        name: "filter",
        kind: CapabilityKind::Transform,
    },
    Capability {
        name: "replace",
        kind: CapabilityKind::Transform,
    },
    Capability {
        name: "dedupe",
        kind: CapabilityKind::Transform,
    },
    Capability {
        name: "transpose",
        kind: CapabilityKind::Transform,
    },
    Capability {
        name: "select",
        kind: CapabilityKind::Transform,
    },
    // Analytics-ish
    Capability {
        name: "head",
        kind: CapabilityKind::Analytics,
    },
    Capability {
        name: "tail",
        kind: CapabilityKind::Analytics,
    },
    Capability {
        name: "describe",
        kind: CapabilityKind::Analytics,
    },
    // Advanced
    Capability {
        name: "validate",
        kind: CapabilityKind::Advanced,
    },
    Capability {
        name: "profile",
        kind: CapabilityKind::Advanced,
    },
    Capability {
        name: "schema",
        kind: CapabilityKind::Advanced,
    },
];

pub const FORMATS: &[&str] = &["csv", "xlsx", "xls", "ods", "parquet", "avro", "json"];

