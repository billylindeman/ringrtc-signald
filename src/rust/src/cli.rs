//
// Copyright 2019-2021 Signal Messenger, LLC
// SPDX-License-Identifier: AGPL-3.0-only
//

use log::{debug, info};

use ringrtc::{
    common::{
        actor::{Actor, Stopper},
        units::DataRate,
        CallId, CallMediaType, DeviceId, Result,
    },
    core::{bandwidth_mode::BandwidthMode, call_manager::CallManager, group_call, signaling},
    lite::{http, sfu::UserId},
    native::{
        CallState, CallStateHandler, GroupUpdate, GroupUpdateHandler, NativeCallContext,
        NativePlatform, PeerId, SignalingSender,
    },
    simnet::{
        router,
        router::{LinkConfig, Router},
    },
    webrtc::{
        injectable_network,
        injectable_network::InjectableNetwork,
        media::{VideoFrame, VideoPixelFormat, VideoSink, VideoSource},
        network::NetworkInterfaceType,
        peer_connection::AudioLevel,
        peer_connection_factory::{self as pcf, IceServer, PeerConnectionFactory},
        peer_connection_observer::NetworkRoute,
    },
};

use signald::types::{ClientMessageWrapperV1, SubscribeRequestV1};
use signald::Signald;

const ACCOUNT: &str = "+17346081614";

#[tokio::main]
async fn main() -> Result<()> {
    log::set_logger(&LOG).expect("set logger");
    log::set_max_level(log::LevelFilter::Debug);

    // Show WebRTC logs via application Logger while debugging.
    #[cfg(debug_assertions)]
    ringrtc::webrtc::logging::set_logger(log::LevelFilter::Debug);

    #[cfg(not(debug_assertions))]
    ringrtc::webrtc::logging::set_logger(log::LevelFilter::Warn);

    info!("connecting to signald");
    let mut socket = Signald::connect("/signald/signald.sock").await?;

    info!("subscribing to messages");
    let mut subscribe = SubscribeRequestV1::default();
    subscribe.account = Some(ACCOUNT.into());

    Ok(())
}

struct Log;

static LOG: Log = Log;

impl log::Log for Log {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Debug
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
