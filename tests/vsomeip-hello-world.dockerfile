FROM debian:11-slim AS builder

# Just a random diagnosis address
ARG vsomeip_ddiagnosis_address=0x12
ARG capicxx_core_tools_version=3.2.0.1
ARG capicxx_someip_tools_version=3.2.0.1
ARG capicxx_dbus_tools_version=3.2.0

RUN apt update && \
    apt install -y \
        build-essential \
        git \
        wget \
        unzip \
        libcurl4-openssl-dev \
        cmake \
        libboost-all-dev \
        pkg-config \
        default-jre

RUN git clone --depth 1 https://github.com/GENIVI/vsomeip.git /tmp/vsomeip && \
    mkdir -p /tmp/vsomeip/build && \
    cd /tmp/vsomeip/build && \
    cmake -DENABLE_SIGNAL_HANDLING=1 -DDIAGNOSIS_ADDRESS=$vsomeip_ddiagnosis_address .. && \
    make && \
    make install

RUN git clone --depth 1 https://github.com/GENIVI/capicxx-core-runtime.git /tmp/capicxx-core-runtime && \
    mkdir -p /tmp/capicxx-core-runtime/build && \
    cd /tmp/capicxx-core-runtime/build && \
    cmake .. && \
    make && \
    make install

RUN git clone --depth 1 https://github.com/GENIVI/capicxx-someip-runtime.git /tmp/capicxx-someip-runtime && \
    mkdir -p /tmp/capicxx-someip-runtime/build && \
    cd /tmp/capicxx-someip-runtime/build && \
    cmake -DUSE_INSTALLED_COMMONAPI=ON .. && \
    make && \
    make install

RUN git clone --depth 1 https://github.com/COVESA/capicxx-dbus-runtime.git /tmp/capicxx-dbus-runtime && \
    # capicxx-dbus-runtime is missing documentation what libdbus version the patches were intended for
    # so later versions might also work
    wget -P /tmp https://dbus.freedesktop.org/releases/dbus/dbus-1.9.20.tar.gz && \
    cd /tmp && \
    tar -xf dbus-1.9.20.tar.gz && \
    cd dbus-1.9.20 && \
    patch -f -p1 < /tmp/capicxx-dbus-runtime/src/dbus-patches/capi-dbus-1-pc.patch && \
    patch -f -p1 < /tmp/capicxx-dbus-runtime/src/dbus-patches/capi-dbus-add-send-with-reply-set-notify.patch && \
    patch -f -p1 < /tmp/capicxx-dbus-runtime/src/dbus-patches/capi-dbus-add-support-for-custom-marshalling.patch && \
    patch -f -p1 < /tmp/capicxx-dbus-runtime/src/dbus-patches/capi-dbus-block-acquire-io-path-on-send.patch && \
    patch -f -p1 < /tmp/capicxx-dbus-runtime/src/dbus-patches/capi-dbus-correct-dbus-connection-block-pending-call.patch && \
    patch -f -p1 < /tmp/capicxx-dbus-runtime/src/dbus-patches/capi-dbus-send-with-reply-and-block-delete-reply-on-error.patch && \
    ./configure --prefix=/usr/local && \
    make -C dbus && \
    make -C dbus install && \
    make install-pkgconfigDATA

RUN mkdir -p /tmp/capicxx-dbus-runtime/build && \
    cd /tmp/capicxx-dbus-runtime/build && \
    cmake -DUSE_INSTALLED_COMMONAPI=ON .. && \
    make && \
    make install

RUN wget -P /tmp/commonapi_core_generator https://github.com/GENIVI/capicxx-core-tools/releases/download/$capicxx_core_tools_version/commonapi_core_generator.zip && \
    cd /tmp/commonapi_core_generator && \
    unzip commonapi_core_generator.zip

RUN wget -P /tmp/commonapi_someip_generator https://github.com/GENIVI/capicxx-someip-tools/releases/download/$capicxx_someip_tools_version/commonapi_someip_generator.zip && \
    cd /tmp/commonapi_someip_generator && \
    unzip commonapi_someip_generator.zip

RUN wget -P /tmp/commonapi_dbus_generator https://github.com/COVESA/capicxx-dbus-tools/releases/download/3.2.0/commonapi_dbus_generator.zip && \
    cd /tmp/commonapi_dbus_generator && \
    unzip commonapi_dbus_generator.zip

RUN git clone --depth 1 https://github.com/COVESA/capicxx-core-tools.git /capicxx-core-tools

# is missing in the repository so is basically an empty place holder to make the build work
COPY E01HelloWorld-DBus.fdepl /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/fidl/E01HelloWorld-DBus.fdepl
# adapted "name" SomeIpStringEncoding to utf8 due to client issues with the default utf16le encoding.
COPY E01HelloWorld-SomeIP.fdepl /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/fidl/E01HelloWorld-SomeIP.fdepl

RUN ./tmp/commonapi_core_generator/commonapi-core-generator-linux-x86_64 -d /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/src-gen/core -sk /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/fidl/E01HelloWorld.fidl && \
    ./tmp/commonapi_someip_generator/commonapi-someip-generator-linux-x86_64 -d /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/src-gen/someip /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/fidl/E01HelloWorld-SomeIP.fdepl && \
    ./tmp/commonapi_dbus_generator/commonapi-dbus-generator-linux-x86_64 -d /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/src-gen/dbus /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/fidl/E01HelloWorld-DBus.fdepl && \
    mkdir -p /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/build && \
    cd /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/build && \
    cp /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/commonapi4someip.ini /etc/commonapi.ini && \
    cmake -DUSE_INSTALLED_COMMONAPI=ON -DUSE_INSTALLED_DBUS=OFF .. && \
    make

WORKDIR /capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/build/

ENV VSOMEIP_CONFIGURATION=/capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/vsomeip-service.json
ENV COMMONAPI_CONFIG=/capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/commonapi4someip.ini

RUN cp libE01HelloWorld-someip.so /usr/lib/libE01HelloWorld-someip.so
# Library is not found otherwise
RUN ldconfig

# The following seems to have no impact, and leads to cannot open shared object file: No such file or directory if the
# library is not copied to /usr/lib
RUN export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:/capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/build

CMD ["/capicxx-core-tools/CommonAPI-Examples/E01HelloWorld/build/E01HelloWorldService"]
