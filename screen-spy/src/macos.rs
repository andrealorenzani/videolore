// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! What the recorder asks macOS (ADR-264): its version, the screen recording permission
//! and whether the microphone was denied. The one place of the program with `unsafe`.

use crate::app::Permission;
use crate::session::{MacVersion, MicrophoneAccess, MicrophonePermission, SCREEN_NOT_ALLOWED};
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2::{AnyThread, msg_send};
use objc2_core_audio::{
    AudioHardwareCreateAggregateDevice, AudioHardwareCreateProcessTap,
    AudioHardwareDestroyAggregateDevice, AudioHardwareDestroyProcessTap, AudioObjectID,
    CATapDescription, CATapMuteBehavior, kAudioAggregateDeviceIsPrivateKey,
    kAudioAggregateDeviceNameKey, kAudioAggregateDeviceTapAutoStartKey,
    kAudioAggregateDeviceTapListKey, kAudioAggregateDeviceUIDKey, kAudioSubTapDriftCompensationKey,
    kAudioSubTapUIDKey,
};
use objc2_core_foundation::CFDictionary;
use objc2_core_graphics::{CGPreflightScreenCaptureAccess, CGRequestScreenCaptureAccess};
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSProcessInfo, NSString};
use std::ffi::CStr;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;

/// The version of this macOS.
pub fn version() -> MacVersion {
    let v = NSProcessInfo::processInfo().operatingSystemVersion();
    MacVersion {
        major: u32::try_from(v.majorVersion).unwrap_or(0),
        minor: u32::try_from(v.minorVersion).unwrap_or(0),
    }
}

/// The screen recording permission: when it is missing macOS is asked to show its prompt
/// (once; it also lists the program in the settings).
pub struct ScreenPermission;

impl Permission for ScreenPermission {
    fn screen(&mut self) -> Result<(), String> {
        if CGPreflightScreenCaptureAccess() || CGRequestScreenCaptureAccess() {
            Ok(())
        } else {
            Err(SCREEN_NOT_ALLOWED.into())
        }
    }
}

// the class of the microphone permission lives here
#[link(name = "AVFoundation", kind = "framework")]
unsafe extern "C" {}

/// The microphone permission, through `AVCaptureDevice`.
pub struct Microphone;

/// The class and the value of `AVMediaTypeAudio`.
fn capture_device() -> Option<(&'static AnyClass, Retained<NSString>)> {
    AnyClass::get(c"AVCaptureDevice").map(|class| (class, NSString::from_str("soun")))
}

impl MicrophoneAccess for Microphone {
    fn status(&self) -> MicrophonePermission {
        let Some((class, audio)) = capture_device() else {
            return MicrophonePermission::Denied;
        };
        // SAFETY: `+[AVCaptureDevice authorizationStatusForMediaType:]` exists since
        // macOS 10.14 (this program needs 11), takes an NSString that is one of the media
        // types and returns an NSInteger (AVAuthorizationStatus).
        let status: isize = unsafe { msg_send![class, authorizationStatusForMediaType: &*audio] };
        MicrophonePermission::of_status(status)
    }

    fn ask(&self, answer: Sender<bool>) {
        let Some((class, audio)) = capture_device() else {
            return;
        };
        let block = RcBlock::new(move |granted: Bool| {
            // nobody listens any more after a cancelled start
            let _ = answer.send(granted.as_bool());
        });
        // SAFETY: `+[AVCaptureDevice requestAccessForMediaType:completionHandler:]`
        // exists since macOS 10.14, takes a media type and a block `void (^)(BOOL)` that
        // it copies and calls once, on a queue of its own; the block owns only the
        // channel's sender, which may be used from any thread.
        let () = unsafe {
            msg_send![class, requestAccessForMediaType: &*audio, completionHandler: &*block]
        };
    }
}

/// A Core Audio key as a Foundation string.
fn key(name: &CStr) -> Retained<NSString> {
    NSString::from_str(&name.to_string_lossy())
}

/// A tap of everything the computer plays, inside a private device that has it as its one
/// input (macOS 14.2 or later; the recorder asks for 14.6). Both are removed on drop.
pub struct SystemTap {
    tap: AudioObjectID,
    device: AudioObjectID,
    /// The identifier of the device.
    pub uid: String,
}

impl SystemTap {
    pub fn new() -> Result<SystemTap, String> {
        // each recording of a run has a device of its own name
        static MADE: AtomicU32 = AtomicU32::new(0);
        let number = MADE.fetch_add(1, Ordering::Relaxed);
        let name = NSString::from_str("videolore screen spy computer sound");
        // SAFETY: the initialiser takes an array of process objects to leave out (none),
        // and the setters take the values of their types.
        let description = unsafe {
            let d = CATapDescription::initStereoGlobalTapButExcludeProcesses(
                CATapDescription::alloc(),
                &NSArray::new(),
            );
            d.setName(&name);
            d.setPrivate(true);
            // the sound still goes to the speakers
            d.setMuteBehavior(CATapMuteBehavior::Unmuted);
            d
        };
        let mut made = SystemTap {
            tap: 0,
            device: 0,
            uid: format!("videolore-screen-spy-tap-{}-{number}", std::process::id()),
        };
        // SAFETY: the description is valid and the out pointer is a live variable.
        let status = unsafe { AudioHardwareCreateProcessTap(Some(&description), &mut made.tap) };
        check(status, "the computer's sound cannot be tapped")?;
        // SAFETY: a property of the live description.
        let tap_uid = unsafe { description.UUID() }.UUIDString();
        let yes = NSNumber::new_bool(true);
        let uid = NSString::from_str(&made.uid);
        let sub_tap = NSDictionary::<NSString, AnyObject>::from_slices(
            &[
                &*key(kAudioSubTapUIDKey),
                &*key(kAudioSubTapDriftCompensationKey),
            ],
            &[tap_uid.as_ref(), yes.as_ref()],
        );
        let taps = NSArray::from_retained_slice(&[sub_tap]);
        let properties = NSDictionary::<NSString, AnyObject>::from_slices(
            &[
                &*key(kAudioAggregateDeviceNameKey),
                &*key(kAudioAggregateDeviceUIDKey),
                &*key(kAudioAggregateDeviceTapListKey),
                &*key(kAudioAggregateDeviceTapAutoStartKey),
                &*key(kAudioAggregateDeviceIsPrivateKey),
            ],
            &[
                name.as_ref(),
                uid.as_ref(),
                taps.as_ref(),
                yes.as_ref(),
                yes.as_ref(),
            ],
        );
        let properties: *const NSDictionary<NSString, AnyObject> = &*properties;
        // SAFETY: NSDictionary and CFDictionary are the same object (toll-free bridging),
        // alive for the whole call; its keys are strings and its values the types Core
        // Audio documents for them; the out pointer is a live variable.
        let status = unsafe {
            AudioHardwareCreateAggregateDevice(
                &*properties.cast::<CFDictionary>(),
                NonNull::from(&mut made.device),
            )
        };
        check(status, "the device of the computer's sound cannot be made")?;
        Ok(made)
    }
}

fn check(status: i32, what: &str) -> Result<(), String> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{what} (Core Audio error {status})"))
    }
}

impl Drop for SystemTap {
    fn drop(&mut self) {
        // SAFETY: only identifiers this value created are destroyed, once; a failure
        // leaves nothing to do.
        unsafe {
            if self.device != 0 {
                let _ = AudioHardwareDestroyAggregateDevice(self.device);
            }
            if self.tap != 0 {
                let _ = AudioHardwareDestroyProcessTap(self.tap);
            }
        }
    }
}
