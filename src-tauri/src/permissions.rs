#[cfg(target_os = "macos")]
use std::sync::mpsc;
#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(target_os = "macos")]
use block2::RcBlock;
#[cfg(target_os = "macos")]
use objc2::runtime::Bool;
#[cfg(target_os = "macos")]
use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

#[cfg(target_os = "macos")]
fn microphone_media_type() -> Option<&'static objc2_av_foundation::AVMediaType> {
    unsafe { AVMediaTypeAudio }
}

#[cfg(target_os = "macos")]
pub fn check_microphone_permission() -> bool {
    let Some(media_type) = microphone_media_type() else {
        return false;
    };

    let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) };
    status == AVAuthorizationStatus::Authorized
}

#[cfg(target_os = "macos")]
pub fn request_microphone_permission() -> bool {
    let Some(media_type) = microphone_media_type() else {
        return false;
    };

    match unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) } {
        status if status == AVAuthorizationStatus::Authorized => true,
        status
            if status == AVAuthorizationStatus::Denied
                || status == AVAuthorizationStatus::Restricted =>
        {
            false
        }
        _ => {
            let (tx, rx) = mpsc::channel();
            let block = RcBlock::new(move |granted: Bool| {
                let _ = tx.send(granted.as_bool());
            });

            // Apple delivers this callback on an arbitrary queue, so use a channel
            // and wait briefly for the user's decision.
            // Safety: the block only captures an mpsc sender, which is safe to
            // use across threads when AVFoundation invokes the completion handler.
            unsafe {
                AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &block);
            }
            rx.recv_timeout(Duration::from_secs(60))
                .unwrap_or_else(|_| check_microphone_permission())
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_microphone_permission() -> bool {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    if let Some(device) = host.default_input_device() {
        device.default_input_config().is_ok()
    } else {
        false
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_microphone_permission() -> bool {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    if let Some(device) = host.default_input_device() {
        if let Ok(config) = device.default_input_config() {
            let stream = device.build_input_stream(
                &config.into(),
                |_data: &[f32], _: &cpal::InputCallbackInfo| {},
                |_err| {},
                None,
            );
            if let Ok(stream) = stream {
                let _ = stream.play();
                std::thread::sleep(std::time::Duration::from_millis(200));
                drop(stream);
                return true;
            }
        }
    }
    false
}
