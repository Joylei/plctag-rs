use crate::DebugLevel;
use std::collections::HashMap;
use std::fmt;

#[derive(Default, Debug)]
pub struct PathBuilder {
    debug: Option<DebugLevel>,
    protocol: Option<Protocol>,
    elem_count: Option<usize>,
    elem_size: Option<usize>,
    read_cache_ms: Option<usize>,
    plc: Option<PlcKind>,
    name: Option<String>,
    path: Option<String>,
    gateway: Option<String>,
    use_connected_msg: Option<bool>,
}

impl PathBuilder {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// generic attribute.
    /// defining the current debugging level.
    #[deprecated]
    #[inline]
    pub fn debug(&mut self, level: DebugLevel) -> &Self {
        self.debug = Some(level);
        &self
    }

    /// generic attribute.
    /// Required. Determines the type of the PLC protocol.
    #[inline]
    pub fn protocol(&mut self, protocol:Protocol) -> &Self {
        self.protocol = Some(protocol);
        &self
    }

    /// generic attribute.
    ///  Optional. All tags are treated as arrays. Tags that are not arrays are considered to have a length of one element. This attribute determines how many elements are in the tag. Defaults to one (1)
    #[inline]
    pub fn element_count(&mut self, count: usize) -> &Self {
        self.elem_count = Some(count);
        &self
    } 

    /// generic attribute
    /// Required for some protocols or PLC types. This attribute determines the size of a single element of the tag. All tags are considered to be arrays, even those with only one entry. Ignored for Modbus and for ControlLogix-class Allen-Bradley PLCs. This parameter will become optional for as many PLC types as possible
    #[inline]
    pub fn element_size(&mut self, size: usize) -> &Self {
        self.elem_size = Some(size);
        &self
    } 

    /// generic attribute:
    ///  Optional. An integer number of milliseconds to cache read data.
    /// Use this attribute to cause the tag read operations to cache data the requested number of milliseconds. This can be used to lower the actual number of requests against the PLC. Example read_cache_ms=100 will result in read operations no more often than once every 100 milliseconds.
    #[inline]
    pub fn read_cache_ms(&mut self, millis: usize) -> &Self {
        self.read_cache_ms = Some(millis);
        &self
    } 

    /// Required for EIP. Determines the type of the PLC
    #[inline]
    pub fn plc(&mut self, plc:PlcKind) -> &Self {
        self.plc = Some(plc);
        &self
    }

    /// - EIP
    /// IP address or host name.
    /// This tells the library what host name or IP address to use for the PLC or the gateway to the PLC (in the case that the PLC is remote).
    /// - ModBus
    /// Required IP address or host name and optional port
    /// This tells the library what host name or IP address to use for the PLC. Can have an optional port at the end, e.g. gateway=10.1.2.3:502 where the :502 part specifies the port.
    #[inline]
    pub fn gateway(&mut self, gateway: &str) -> &Self {
        self.gateway = Some(gateway.to_string());
        &self
    }

    /// - EIP
    /// This is the full name of the tag. For program tags, prepend Program:<program name>. where <program name> is the name of the program in which the tag is created
    /// - ModBus
    /// Required the type and first register number of a tag, e.g. co42 for coil 42 (counts from zero).
    /// The supported register type prefixes are co for coil, di for discrete inputs, hr for holding registers and ir for input registers. The type prefix must be present and the register number must be greater than or equal to zero and less than or equal to 65535. Modbus examples: co21 - coil 21, di22 - discrete input 22, hr66 - holding register 66, ir64000 - input register 64000.
    #[inline]
    pub fn name(&mut self, name: &str) -> &Self {
        self.name = Some(name.to_string());
        &self
    }

    /// - EIP
    /// AB: CIP path to PLC CPU. I.e. 1,0.	
    /// This attribute is required for CompactLogix/ControlLogix tags and for tags using a DH+ protocol bridge (i.e. a DHRIO module) to get to a PLC/5, SLC 500, or MicroLogix PLC on a remote DH+ link. The attribute is ignored if it is not a DH+ bridge route, but will generate a warning if debugging is active. Note that Micro800 connections must not have a path attribute.
    /// - ModBus
    /// Required The server/unit ID. Must be an integer value between 0 and 255.
    /// Servers may support more than one unit or may bridge to other units.
    #[inline]
    pub fn path(&mut self, path: &str) -> &Self {
        self.path = Some(path.to_string());
        &self
    }

    /// EIP only
    /// Optional 1 = use CIP connection, 0 = use UCMM.
    /// Control whether to use connected or unconnected messaging. Only valid on Logix-class PLCs. Connected messaging is required on Micro800 and DH+ bridged links. Default is PLC-specific and link-type specific. Generally you do not need to set this.
    #[inline]
    pub fn use_connected_msg(&mut self, yes: bool) -> &Self {
        self.use_connected_msg = Some(yes);
        &self
    }

    /// check required attributes or conflict attributes
    fn check()->std::result::Result<(),String> {
        //check protocol, required
        if self.protocol.is_none() {
            return Err("protocol required");
        }

        let protocol = self.protocol.unwrap();
        // check required attributes
        match protocol{
            Protocol::EIP => {
                //TODO: check gateway, either ip or host name
                //check plc, required
                if self.plc.is_none() {
                    return Err("plc required");
                }
                let plc = self.plc.unwrap();
                if plc == PlcKind::ControlLogix {
                    if self.path.is_none() {
                        return Err("path required for controllogix");
                    }
                    return Ok(()); //skip check for elem_size
                } else if plc == PlcKind::Micro800 {
                    if self.path.is_some() {
                        return Err("path must not provided for micro800");
                    }
                }
                if self.elem_size is_none() {
                    return Err("element size required");
                }
            },
            Protocol::ModBus => {
                //TODO: check gateway, host with port
                if self.gateway.is_none() {
                    return Err("gateway required");
                }
                if self.name.is_none() {
                    return Err("name required");
                }
                if self.path.is_none() {
                    return Err("path required");
                }
                if self.elem_size is_none() {
                    return Err("element size required");
                }
            },
            _ => {},
        }
        Ok(())
    }

    pub fn build(&self) -> std::result::Result<String, String> {
        self.check()?;
        let mut path_buf = vec!();
        let protocol = self.protocol.unwrap();
        path_buf.push(format!("protocol={}", protocol));

        if let Some(plc) = self.plc {
            path_buf.push(format!("plc={}", plc));
        }
        if let Some(path) = self.path {
            path_buf.push(format!("path={}", path));
        }
        if let Some(gateway) = self.gateway {
            path_buf.push(format!("gateway={}", gateway));
        } 
        if let Some(name) = self.name {
            path_buf.push(format!("name={}", name));
        }
        if let Some(elem_count) = self.elem_count {
            path_buf.push(format!("elem_count={}", elem_count));
        }

        if let Some(elem_size) = self.elem_size {
            path_buf.push(format!("elem_size={}", elem_size));
        }

        if let Some(read_cache_ms) = self.read_cache_ms {
            path_buf.push(format!("read_cache_ms={}", read_cache_ms));
        }
        if let Some(yes) = self.use_connected_msg {
            path_buf.push(format!("use_connected_msg={}", if yes { 1 } else {0}));
        }
        if let Some(debug) = self.debug {
            path_buf.push(format!("debug={}", debug.into() as u8));
        }
        let buf = path_buf.join("&");
        Ok(buf.to_owned())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Protocol {
    EIP,
    ModBus,
}

impl fmt::Display for Protocol {
    format(&self, f: fmt::Formatter<_>) ->fmt::Result {
        match protocol {
            Protocol::EIP => write!(f, "ab-eip"),
            Protocol::ModBus => write!(f, "modbus-tcp"),
        }
    }
}

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
}

impl fmt::Display for PlcKind {
    fn format(&self, f: fmt::Formatter<_>) ->fmt::Result {
        match kind {
            PlcKind::ControlLogix => write!(f,"controllogix"),
            PlcKind::PLC5 => write!(f, "plc5"),
            PlcKind::SLC500 => write!(f, "slc500"),
            PlcKind::LogixPCCC => write!(f, "logixpccc"),
            PlcKind::Micro800 => write!(f,"micro800"),
            PlcKind::MicroLogix => write!(f,"micrologix"),
        }
    }
}

pub fn builder()-> PathBuilder{
    PathBuilder::new()
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_eip_builder(){
        let path = builder().protocol(Protocol::EIP)
            .gateway("192.168.1.120")
            .plc(PlcKind::ControlLogix)
            .name("MyTag1")
            .element_size(16)
            .element_count(1)
            .path("1,0")
            .read_cache_ms(0)
            .build();
        assert_eq!(path, "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16&read_cache_ms=0");
    }

    #[test]
    fn test_modbus_builder(){
        let path = builder().protocol(Protocol::ModBus)
        .gateway("192.168.1.120:8080")
        .plc(PlcKind::ControlLogix)
        .name("MyTag1")
        .element_size(16)
        .element_count(1)
        .read_cache_ms(0)
        .build();
    assert_eq!(path, "protocol=modbus-tcp&plc=controllogix&gateway=192.168.1.120:8080&name=MyTag1&elem_count=1&elem_size=16&read_cache_ms=0");
    }
}