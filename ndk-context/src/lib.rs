//! Provides a stable api to rust crates for interfacing with the Android platform. It is
//! initialized by the runtime, usually [__ndk-glue__](https://crates.io/crates/ndk-glue),
//! but could also be initialized by Java or Kotlin code when embedding in an existing Android
//! project.
//!
//! ```no_run
//! # use jni::objects::JObject;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let ctx = ndk_context::android_context();
//! let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
//! let context = unsafe { JObject::from_raw(ctx.context().cast()) };
//! let env = vm.attach_current_thread()?;
//! let class_ctx = env.find_class("android/content/Context")?;
//! let audio_service = env.get_static_field(class_ctx, "AUDIO_SERVICE", "Ljava/lang/String;")?;
//! let audio_manager = env
//!     .call_method(
//!         context,
//!         "getSystemService",
//!         "(Ljava/lang/String;)Ljava/lang/Object;",
//!         &[audio_service],
//!     )?
//!     .l()?;
//! # Ok(())
//! # }
//! ```
use std::ffi::c_void;
use std::sync::OnceLock;

// uplink patch: set once, first caller wins, never released. See docs/ref/ndk-vendoring.md.
static ANDROID_CONTEXT: OnceLock<Shared> = OnceLock::new();

/// The pointers are a process-wide `JavaVM` and a JNI global reference, both usable from any
/// thread, which is what makes sharing them sound.
struct Shared(AndroidContext);

// SAFETY: see `Shared`.
unsafe impl Send for Shared {}
// SAFETY: see `Shared`.
unsafe impl Sync for Shared {}

/// [`AndroidContext`] provides the pointers required to interface with the jni on Android
/// platforms.
#[derive(Clone, Copy, Debug)]
pub struct AndroidContext {
    java_vm: *mut c_void,
    context_jobject: *mut c_void,
}

impl AndroidContext {
    /// A handle to the `JavaVM` object.
    ///
    /// Usage with [__jni__](https://crates.io/crates/jni) crate:
    /// ```no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let ctx = ndk_context::android_context();
    /// let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
    /// let env = vm.attach_current_thread();
    /// # Ok(())
    /// # }
    /// ```
    pub fn vm(self) -> *mut c_void {
        self.java_vm
    }

    /// A handle to an [android.content.Context](https://developer.android.com/reference/android/content/Context).
    /// In most cases this will be a ptr to an `Activity`, but this isn't guaranteed.
    ///
    /// Usage with [__jni__](https://crates.io/crates/jni) crate:
    /// ```no_run
    /// # use jni::objects::JObject;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let ctx = ndk_context::android_context();
    /// let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
    /// let context = unsafe { JObject::from_raw(ctx.context().cast()) };
    /// let env = vm.attach_current_thread()?;
    /// let class_ctx = env.find_class("android/content/Context")?;
    /// let audio_service = env.get_static_field(class_ctx, "AUDIO_SERVICE", "Ljava/lang/String;")?;
    /// let audio_manager = env
    ///     .call_method(
    ///         context,
    ///         "getSystemService",
    ///         "(Ljava/lang/String;)Ljava/lang/Object;",
    ///         &[audio_service],
    ///     )?
    ///     .l()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn context(self) -> *mut c_void {
        self.context_jobject
    }
}

/// Main entry point to this crate. Returns an [`AndroidContext`].
pub fn android_context() -> AndroidContext {
    ANDROID_CONTEXT
        .get()
        .map(|shared| shared.0)
        .expect("android context was not initialized")
}

/// Initializes the [`AndroidContext`].
///
/// uplink patch: the first call wins and later ones are ignored, instead of panicking. The app
/// sets the `Application` from `Application.onCreate`, before anything reads it; android-activity
/// sets the same `Application` again when the first activity starts, and that call is a no-op.
///
/// # Safety
///
/// The pointers must be valid until the process exits.
pub unsafe fn initialize_android_context(java_vm: *mut c_void, context_jobject: *mut c_void) {
    // A second caller's context is its own global ref to the same Application, so only the VM,
    // of which a process has exactly one, can be compared.
    let first = ANDROID_CONTEXT.get_or_init(|| {
        Shared(AndroidContext {
            java_vm,
            context_jobject,
        })
    });
    debug_assert_eq!(first.0.java_vm, java_vm, "a second JavaVM in one process");
}

/// uplink patch: a no-op. The context lives as long as the process, because threads that read it
/// can outlive any activity, and nothing ever tells an Android app its process is ending.
///
/// # Safety
///
/// Always safe; kept `unsafe` so upstream callers compile unchanged.
pub unsafe fn release_android_context() {}

#[cfg(test)]
mod tests {
    use super::*;

    // One test, because the context is process-wide and tests share the process.
    #[test]
    fn first_caller_wins() {
        let vm = std::ptr::without_provenance_mut(0x10);
        let (ours, theirs) = (
            std::ptr::without_provenance_mut(0x20),
            std::ptr::without_provenance_mut(0x30),
        );
        unsafe {
            initialize_android_context(vm, ours);
            initialize_android_context(vm, theirs);
            release_android_context();
        }
        let ctx = android_context();
        assert_eq!((ctx.vm(), ctx.context()), (vm, ours));
    }
}
