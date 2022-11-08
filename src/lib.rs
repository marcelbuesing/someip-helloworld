use async_stream::try_stream;
use bytes::BytesMut;
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_someip::{SomeIp, SomeIpOptions};
use socket2::{Domain, Protocol, Socket, Type};
use someip_parse::{
    MessageType, ReturnCode, SdEntry, SdHeader, SdOption, SdServiceEntryType, SliceIterator,
    SomeIpHeader, TransportProtocol,
};
use std::future;
use std::time::Duration;
use std::{
    io::Cursor,
    net::{Ipv4Addr, SocketAddrV4},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs, UdpSocket};
use tokio::time::error::Elapsed;
use tokio::time::timeout;
use tracing::{error, trace};

#[derive(Debug, thiserror :: Error)]
pub enum SomeIpClientError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Request returned error")]
    ErrorResponse,
    #[error("Read returned error")]
    ReadError(someip_parse::ReadError),
    #[error("Invalid value error")]
    ValueError(someip_parse::ValueError),
    #[error("Timeout: Failed to receive a SomeIP OfferService")]
    FindServiceTimeout(#[from] Elapsed),
}

#[derive(Debug)]
struct SomeIpTcpClient {
    tcp_stream: TcpStream,
    buffer: BytesMut,
}

impl SomeIpTcpClient {
    pub async fn connect<T: ToSocketAddrs>(service_addr: T) -> Result<Self, SomeIpClientError> {
        let tcp_stream = TcpStream::connect(service_addr).await?;
        Ok(Self {
            tcp_stream,
            buffer: BytesMut::new(),
        })
    }
    pub async fn request<'a, TReq, TResp>(
        &'a mut self,
        service_id: u16,
        method_id: u16,
        request_id: u16,
        interface_version: u8,
        req: &TReq,
    ) -> Result<TResp, SomeIpClientError>
    where
        TReq: Serialize + SomeIp,
        TResp: DeserializeOwned + SomeIp + ?Sized,
    {
        let req_serialized = serde_someip::to_vec::<VSomeIpSeOptions, _>(req).unwrap();

        // Create header for request
        let mut someip_hdr = SomeIpHeader {
            message_id: 0,
            length: 4 + 1 + 1 + 1 + 1 + req_serialized.len() as u32,
            request_id: (request_id as u32) << 16 | 0x0001,
            interface_version, // Major version of interface
            message_type: MessageType::Request,
            return_code: ReturnCode::Ok.into(),
            tp_header: None,
        };
        someip_hdr.set_service_id(service_id);
        someip_hdr.set_method_id(method_id);

        let hdr_serialized = &someip_hdr.base_to_bytes();

        // Write header and request payload
        self.tcp_stream
            .write_all(&[hdr_serialized, req_serialized.as_slice()].concat())
            .await?;

        loop {
            let len = self.tcp_stream.read_buf(&mut self.buffer).await?;
            if 0 == len {
                if self.buffer.is_empty() {
                    panic!("Not properly handled");
                } else {
                    panic!("Not properly handled peer reset");
                }
            }

            let someip_message = SliceIterator::new(&self.buffer).next();

            match someip_message {
                Some(Ok(header)) => {
                    if header.length() + 8 != self.buffer.len() as u32 {
                        error!(
                            "Insufficient content in buffer, expected {}, actual {}",
                            header.length() + 8,
                            self.buffer.len()
                        );
                    } else if header.message_type() == MessageType::Error {
                        return Err(SomeIpClientError::ErrorResponse);
                    } else if header.message_type() == MessageType::Response {
                        let deserialized =
                            serde_someip::from_slice::<VSomeIpDeOptions, _>(header.payload())
                                .unwrap();
                        return Ok(deserialized);
                    } else {
                        trace!("Skipping message type: {:#?}", header.message_type());
                    }
                }
                Some(Err(read_error)) => return Err(SomeIpClientError::ReadError(read_error)),
                // Continue reading
                // TODO handle timeout
                None => continue,
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, SomeIp)]
#[someip(max_size = 20usize)]
pub struct SayHelloRequest(pub String);

#[derive(Serialize, Deserialize, Debug, SomeIp)]
#[someip(max_size = 20usize)]
pub struct SayHelloResponse(pub String);

#[derive(Debug)]
pub struct E01HelloWorldClient {
    tcp_client: SomeIpTcpClient,
}

impl E01HelloWorldClient {
    // https://github.com/COVESA/capicxx-core-tools/blob/1e6e696fcf3a0dcf470e0d3a761cffd85321609f/CommonAPI-Examples/E01HelloWorld/fidl/E01HelloWorld-SomeIP.fdepl#L10
    pub const SERVICE_ID: u16 = 4660;
    // https://github.com/COVESA/capicxx-core-tools/blob/1e6e696fcf3a0dcf470e0d3a761cffd85321609f/CommonAPI-Examples/E01HelloWorld/fidl/E01HelloWorld-SomeIP.fdepl#L28
    pub const INSTANCE_ID: u16 = 22136;
    // https://github.com/COVESA/capicxx-core-tools/blob/1e6e696fcf3a0dcf470e0d3a761cffd85321609f/CommonAPI-Examples/E01HelloWorld/fidl/E01HelloWorld.fidl#L9
    pub const MAJOR_VERSION: u8 = 0;
    // https://github.com/COVESA/capicxx-core-tools/blob/1e6e696fcf3a0dcf470e0d3a761cffd85321609f/CommonAPI-Examples/E01HelloWorld/fidl/E01HelloWorld.fidl#L9
    pub const MINOR_VERSION: u32 = 1;

    /// Directly connect to a service instance without SomeIP Service Discovery.
    pub async fn connect_with_addr<T: ToSocketAddrs>(
        service_addr: T,
    ) -> Result<Self, SomeIpClientError> {
        let tcp_client = SomeIpTcpClient::connect(service_addr).await?;
        Ok(Self { tcp_client })
    }

    /// Tries to find a service instance via SomeIP Service Discovery.
    /// Connects once an instance is found (an ``OfferService was received).
    #[tracing::instrument]
    pub async fn connect() -> Result<Self, SomeIpClientError> {
        let find_opts = FindServiceOpt {
            service_id: Self::SERVICE_ID,
            instance_id: Self::INSTANCE_ID,
            major_version: Self::MAJOR_VERSION,
            minor_version: Self::MINOR_VERSION,
            ..Default::default()
        };

        // Send FIND
        let instances = Self::find_service(&find_opts).await?;

        pin_mut!(instances);

        let mut matching_instances = instances.try_filter_map(|(someip_header, sd_header)| {
            trace!(
                "Received SD message, header: {:?}, sd header: {:?}",
                someip_header,
                sd_header
            );

            // Checks if the service entry contains the expected service details
            let matching_sd_entry = sd_header.entries.iter().any(|sd_entry| match sd_entry {
                SdEntry::Service(sd_entry) => {
                    sd_entry._type == SdServiceEntryType::OfferService
                        && sd_entry.service_id == Self::SERVICE_ID
                        && sd_entry.instance_id == Self::INSTANCE_ID
                        && sd_entry.major_version == Self::MAJOR_VERSION
                        && sd_entry.minor_version == Self::MINOR_VERSION
                }
                SdEntry::Eventgroup(_event_group) => false,
            });

            if matching_sd_entry {
                // Check for sd option that matches our expected transport protocol
                let opt = sd_header.options.into_iter().find_map(|sd_option| {
                    if let SdOption::Ipv4Endpoint(ipv4_endpoint_opt) = sd_option {
                        // TODO UDP support
                        if ipv4_endpoint_opt.transport_protocol == TransportProtocol::Tcp {
                            Some(ipv4_endpoint_opt)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });
                future::ready(Ok(opt))
            } else {
                future::ready(Ok(None))
            }
        });

        // Find first sd header that matches our requirements

        let ipv4_endpoint_opt = timeout(Duration::from_secs(5), matching_instances.next())
            .await?
            .expect("Stream terminated")?;

        trace!("Found matching endpoint: {:?}", ipv4_endpoint_opt);

        let addr = SocketAddrV4::new(
            Ipv4Addr::from(ipv4_endpoint_opt.ipv4_address),
            ipv4_endpoint_opt.port,
        );

        let client = Self::connect_with_addr(addr).await?;
        Ok(client)
    }

    /// Sends out a SomeIP Service Discovery `FindService`.
    #[tracing::instrument]
    pub async fn find_service(
        opt: &FindServiceOpt,
    ) -> Result<
        impl Stream<Item = Result<(SomeIpHeader, SdHeader), SomeIpClientError>>,
        SomeIpClientError,
    > {
        let inaddr_any = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0);

        trace!(
            "Binding udp socket to multicast addr: {}",
            opt.sd_multicast_addr
        );

        // Setup reading socket
        let read_socket = UdpSocket::bind(&opt.sd_multicast_addr).await?;
        read_socket.join_multicast_v4(*opt.sd_multicast_addr.ip(), *inaddr_any.ip())?;

        // Tokio's UdpSocket does not directly offer "set_reuse_address", go with socket2
        let udp_socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        udp_socket.set_reuse_address(true)?;
        udp_socket.bind(&inaddr_any.into())?;

        let udp_socket = UdpSocket::from_std(udp_socket.into())?;
        udp_socket.connect(&opt.sd_multicast_addr).await?;
        udp_socket.set_multicast_ttl_v4(10)?;
        udp_socket.join_multicast_v4(*opt.sd_multicast_addr.ip(), *inaddr_any.ip())?;
        trace!("Sending FIND {opt:?}");

        let message_bytes = Self::find_service_message(opt)?;
        udp_socket.send(&message_bytes).await?;

        let mut buffer = Vec::new();
        buffer.resize(1500, 0x00);

        let s = try_stream! {
            loop {
                let (_received, _addr) = read_socket.recv_from(&mut buffer).await?;

                let mut cursor = Cursor::new(&buffer);
                let someip_header = SomeIpHeader::read(&mut cursor).map_err(SomeIpClientError::ReadError)?;

                if someip_header.is_someip_sd() {
                    let someip_sd = SdHeader::read(&mut cursor).map_err(SomeIpClientError::ReadError)?;
                        // .map_err(|err| anyhow!("Failed to read someip sd header: {:?}", err))?;

                    yield (someip_header, someip_sd);
                } else {
                    continue;
                }
            }
        };
        Ok(s)
    }

    #[tracing::instrument]
    fn find_service_message(opt: &FindServiceOpt) -> Result<Vec<u8>, SomeIpClientError> {
        // Init someip sd header
        let find_service = SdEntry::new_find_service_entry(
            0,
            0,
            0,
            0,
            opt.service_id,
            opt.instance_id,
            opt.major_version,
            opt.ttl,
            opt.minor_version,
        )
        .map_err(SomeIpClientError::ValueError)?;

        let entries = vec![find_service];
        let someip_sd_header = SdHeader::new(false, entries, vec![]);
        let someip_sd_header_bytes = someip_sd_header.to_bytes_vec().unwrap();

        // Init someip header
        let length = (4 + 1 + 1 + 1 + 1 + someip_sd_header_bytes.len()) as u32;
        let someip_header = SomeIpHeader::new_sd_header(length, 0x01, None);
        let someip_header_bytes = someip_header.base_to_bytes();

        // Combine someip header and someip sd header
        Ok([&someip_header_bytes[..], &someip_sd_header_bytes].concat())
    }

    #[tracing::instrument]
    pub async fn say_hello(
        &mut self,
        req: &SayHelloRequest,
    ) -> Result<SayHelloResponse, SomeIpClientError> {
        // https://github.com/COVESA/capicxx-core-tools/blob/1e6e696fcf3a0dcf470e0d3a761cffd85321609f/CommonAPI-Examples/E01HelloWorld/fidl/E01HelloWorld-SomeIP.fdepl#L13
        const METHOD_ID: u16 = 30000;

        let resp = self
            .tcp_client
            .request(
                Self::SERVICE_ID,
                METHOD_ID,
                0x1343,
                Self::MAJOR_VERSION,
                req,
            )
            .await;
        trace!("Deserialized response={:?}", resp);
        resp
    }
}

/// Serialization Options
pub struct VSomeIpSeOptions {}

impl SomeIpOptions for VSomeIpSeOptions {
    const BYTE_ORDER: serde_someip::options::ByteOrder =
        serde_someip::options::ByteOrder::BigEndian;

    // https://github.com/COVESA/capicxx-someip-runtime/blob/0ad2bdc1807fc0f078b9f9368a47ff2f3366ed13/src/CommonAPI/SomeIP/OutputStream.cpp#L368
    const STRING_WITH_BOM: bool = true;

    const STRING_ENCODING: serde_someip::options::StringEncoding =
        serde_someip::options::StringEncoding::Utf16Le;

    // https://github.com/COVESA/capicxx-someip-runtime/blob/0ad2bdc1807fc0f078b9f9368a47ff2f3366ed13/src/CommonAPI/SomeIP/OutputStream.cpp#L228
    const STRING_WITH_TERMINATOR: bool = true;

    // https://github.com/COVESA/capicxx-someip-runtime/blob/0ad2bdc1807fc0f078b9f9368a47ff2f3366ed13/src/CommonAPI/SomeIP/OutputStream.cpp#L246
    const DEFAULT_LENGTH_FIELD_SIZE: Option<serde_someip::length_fields::LengthFieldSize> =
        Some(serde_someip::length_fields::LengthFieldSize::FourBytes);

    const SERIALIZER_USE_LEGACY_WIRE_TYPE: bool = true;

    const SERIALIZER_LENGTH_FIELD_SIZE_SELECTION: serde_someip::options::LengthFieldSizeSelection =
        serde_someip::options::LengthFieldSizeSelection::AsConfigured;

    const DESERIALIZER_STRICT_BOOL: bool = false;

    const DESERIALIZER_ACTION_ON_TOO_MUCH_DATA: serde_someip::options::ActionOnTooMuchData =
        serde_someip::options::ActionOnTooMuchData::Fail;
}

/// Deserialization Options
pub struct VSomeIpDeOptions {}

impl SomeIpOptions for VSomeIpDeOptions {
    const BYTE_ORDER: serde_someip::options::ByteOrder =
        serde_someip::options::ByteOrder::BigEndian;

    // https://github.com/COVESA/capicxx-someip-runtime/blob/0ad2bdc1807fc0f078b9f9368a47ff2f3366ed13/src/CommonAPI/SomeIP/OutputStream.cpp#L368
    const STRING_WITH_BOM: bool = true;

    // The Hello World Service deployment file does not specify an encoding for the response string
    // commonapi-someip runtime therefor defaults to utf8.
    const STRING_ENCODING: serde_someip::options::StringEncoding =
        serde_someip::options::StringEncoding::Utf8;

    // https://github.com/COVESA/capicxx-someip-runtime/blob/0ad2bdc1807fc0f078b9f9368a47ff2f3366ed13/src/CommonAPI/SomeIP/OutputStream.cpp#L228
    const STRING_WITH_TERMINATOR: bool = true;

    // https://github.com/COVESA/capicxx-someip-runtime/blob/0ad2bdc1807fc0f078b9f9368a47ff2f3366ed13/src/CommonAPI/SomeIP/OutputStream.cpp#L246
    const DEFAULT_LENGTH_FIELD_SIZE: Option<serde_someip::length_fields::LengthFieldSize> =
        Some(serde_someip::length_fields::LengthFieldSize::FourBytes);

    const SERIALIZER_USE_LEGACY_WIRE_TYPE: bool = true;

    const SERIALIZER_LENGTH_FIELD_SIZE_SELECTION: serde_someip::options::LengthFieldSizeSelection =
        serde_someip::options::LengthFieldSizeSelection::AsConfigured;

    const DESERIALIZER_STRICT_BOOL: bool = false;

    const DESERIALIZER_ACTION_ON_TOO_MUCH_DATA: serde_someip::options::ActionOnTooMuchData =
        serde_someip::options::ActionOnTooMuchData::Fail;
}

#[derive(Debug, Clone)]
pub struct FindServiceOpt {
    pub sd_multicast_addr: SocketAddrV4,
    pub service_id: u16,
    pub instance_id: u16,
    pub major_version: u8,
    pub ttl: u32,
    pub minor_version: u32,
}

impl Default for FindServiceOpt {
    fn default() -> Self {
        Self {
            // https://github.com/COVESA/vsomeip/blob/13f9c89ced6ffaeb1faf485152e27e1f40d234cd/implementation/service_discovery/include/defines.hpp#L38
            sd_multicast_addr: SocketAddrV4::new(Ipv4Addr::new(224, 244, 224, 245), 30490),
            service_id: u16::MAX,
            // https://github.com/COVESA/vsomeip/blob/13f9c89ced6ffaeb1faf485152e27e1f40d234cd/interface/vsomeip/constants.hpp#L28
            instance_id: u16::MAX,
            major_version: u8::MAX,
            // https://github.com/COVESA/vsomeip/blob/13f9c89ced6ffaeb1faf485152e27e1f40d234cd/interface/vsomeip/constants.hpp#L18
            ttl: 0xFFFF_FF,
            minor_version: u32::MAX,
        }
    }
}
