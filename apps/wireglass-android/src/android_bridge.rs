//! Android-only JNI bridge for Share/Copy/OpenURL/files-dir.

#![cfg(target_os = "android")]

use jni::bind_java_type;
// jni types referenced by bind_java_type! macro expansion
use jni::JValue;
use jni::JavaVM;
use std::sync::Mutex;

static APP: Mutex<Option<slint::android::AndroidApp>> = Mutex::new(None);

pub fn store_app(app: slint::android::AndroidApp) {
    *APP.lock().unwrap() = Some(app);
}

pub fn app() -> Option<slint::android::AndroidApp> {
    APP.lock().unwrap().clone()
}

bind_java_type! {
    JFile => java.io.File,
    methods {
        fn get_absolute_path {
            sig = () -> JString,
            name = "getAbsolutePath",
        },
    },
}

bind_java_type! {
    AndroidUri => android.net.Uri,
    methods {
        static fn parse(uri: JString) -> AndroidUri,
    },
}

bind_java_type! {
    AndroidContext => android.content.Context,
    type_map = {
        JFile => java.io.File,
        AndroidIntent => android.content.Intent,
    },
    methods {
        fn get_files_dir {
            sig = () -> JFile,
            name = "getFilesDir",
        },
        fn get_system_service {
            sig = (name: JString) -> JObject,
            name = "getSystemService",
        },
        fn start_activity {
            sig = (intent: AndroidIntent) -> (),
            name = "startActivity",
        },
    },
}

bind_java_type! {
    AndroidActivity => android.app.Activity,
    type_map = {
        AndroidContext => android.content.Context,
        JFile => java.io.File,
        AndroidIntent => android.content.Intent,
    },
    methods {
        fn get_files_dir {
            sig = () -> JFile,
            name = "getFilesDir",
        },
        fn get_system_service {
            sig = (name: JString) -> JObject,
            name = "getSystemService",
        },
        fn start_activity {
            sig = (intent: AndroidIntent) -> (),
            name = "startActivity",
        },
    },
}

bind_java_type! {
    AndroidIntent => android.content.Intent,
    type_map = {
        AndroidUri => android.net.Uri,
    },
    constructors {
        fn new {
            sig = (action: JString),
        },
    },
    methods {
        fn put_extra {
            sig = (name: JString, value: JString) -> AndroidIntent,
            name = "putExtra",
        },
        fn set_type {
            sig = (mime: JString) -> AndroidIntent,
            name = "setType",
        },
        fn set_data {
            sig = (uri: AndroidUri) -> AndroidIntent,
            name = "setData",
        },
    },
    methods {
        static fn create_chooser {
            sig = (target: AndroidIntent, title: JString) -> AndroidIntent,
            name = "createChooser",
        },
    },
}

fn with_env<R>(
    f: impl FnOnce(&mut jni::Env, slint::android::AndroidApp) -> Result<R, jni::errors::Error>,
) -> Option<R> {
    let app = app()?;
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };
    vm.attach_current_thread(|env| f(env, app.clone())).ok()
}

pub fn files_dir() -> Option<String> {
    with_env(|env, app| {
        let activity_ptr = app.activity_as_ptr() as jni::sys::jobject;
        let activity = unsafe { env.as_cast_raw::<AndroidActivity>(&activity_ptr)? };
        let ctx = activity.as_ref();
        let file = ctx.get_files_dir(env)?;
        let path = file.get_absolute_path(env)?;
        Ok(path.to_string())
    })
}

pub fn share_text(text: &str) -> Result<(), String> {
    with_env(|env, app| {
        let activity_ptr = app.activity_as_ptr() as jni::sys::jobject;
        let activity = unsafe { env.as_cast_raw::<AndroidActivity>(&activity_ptr)? };
        let ctx = activity.as_ref();
        let action = env.new_string("android.intent.action.SEND")?;
        let intent = AndroidIntent::new(env, &action)?;
        let extra_key = env.new_string("android.intent.extra.TEXT")?;
        let extra_val = env.new_string(text)?;
        let _ = intent.put_extra(env, &extra_key, &extra_val);
        let mime = env.new_string("text/plain")?;
        let _ = intent.set_type(env, &mime);
        let title = env.new_string("Share Wireglass report")?;
        let chooser = AndroidIntent::create_chooser(env, &intent, &title)?;
        ctx.start_activity(env, &chooser)?;
        Ok(())
    })
    .ok_or_else(|| "android app handle unavailable".to_string())
}

pub fn open_url(url: &str) {
    let _ = with_env(|env, app| {
        let activity_ptr = app.activity_as_ptr() as jni::sys::jobject;
        let activity = unsafe { env.as_cast_raw::<AndroidActivity>(&activity_ptr)? };
        let ctx = activity.as_ref();
        let action = env.new_string("android.intent.action.VIEW")?;
        let intent = AndroidIntent::new(env, &action)?;
        let url_str = env.new_string(url)?;
        let uri = AndroidUri::parse(env, &url_str)?;
        let _ = intent.set_data(env, &uri);
        ctx.start_activity(env, &intent)?;
        Ok(())
    });
}

pub fn set_clipboard(text: &str) -> Result<(), String> {
    let app = app().ok_or_else(|| "android app handle unavailable".to_string())?;
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };
    let outcome = vm.attach_current_thread(|env| -> Result<(), jni::errors::Error> {
        let activity_ptr = app.activity_as_ptr() as jni::sys::jobject;
        let ctx = unsafe { jni::objects::JObject::from_raw(env, activity_ptr) };
        // ClipboardManager via system service (crate-style raw call)
        let svc_name = env.new_string("clipboard")?;
        let cm = env
            .call_method(
                &ctx,
                jni::jni_str!("getSystemService"),
                jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                &[JValue::from(&svc_name)],
            )?
            .l()?;
        // ClipData.newPlainText(label, text)
        let label = env.new_string("Wireglass")?;
        let txt = env.new_string(text)?;
        let clip_class = env.find_class(jni::jni_str!("android/content/ClipData"))?;
        let clip = env
            .call_static_method(
                &clip_class,
                jni::jni_str!("newPlainText"),
                jni::jni_sig!(
                    "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"
                ),
                &[JValue::from(&label), JValue::from(&txt)],
            )?
            .l()?;
        // setPrimaryClip(clip)
        env.call_method(
            &cm,
            jni::jni_str!("setPrimaryClip"),
            jni::jni_sig!("(Landroid/content/ClipData;)V"),
            &[JValue::from(&clip)],
        )?;
        Ok(())
    });
    match outcome {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("clipboard jni error: {e}")),
    }
}
