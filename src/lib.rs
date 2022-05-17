use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use serde_someip::{SomeIp, SomeIpOptions};
use someip_parse::{MessageType, ReturnCode, SliceIterator, SomeIpHeader};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};
use tracing::debug;

#[derive(Debug, thiserror :: Error)]
pub enum SomeIpClientError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Request returned error")]
    ErrorResponse,
}

#[derive(Serialize, Deserialize, Debug, SomeIp)]
#[someip(max_size = 13usize)]
pub struct SayHelloRequest(pub String);

#[derive(Serialize, Deserialize, Debug, SomeIp)]
#[someip(max_size = 13usize)]
pub struct SayHelloResponse(pub String);

#[derive(Debug)]
pub struct E01HelloWorldClient {
    tcp_stream: TcpStream,
    buffer: BytesMut,
}

impl E01HelloWorldClient {
    pub async fn connect<T: ToSocketAddrs>(service_addr: T) -> Result<Self, SomeIpClientError> {
        let tcp_stream = TcpStream::connect(service_addr).await?;
        Ok(Self {
            tcp_stream,
            buffer: BytesMut::new(),
        })
    }

    #[tracing::instrument]
    pub async fn say_hello(
        &mut self,
        req: SayHelloRequest,
    ) -> Result<SayHelloResponse, SomeIpClientError> {
        let req_serialized = serde_someip::to_vec::<VSomeIpDefaultOptions, _>(&req).unwrap();

        // Create header for request
        let mut someip_hdr = SomeIpHeader {
            message_id: 0,
            length: 4 + 1 + 1 + 1 + 1 + req_serialized.len() as u32,
            request_id: 0x1343 << 16 | 0x0001,
            interface_version: 0x00, // Major version of interface
            message_type: MessageType::Request,
            return_code: ReturnCode::Ok.into(),
            tp_header: None,
        };
        someip_hdr.set_service_id(4660);
        someip_hdr.set_method_id(30000);

        let hdr_serialized = &someip_hdr.base_to_bytes();

        // Write header and request payload
        self.tcp_stream
            .write_all(&[hdr_serialized, req_serialized.as_slice()].concat())
            .await?;

        loop {
            if 0 == self.tcp_stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    panic!("Not properly handled");
                } else {
                    panic!("Not properly handled peer reset");
                }
            }

            for someip_message in SliceIterator::new(&self.buffer) {
                match someip_message {
                    Ok(header) => {
                        if header.length() + 8 != self.buffer.len() as u32 {
                            println!(
                                "Insufficient content in buffer, expected {}, actual {}",
                                header.length() + 8,
                                self.buffer.len()
                            );
                        } else {
                            if header.message_type() == MessageType::Error {
                                return Err(SomeIpClientError::ErrorResponse);
                            } else if header.message_type() == MessageType::Response {
                                let deserialized =
                                    serde_someip::from_slice::<
                                        VSomeIpDefaultOptions,
                                        SayHelloResponse,
                                    >(header.payload())
                                    .unwrap();
                                debug!("Deserialized response={:?}", deserialized);
                                return Ok(deserialized);
                            } else {
                                println!("Skipping message type: {:#?}", header.message_type());
                            }
                        }
                    }
                    Err(_) => eprintln!("Failed to decode packet"),
                }
            }
        }
    }
}
pub struct VSomeIpDefaultOptions {}

impl SomeIpOptions for VSomeIpDefaultOptions {
    const BYTE_ORDER: serde_someip::options::ByteOrder =
        serde_someip::options::ByteOrder::BigEndian;

    // https://github.com/COVESA/capicxx-someip-runtime/blob/0ad2bdc1807fc0f078b9f9368a47ff2f3366ed13/src/CommonAPI/SomeIP/OutputStream.cpp#L368
    const STRING_WITH_BOM: bool = true;

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
