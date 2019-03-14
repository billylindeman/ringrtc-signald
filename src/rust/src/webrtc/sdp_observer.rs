//
// Copyright (C) 2019 Signal Messenger, LLC.
// All rights reserved.
//
// SPDX-License-Identifier: GPL-3.0-only
//

//! WebRTC Create Session Description Interface.

use std::ffi::{
    CStr,
    CString,
};
use std::fmt;
use std::os::raw::c_char;
use std::ptr;
use std::sync::{
    Arc,
    Mutex,
    Condvar,
};

use crate::common::Result;
use crate::core::util::{
    RustObject,
    CppObject,
    FutureResult,
    get_object_from_cpp,
};
use crate::error::RingRtcError;

/// Incomplete type for SessionDescriptionInterface, used by
/// CreateSessionDescriptionObserver callbacks.
#[repr(C)]
pub struct RffiSessionDescriptionInterface { _private: [u8; 0] }

/// Rust wrapper around WebRTC C++ SessionDescriptionInterface.
pub struct SessionDescriptionInterface {
    /// Pointer to C++ SessionDescriptionInterface object.
    sd_interface: *const RffiSessionDescriptionInterface,
}

impl fmt::Display for SessionDescriptionInterface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "sd_interface: {:p}", self.sd_interface)
    }
}

impl fmt::Debug for SessionDescriptionInterface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl SessionDescriptionInterface {
    /// Create a new SessionDescriptionInterface from a C++ SessionDescriptionInterface object.
    pub fn new(sd_interface: *const RffiSessionDescriptionInterface) -> Self {
        Self {
            sd_interface,
        }
    }

    /// Return the internal WebRTC C++ SessionDescriptionInterface pointer.
    pub fn get_rffi_interface(&self) -> *const RffiSessionDescriptionInterface {
        self.sd_interface
    }

    /// Return a string representation of this SessionDescriptionInterface.
    pub fn get_description(&self) -> Result<String> {

        let string_ptr = unsafe { Rust_getOfferDescription(self.sd_interface) };
        if string_ptr.is_null() {
            Err(RingRtcError::GetOfferDescription.into())
        } else {
            let description = unsafe { CStr::from_ptr(string_ptr).to_string_lossy().into_owned() };
            unsafe { libc::free(string_ptr as *mut libc::c_void) };
            Ok(description)
        }
    }

    /// Create a SDP answer from the session description string.
    pub fn create_sdp_answer(session_desc: String) -> Result<Self> {
        let sdp = CString::new(session_desc)?;
        let answer = unsafe {
            Rust_createSessionDescriptionAnswer(sdp.as_ptr())
        };
        if answer.is_null() {
            return Err(RingRtcError::ConvertSdpAnswer.into());
        }
        Ok(SessionDescriptionInterface::new(answer))
    }

    /// Create a SDP offer from the session description string.
    pub fn create_sdp_offer(session_desc: String) -> Result<Self> {
        let sdp = CString::new(session_desc)?;
        let offer = unsafe {
            Rust_createSessionDescriptionOffer(sdp.as_ptr())
        };
        if offer.is_null() {
            return Err(RingRtcError::ConvertSdpOffer.into());
        }
        Ok(SessionDescriptionInterface::new(offer))
    }

}

/// Incomplete type for C++ webrtc::rffi::CreateSessionDescriptionObserverRffi
#[repr(C)]
pub struct RffiCreateSessionDescriptionObserver { _private: [u8; 0] }

/// Observer object for creating a session description.
#[derive(Debug)]
pub struct CreateSessionDescriptionObserver {
    /// condition varialbe used to signal the completion of the create
    /// session description operation.
    condition: FutureResult<Result<*const RffiSessionDescriptionInterface>>,
    /// Pointer to C++ webrtc::rffi::CreateSessionDescriptionObserverRffi object
    rffi_csd_observer: *const RffiCreateSessionDescriptionObserver,
}

impl CreateSessionDescriptionObserver {
    /// Create a new CreateSessionDescriptionObserver.
    fn new() -> Self {
        Self {
            condition: Arc::new((Mutex::new((false, Ok(ptr::null()))), Condvar::new())),
            rffi_csd_observer: ptr::null(),
        }
    }

    /// Called back when the create session description operation is a
    /// success.
    ///
    /// This call signals the condition variable.
    fn on_create_success(&self, desc: *const RffiSessionDescriptionInterface) {
        info!("on_create_success()");
        let &(ref mtx, ref cvar) = &*self.condition;
        if let Ok(mut guard) = mtx.lock() {
            guard.1 = Ok(desc);
            guard.0 = true;
            // We notify the condvar that the value has changed.
            cvar.notify_one();
        }
    }

    /// Called back when the create session description operation is a
    /// failure.
    ///
    /// This call signals the condition variable.
    fn on_create_failure(&self, err_message: String, err_type: i32) {
        warn!("on_create_failure(). error msg: {}, type: {}", err_message, err_type);
        let &(ref mtx, ref cvar) = &*self.condition;
        if let Ok(mut guard) = mtx.lock() {
            guard.1 = Err(RingRtcError::CreateSessionDescriptionObserver(err_message, err_type).into());
            guard.0 = true;
            // We notify the condvar that the value has changed.
            cvar.notify_one();
        }
    }

    /// Retrieve the result of the create session description operation.
    ///
    /// This call blocks on the condition variable.
    pub fn get_result(&self) -> Result<SessionDescriptionInterface> {
        let &(ref mtx, ref cvar) = &*self.condition;
        if let Ok(mut guard) = mtx.lock() {
            while !guard.0 {
                guard = cvar.wait(guard).map_err(|_| { RingRtcError::MutexPoisoned("CreateSessionDescription condvar mutex".to_string()) })?;
            }
            // TODO: implement guard.1.clone() here ....
            match &guard.1 {
                Ok(v) => Ok(SessionDescriptionInterface::new(*v)),
                Err(e) => Err(RingRtcError::CreateSessionDescriptionObserverResult(format!("{}", e)).into()),
            }
        } else {
            Err(RingRtcError::MutexPoisoned("CreateSessionDescription condvar mutex".to_string()).into())
        }
    }

    pub fn set_rffi_observer(&mut self, observer: *const RffiCreateSessionDescriptionObserver) {
        self.rffi_csd_observer = observer
    }

    pub fn get_rffi_observer(&self) -> *const RffiCreateSessionDescriptionObserver {
        self.rffi_csd_observer
    }

}

/// CreateSessionDescription observer OnSuccess() callback.
#[no_mangle]
#[allow(non_snake_case)]
extern fn csd_observer_OnSuccess(csd_observer: RustObject,
                                 desc: *const RffiSessionDescriptionInterface) {
    info!("csd_observer_OnSuccess()");
    if let Ok(v) = get_object_from_cpp(csd_observer) {
        let csd_observer: & CreateSessionDescriptionObserver = v;
        csd_observer.on_create_success(desc);
    }
}

/// CreateSessionDescription observer OnFailure() callback.
#[no_mangle]
#[allow(non_snake_case)]
extern fn csd_observer_OnFailure(csd_observer: RustObject,
                                 err_message: *const c_char, err_type: i32) {
    let err_string: String = unsafe { CStr::from_ptr(err_message).to_string_lossy().into_owned() };
    error!("csd_observer_OnFailure(): {}, type: {}", err_string, err_type);
    if let Ok(v) = get_object_from_cpp(csd_observer) {
        let csd_observer: & CreateSessionDescriptionObserver = v;
        csd_observer.on_create_failure(err_string, err_type);
    }
}

/// CreateSessionDescription observer callback function pointers.
#[repr(C)]
#[allow(non_snake_case)]
struct CreateSessionDescriptionObserverCallbacks {
    onSuccess: extern fn(csd_observer: RustObject, desc: *const RffiSessionDescriptionInterface),
    onFailure: extern fn (csd_observer: RustObject, error_message: *const c_char, error_type: i32),
}

const CSD_OBSERVER_CBS: CreateSessionDescriptionObserverCallbacks = CreateSessionDescriptionObserverCallbacks {
    onSuccess: csd_observer_OnSuccess,
    onFailure: csd_observer_OnFailure,
};
const CSD_OBSERVER_CBS_PTR: *const CreateSessionDescriptionObserverCallbacks = &CSD_OBSERVER_CBS;

/// Create a new Rust CreateSessionDescriptionObserver object.
///
/// Creates a new WebRTC C++ CreateSessionDescriptionObserver object,
/// registering the observer callbacks to this module, and wraps the
/// result in a Rust CreateSessionDescriptionObserver object.
pub fn create_csd_observer() -> Box<CreateSessionDescriptionObserver> {
    let csd_observer = Box::new(CreateSessionDescriptionObserver::new());
    let csd_observer_ptr = Box::into_raw(csd_observer);
    let rffi_csd_observer = unsafe {
        Rust_createCreateSessionDescriptionObserver(csd_observer_ptr as RustObject,
                                                    CSD_OBSERVER_CBS_PTR)
    };
    let mut csd_observer = unsafe { Box::from_raw(csd_observer_ptr) };

    csd_observer.set_rffi_observer(rffi_csd_observer);
    csd_observer
}

/// Incomplete type for C++ CreateSessionDescriptionObserverRffi
#[repr(C)]
pub struct RffiSetSessionDescriptionObserver { _private: [u8; 0] }

/// Observer object for setting a session description.
#[derive(Debug)]
pub struct SetSessionDescriptionObserver {
    /// condition varialbe used to signal the completion of the set
    /// session description operation.
    condition: FutureResult<Result<()>>,
    /// Pointer to C++ CreateSessionDescriptionObserver object
    rffi_ssd_observer: *const RffiSetSessionDescriptionObserver,
}

impl SetSessionDescriptionObserver {
    /// Create a new SetSessionDescriptionObserver.
    fn new() -> Self {
        Self {
            condition: Arc::new((Mutex::new((false, Ok(()))), Condvar::new())),
            rffi_ssd_observer: ptr::null(),
        }
    }

    /// Called back when the set session description operation is a
    /// success.
    ///
    /// This call signals the condition variable.
    fn on_set_success(&self) {
        info!("on_set_success()");
        let &(ref mtx, ref cvar) = &*self.condition;
        if let Ok(mut guard) = mtx.lock() {
            guard.1 = Ok(());
            guard.0 = true;
            // We notify the condvar that the value has changed.
            cvar.notify_one();
        }
    }

    /// Called back when the set session description operation is a
    /// failure.
    ///
    /// This call signals the condition variable.
    fn on_set_failure(&self, err_message: String, err_type: i32) {
        warn!("on_set_failure(). error msg: {}, type: {}", err_message, err_type);
        let &(ref mtx, ref cvar) = &*self.condition;
        if let Ok(mut guard) = mtx.lock() {
            guard.1 = Err(RingRtcError::SetSessionDescriptionObserver(err_message, err_type).into());
            guard.0 = true;
            // We notify the condvar that the value has changed.
            cvar.notify_one();
        }
    }

    /// Retrieve the result of the create session description operation.
    ///
    /// This call blocks on the condition variable.
    pub fn get_result(&self) -> Result<()> {
        let &(ref mtx, ref cvar) = &*self.condition;
        if let Ok(mut guard) = mtx.lock() {
            while !guard.0 {
                guard = cvar.wait(guard).map_err(|_| { RingRtcError::MutexPoisoned("SetSessionDescription condvar mutex".to_string()) })?;
            }
            // TODO: implement guard.1.clone() here ....
            match &guard.1 {
                Ok(_) => Ok(()),
                Err(e) => Err(RingRtcError::SetSessionDescriptionObserverResult(format!("{}", e)).into()),
            }
        } else {
            Err(RingRtcError::MutexPoisoned("SetSessionDescription condvar mutex".to_string()).into())
        }
    }

    pub fn set_rffi_observer(&mut self, observer: *const RffiSetSessionDescriptionObserver) {
        self.rffi_ssd_observer = observer
    }

    pub fn get_rffi_observer(&self) -> *const RffiSetSessionDescriptionObserver {
        self.rffi_ssd_observer
    }

}

/// SetSessionDescription observer OnSuccess() callback.
#[no_mangle]
#[allow(non_snake_case)]
extern fn ssd_observer_OnSuccess(ssd_observer: RustObject) {
    info!("ssd_observer_OnSuccess()");
    if let Ok(v) = get_object_from_cpp(ssd_observer) {
        let ssd_observer: & SetSessionDescriptionObserver = v;
        ssd_observer.on_set_success();
    }
}

/// SetSessionDescription observer OnFailure() callback.
#[no_mangle]
#[allow(non_snake_case)]
extern fn ssd_observer_OnFailure(ssd_observer: RustObject,
                                 err_message: *const c_char, err_type: i32) {
    let err_string: String = unsafe { CStr::from_ptr(err_message).to_string_lossy().into_owned() };
    error!("ssd_observer_OnFailure(): {}, type: {}", err_string, err_type);
    if let Ok(v) = get_object_from_cpp(ssd_observer) {
        let ssd_observer: & SetSessionDescriptionObserver = v;
        ssd_observer.on_set_failure(err_string, err_type);
    }
}

/// SetSessionDescription observer callback function pointers.
#[repr(C)]
#[allow(non_snake_case)]
struct SetSessionDescriptionObserverCallbacks {
    onSuccess: extern fn(ssd_observer: RustObject),
    onFailure: extern fn (ssd_observer: RustObject, error_message: *const c_char, error_type: i32),
}

const SSD_OBSERVER_CBS: SetSessionDescriptionObserverCallbacks = SetSessionDescriptionObserverCallbacks {
    onSuccess: ssd_observer_OnSuccess,
    onFailure: ssd_observer_OnFailure,
};
const SSD_OBSERVER_CBS_PTR: *const SetSessionDescriptionObserverCallbacks = &SSD_OBSERVER_CBS;

/// Create a new Rust SetSessionDescriptionObserver object.
///
/// Creates a new WebRTC C++ SetSessionDescriptionObserver object,
/// registering the observer callbacks to this module, and wraps the
/// result in a Rust SetSessionDescriptionObserver object.
pub fn create_ssd_observer() -> Box<SetSessionDescriptionObserver> {

    let ssd_observer = Box::new(SetSessionDescriptionObserver::new());
    let ssd_observer_ptr = Box::into_raw(ssd_observer);
    let rffi_ssd_observer = unsafe {
        Rust_createSetSessionDescriptionObserver(ssd_observer_ptr as CppObject,
                                                 SSD_OBSERVER_CBS_PTR)
    };
    let mut ssd_observer = unsafe { Box::from_raw(ssd_observer_ptr) };

    ssd_observer.set_rffi_observer(rffi_ssd_observer as *const RffiSetSessionDescriptionObserver);
    ssd_observer
}

extern {
    fn Rust_createSetSessionDescriptionObserver(ssd_observer:    RustObject,
                                                ssd_observer_cb: *const SetSessionDescriptionObserverCallbacks)
                                                -> *const RffiSetSessionDescriptionObserver;

    fn Rust_createCreateSessionDescriptionObserver(csd_observer:     RustObject,
                                                   csd_observer_cb: *const CreateSessionDescriptionObserverCallbacks)
                                                   -> *const RffiCreateSessionDescriptionObserver;

    fn Rust_getOfferDescription(offer: *const RffiSessionDescriptionInterface)
                                -> *const c_char;

    fn Rust_createSessionDescriptionAnswer(description: *const c_char)
                                           ->  *const RffiSessionDescriptionInterface;

    fn Rust_createSessionDescriptionOffer(description: *const c_char)
                                          ->  *const RffiSessionDescriptionInterface;
}