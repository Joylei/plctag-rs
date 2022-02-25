// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

//! builders for tag path and tag

pub use crate::debug::DebugLevel;
use core::fmt;
use core::fmt::Write;

type Result<T> = std::result::Result<T, Error>;

/// tag builder error
#[derive(Debug)]
pub struct Error(&'static str);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

impl From<fmt::Error> for Error {
    fn from(_e: fmt::Error) -> Self {
        Error("fmt error")
    }
}

/// builder to build tag full path
///
/// # Examples
/// ```rust,no_run
/// use plctag_core::builder::*;
/// use plctag_core::RawTag;
///
///     let timeout = 100;
///     let path = PathBuilder::default()
///         .protocol(Protocol::EIP)
///         .gateway("192.168.1.120")
///         .plc(PlcKind::ControlLogix)
///         .name("MyTag1")
///         .element_size(16)
///         .element_count(1)
///         .path("1,0")
///         .read_cache_ms(0)
///         .build()
///         .unwrap();
///     let tag = RawTag::new(path, timeout).unwrap();
///     let status = tag.status();
///     assert!(status.is_ok());
///
/// ```
///
/// # Tag String Attributes
/// See https://github.com/libplctag/libplctag/wiki/Tag-String-Attributes for more information.
///
#[derive(Default, Debug)]
pub struct PathBuilder {
    protocol: Option<Protocol>,
    debug: Option<DebugLevel>,
    elem_count: Option<usize>,
    elem_size: Option<usize>,
    read_cache_ms: Option<usize>,
    auto_sync_read_ms: Option<usize>,
    auto_sync_write_ms: Option<usize>,
    plc: Option<PlcKind>,
    name: Option<String>,
    path: Option<String>,
    gateway: Option<String>,
    use_connected_msg: Option<bool>,
    allow_packing: Option<bool>,
    connection_group_id: Option<u16>,
}

impl PathBuilder {
    /// generic attribute.
    /// defining the current debugging level.
    /// please use [`plc::set_debug_level`](../plc/fn.set_debug_level.html) instead.
    #[deprecated]
    pub fn debug(&mut self, level: DebugLevel) -> &mut Self {
        self.debug = Some(level);
        self
    }

    /// generic attribute.
    /// Required. Determines the type of the PLC protocol.
    pub fn protocol(&mut self, protocol: Protocol) -> &mut Self {
        self.protocol = Some(protocol);
        self
    }

    /// generic attribute.
    ///  Optional. All tags are treated as arrays. Tags that are not arrays are considered to have a length of one element. This attribute determines how many elements are in the tag. Defaults to one (1)
    pub fn element_count(&mut self, count: usize) -> &mut Self {
        self.elem_count = Some(count);
        self
    }

    /// generic attribute
    /// Required for some protocols or PLC types. This attribute determines the size of a single element of the tag. All tags are considered to be arrays, even those with only one entry. Ignored for Modbus and for ControlLogix-class Allen-Bradley PLCs. This parameter will become optional for as many PLC types as possible
    pub fn element_size(&mut self, size: usize) -> &mut Self {
        self.elem_size = Some(size);
        self
    }

    /// generic attribute:
    /// Optional. An integer number of milliseconds to cache read data.
    /// Use this attribute to cause the tag read operations to cache data the requested number of milliseconds. This can be used to lower the actual number of requests against the PLC. Example read_cache_ms=100 will result in read operations no more often than once every 100 milliseconds.
    pub fn read_cache_ms(&mut self, millis: usize) -> &mut Self {
        self.read_cache_ms = Some(millis);
        self
    }

    /// Optional An integer number of milliseconds to periodically read data from the PLC
    pub fn auto_sync_read_ms(&mut self, millis: usize) -> &mut Self {
        self.auto_sync_read_ms = Some(millis);
        self
    }

    /// Optional An integer number of milliseconds to buffer tag data changes before writing to the PLC
    pub fn auto_sync_write_ms(&mut self, millis: usize) -> &mut Self {
        self.auto_sync_write_ms = Some(millis);
        self
    }

    /// Required for EIP. Determines the type of the PLC
    pub fn plc(&mut self, plc: PlcKind) -> &mut Self {
        self.plc = Some(plc);
        self
    }

    /// - EIP
    /// IP address or host name.
    /// This tells the library what host name or IP address to use for the PLC or the gateway to the PLC (in the case that the PLC is remote).
    /// - ModBus
    /// Required IP address or host name and optional port
    /// This tells the library what host name or IP address to use for the PLC. Can have an optional port at the end, e.g. gateway=10.1.2.3:502 where the :502 part specifies the port.
    pub fn gateway(&mut self, gateway: impl AsRef<str>) -> &mut Self {
        self.gateway = Some(gateway.as_ref().to_owned());
        self
    }

    /// - EIP
    /// This is the full name of the tag. For program tags, prepend Program:<program name>. where <program name> is the name of the program in which the tag is created
    /// - ModBus
    /// Required the type and first register number of a tag, e.g. co42 for coil 42 (counts from zero).
    /// The supported register type prefixes are co for coil, di for discrete inputs, hr for holding registers and ir for input registers. The type prefix must be present and the register number must be greater than or equal to zero and less than or equal to 65535. Modbus examples: co21 - coil 21, di22 - discrete input 22, hr66 - holding register 66, ir64000 - input register 64000.
    ///
    /// you might want to use `register()` instead of `name()` for Modbus
    pub fn name(&mut self, name: impl AsRef<str>) -> &mut Self {
        self.name = Some(name.as_ref().to_owned());
        self
    }

    /// set register for Modbus
    pub fn register(&mut self, reg: Register) -> &mut Self {
        self.name = Some(format!("{}", reg));
        self
    }

    /// - EIP
    /// AB: CIP path to PLC CPU. I.e. 1,0.
    /// This attribute is required for CompactLogix/ControlLogix tags and for tags using a DH+ protocol bridge (i.e. a DHRIO module) to get to a PLC/5, SLC 500, or MicroLogix PLC on a remote DH+ link. The attribute is ignored if it is not a DH+ bridge route, but will generate a warning if debugging is active. Note that Micro800 connections must not have a path attribute.
    /// - ModBus
    /// Required The server/unit ID. Must be an integer value between 0 and 255.
    /// Servers may support more than one unit or may bridge to other units.
    pub fn path(&mut self, path: impl AsRef<str>) -> &mut Self {
        self.path = Some(path.as_ref().to_owned());
        self
    }

    /// EIP only
    /// Optional 1 = use CIP connection, 0 = use UCMM.
    /// Control whether to use connected or unconnected messaging. Only valid on Logix-class PLCs. Connected messaging is required on Micro800 and DH+ bridged links. Default is PLC-specific and link-type specific. Generally you do not need to set this.
    pub fn use_connected_msg(&mut self, yes: bool) -> &mut Self {
        self.use_connected_msg = Some(yes);
        self
    }

    /// EIP only
    /// Optional 1 = (default) allow use of multi-request CIP command, 0 = use only one CIP request per packet
    pub fn allow_packing(&mut self, allow: bool) -> &mut Self {
        self.allow_packing = Some(allow);
        self
    }

    /// connection group
    pub fn connection_group(&mut self, group_id: u16) -> &mut Self {
        self.connection_group_id = Some(group_id);
        self
    }

    /// check required attributes or conflict attributes
    fn check(&self) -> Result<()> {
        //check protocol, required
        let protocol = match self.protocol {
            None => return Err(Error("protocol required")),
            Some(v) => v,
        };
        // check required attributes
        match protocol {
            Protocol::EIP => {
                //TODO: check gateway, either ip or host name
                //check plc, required
                if self.plc.is_none() {
                    return Err(Error("plc kind required"));
                }
                let plc = self.plc.unwrap();
                if plc == PlcKind::ControlLogix {
                    if self.path.is_none() {
                        return Err(Error("path required for controllogix"));
                    }
                    return Ok(()); //skip check for elem_size
                } else if plc == PlcKind::Micro800 && self.path.is_some() {
                    return Err(Error("path must not provided for micro800"));
                }
                if self.elem_size.is_none() {
                    return Err(Error("element size required"));
                }
            }
            Protocol::ModBus => {
                //TODO: check gateway, host with port
                if self.gateway.is_none() {
                    return Err(Error("gateway required"));
                }
                if self.name.is_none() {
                    return Err(Error("name required"));
                }
                //path is number [0-255]
                match self.path {
                    Some(ref path) => {
                        let _: u8 = path
                            .parse()
                            .or(Err(Error("path is a number in range [0-255]")))?;
                    }
                    None => return Err(Error("path required")),
                }
                if self.elem_size.is_none() {
                    return Err(Error("element size required"));
                }
            }
        }
        Ok(())
    }

    /// build full tag path
    pub fn build(&self) -> Result<String> {
        self.check()?;
        let mut path_buf = String::new();
        let protocol = self.protocol.unwrap();
        write!(path_buf, "protocol={}", protocol)?;

        match protocol {
            Protocol::EIP => {
                if let Some(plc) = self.plc {
                    write!(path_buf, "&plc={}", plc)?;
                }

                if let Some(yes) = self.use_connected_msg {
                    write!(path_buf, "&use_connected_msg={}", yes as u8)?;
                }

                if let Some(v) = self.allow_packing {
                    write!(path_buf, "&allow_packing={}", v as u8)?;
                }
            }
            Protocol::ModBus => {}
        }

        if let Some(ref gateway) = self.gateway {
            write!(path_buf, "&gateway={}", gateway)?;
        }
        if let Some(ref path) = self.path {
            write!(path_buf, "&path={}", path)?;
        }
        if let Some(ref name) = self.name {
            write!(path_buf, "&name={}", name)?;
        }
        if let Some(elem_count) = self.elem_count {
            write!(path_buf, "&elem_count={}", elem_count)?
        }

        if let Some(elem_size) = self.elem_size {
            write!(path_buf, "&elem_size={}", elem_size)?;
        }

        if let Some(ms) = self.read_cache_ms {
            write!(path_buf, "&read_cache_ms={}", ms)?;
        }

        if let Some(ms) = self.auto_sync_read_ms {
            write!(path_buf, "&auto_sync_read_ms={}", ms)?;
        }

        if let Some(ms) = self.auto_sync_write_ms {
            write!(path_buf, "&auto_sync_write_ms={}", ms)?;
        }

        if let Some(group_id) = self.connection_group_id {
            write!(path_buf, "&connection_group_id={}", group_id)?;
        }

        if let Some(debug) = self.debug {
            let level = debug as u8;
            write!(path_buf, "&debug={}", level)?;
        }
        Ok(path_buf)
    }
}

/// library supported protocols
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Protocol {
    /// EIP protocol
    EIP,
    /// Modbus protocol
    ModBus,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::EIP => write!(f, "ab-eip"),
            Protocol::ModBus => write!(f, "modbus-tcp"),
        }
    }
}

/// modbus supported register
pub enum Register {
    /// coil registers
    Coil(u16),
    ///discrete inputs
    Discrete(u16),
    /// holding registers
    Holding(u16),
    /// input registers
    Input(u16),
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Register::Coil(v) => write!(f, "co{}", v),
            Register::Discrete(v) => write!(f, "di{}", v),
            Register::Holding(v) => write!(f, "hr{}", v),
            Register::Input(v) => write!(f, "ir{}", v),
        }
    }
}

/// plc kind, required for EIP protocol
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PlcKind {
    /// Tell the library that this tag is in a Control Logix-class PLC
    ControlLogix,
    /// Tell the library that this tag is in a PLC/5 PLC
    PLC5,
    /// Tell the library that this tag is in a SLC 500 PLC
    SLC500,
    /// Tell the library that this tag is in a Control Logix-class PLC using the PLC/5 protocol
    LogixPCCC,
    /// Tell the library that this tag is in a Micro800-class PLC
    Micro800,
    /// Tell the library that this tag is in a Micrologix PLC
    MicroLogix,
    /// Omron PLC. Synonym for omron-njnx, omron-nj, omron-nx, njnx, nx1p2
    Omron,
}

impl fmt::Display for PlcKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlcKind::ControlLogix => write!(f, "controllogix"),
            PlcKind::PLC5 => write!(f, "plc5"),
            PlcKind::SLC500 => write!(f, "slc500"),
            PlcKind::LogixPCCC => write!(f, "logixpccc"),
            PlcKind::Micro800 => write!(f, "micro800"),
            PlcKind::MicroLogix => write!(f, "micrologix"),
            PlcKind::Omron => write!(f, "omron-njnx"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_eip_builder() {
        let path = PathBuilder::default()
            .protocol(Protocol::EIP)
            .gateway("192.168.1.120")
            .plc(PlcKind::ControlLogix)
            .name("MyTag1")
            .element_size(16)
            .element_count(1)
            .path("1,0")
            .read_cache_ms(0)
            .connection_group(10)
            .build()
            .unwrap();
        assert_eq!(path, "protocol=ab-eip&plc=controllogix&gateway=192.168.1.120&path=1,0&name=MyTag1&elem_count=1&elem_size=16&read_cache_ms=0&connection_group_id=10");
    }

    #[test]
    fn test_modbus_builder() {
        let path = PathBuilder::default()
            .protocol(Protocol::ModBus)
            .gateway("192.168.1.120:502")
            .path("0")
            .register(Register::Coil(42))
            .element_size(16)
            .element_count(1)
            .read_cache_ms(0)
            .build()
            .unwrap();
        assert_eq!(path, "protocol=modbus-tcp&gateway=192.168.1.120:502&path=0&name=co42&elem_count=1&elem_size=16&read_cache_ms=0");
    }
}
