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
RUST_LOG=debug cargo run
```

Expected output:

```
Sending SayHello Request
2022-05-17T19:39:57.833857Z DEBUG say_hello{self=E01HelloWorldClient { tcp_stream: PollEvented { io: Some(TcpStream { addr: 1.2.3.4:38952, peer: 1.2.3.4:30509, fd: 9 }) }, buffer: b"" } req=SayHelloRequest("Hi")}: someip_helloworld: Deserialized response=SayHelloResponse("Hello Hi!")
Result: Ok(SayHelloResponse("Hello Hi!"))
```
