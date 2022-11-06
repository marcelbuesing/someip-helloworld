# Hello World

This demonstrates a Rust someip client using the [HelloWorld - CommonAPI Example](https://github.com/COVESA/capicxx-core-tools/tree/master/CommonAPI-Examples/E01HelloWorld) as a reference service.

It makes use of the following rust crates for someip:

- [someip_parse](https://github.com/JulianSchmid/someip-parse-rs) - for the someip header
- [serde_someip](https://github.com/MortronMeymo/serde_someip) - for serializing the request payload and deserializing the response payload

## Run it

You can run the common api example server like this:

1. Change the "unicast" address in tests/vsomeip-service.json and the "SomeIpUnicastAddress" in E01HelloWorld-SomeIP.fdepl to match your local ip address. 

2. Build and run docker container:

```
cd tests
docker build -t vsomeip-hello-world -f vsomeip-hello-world.dockerfile .
docker run -v /tmp:/tmp -v "$PWD/vsomeip-service.json":"/capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/vsomeip-service.json" -p 31000:31000 -p 30509:30509 -p 30490:30490 --network host vsomeip-hello-world
```

Now run the rust client:

```
cargo run
```

Expected output:

```
Sending SayHello Request
Result: Ok(SayHelloResponse("Hello John Doe!"))
```

With `RUST_LOG=trace cargo run`:

```
2022-11-05T18:52:36.732497Z TRACE connect:find_service{opt=FindServiceOpt { sd_multicast_addr: 224.244.224.245:30490, service_id: 4660, instance_id: 22136, major_version: 0, ttl: 16777215, minor_version: 1 }}: someip_helloworld: Binding udp socket to multicast addr: 224.244.224.245:30490
2022-11-05T18:52:36.732585Z TRACE connect:find_service{opt=FindServiceOpt { sd_multicast_addr: 224.244.224.245:30490, service_id: 4660, instance_id: 22136, major_version: 0, ttl: 16777215, minor_version: 1 }}: mio::poll: registering event source with poller: token=Token(1), interests=READABLE | WRITABLE    
2022-11-05T18:52:36.732764Z TRACE connect:find_service{opt=FindServiceOpt { sd_multicast_addr: 224.244.224.245:30490, service_id: 4660, instance_id: 22136, major_version: 0, ttl: 16777215, minor_version: 1 }}: mio::poll: registering event source with poller: token=Token(2), interests=READABLE | WRITABLE    
2022-11-05T18:52:36.732823Z TRACE connect:find_service{opt=FindServiceOpt { sd_multicast_addr: 224.244.224.245:30490, service_id: 4660, instance_id: 22136, major_version: 0, ttl: 16777215, minor_version: 1 }}: someip_helloworld: Sending FIND FindServiceOpt { sd_multicast_addr: 224.244.224.245:30490, service_id: 4660, instance_id: 22136, major_version: 0, ttl: 16777215, minor_version: 1 }
2022-11-05T18:52:36.733047Z TRACE connect:find_service{opt=FindServiceOpt { sd_multicast_addr: 224.244.224.245:30490, service_id: 4660, instance_id: 22136, major_version: 0, ttl: 16777215, minor_version: 1 }}: mio::poll: deregistering event source from poller    
2022-11-05T18:52:36.733157Z TRACE connect: someip_helloworld: Received SD message, header: SomeIpHeader { message_id: 4294934784, length: 36, request_id: 1, interface_version: 1, message_type: Notification, return_code: 0, tp_header: None }, sd header: SdHeader { flags: SdHeaderFlags { reboot: false, unicast: true, explicit_initial_data_control: true }, entries: [Service(ServiceEntry { _type: FindService, index_first_option_run: 0, index_second_option_run: 0, number_of_options_1: 0, number_of_options_2: 0, service_id: 4660, instance_id: 22136, major_version: 0, ttl: 16777215, minor_version: 1 })], options: [] }
2022-11-05T18:52:37.059201Z TRACE connect: someip_helloworld: Received SD message, header: SomeIpHeader { message_id: 4294934784, length: 60, request_id: 103, interface_version: 1, message_type: Notification, return_code: 0, tp_header: None }, sd header: SdHeader { flags: SdHeaderFlags { reboot: true, unicast: true, explicit_initial_data_control: false }, entries: [Service(ServiceEntry { _type: OfferService, index_first_option_run: 0, index_second_option_run: 0, number_of_options_1: 2, number_of_options_2: 0, service_id: 4660, instance_id: 22136, major_version: 0, ttl: 16777215, minor_version: 1 })], options: [Ipv4Endpoint(Ipv4EndpointOption { ipv4_address: [192, 168, 0, 53], transport_protocol: Udp, port: 31000 }), Ipv4Endpoint(Ipv4EndpointOption { ipv4_address: [192, 168, 0, 53], transport_protocol: Tcp, port: 30509 })] }
2022-11-05T18:52:37.059345Z TRACE connect: someip_helloworld: Found matching endpoint: Ipv4EndpointOption { ipv4_address: [192, 168, 0, 53], transport_protocol: Tcp, port: 30509 }
2022-11-05T18:52:37.059534Z TRACE connect: mio::poll: registering event source with poller: token=Token(16777218), interests=READABLE | WRITABLE    
2022-11-05T18:52:37.059651Z TRACE connect: mio::poll: deregistering event source from poller    
Sending SayHello Request
2022-11-05T18:52:37.065320Z TRACE say_hello{self=E01HelloWorldClient { tcp_client: SomeIpTcpClient { tcp_stream: PollEvented { io: Some(TcpStream { addr: 1.2.3.4:37178, peer: 1.2.3.4:30509, fd: 10 }) }, buffer: b"" } } req=SayHelloRequest("John Doe")}: someip_helloworld: Deserialized response=Ok(SayHelloResponse("Hello John Doe!"))
Result: Ok(SayHelloResponse("Hello John Doe!"))
```