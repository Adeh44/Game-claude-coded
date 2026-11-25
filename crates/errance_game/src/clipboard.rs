//! Copier du texte dans le presse-papiers du téléphone (pour coller le
//! journal du moteur dans la conversation avec Claude).
//!
//! Android : le presse-papiers est un service Java, atteint par JNI :
//! `activity.getSystemService(CLIPBOARD_SERVICE).setPrimaryClip(ClipData.newPlainText(...))`.
//! PC : on écrit le texte dans `journal_copie.txt` (dossier de sauvegarde).

use jni::JavaVM;
use jni::objects::{JObject, JValue};
use std::ffi::c_void;

/// Appel JNI. Compilé partout (vérifiable sur PC), appelé seulement sur Android.
#[allow(dead_code)]
fn copy_jni(vm_ptr: *mut c_void, activity_ptr: *mut c_void, text: &str) -> Result<(), String> {
    let e = |x: jni::errors::Error| x.to_string();
    // SAFETY : pointeurs fournis par android-activity, valides tant que l'appli vit.
    let vm = unsafe { JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) }.map_err(e)?;
    let mut env = vm.attach_current_thread().map_err(e)?;
    let activity = unsafe { JObject::from_raw(activity_ptr as jni::sys::jobject) };
    let result = (|| -> Result<(), jni::errors::Error> {
        let service = env.get_static_field("android/content/Context", "CLIPBOARD_SERVICE", "Ljava/lang/String;")?.l()?;
        let cm = env.call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[JValue::Object(&service)])?.l()?;
        let label = env.new_string("Errance")?;
        let body = env.new_string(text)?;
        let clip = env
            .call_static_method(
                "android/content/ClipData",
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JValue::Object(&label), JValue::Object(&body)],
            )?
            .l()?;
        env.call_method(&cm, "setPrimaryClip", "(Landroid/content/ClipData;)V", &[JValue::Object(&clip)])?;
        Ok(())
    })();
    if result.is_err() {
        // Une exception Java laissée en suspens ferait planter l'appel suivant.
        let _ = env.exception_clear();
    }
    result.map_err(e)
}

/// Copie `text`. Renvoie un message pour le joueur.
pub fn copy(text: &str) -> String {
    #[cfg(target_os = "android")]
    {
        let Some(app) = bevy::android::ANDROID_APP.get() else { return "Copie impossible (application introuvable).".into() };
        match copy_jni(app.vm_as_ptr(), app.activity_as_ptr(), text) {
            Ok(()) => "Journal copié ! Colle-le dans la conversation avec Claude.".into(),
            Err(err) => format!("Copie impossible : {err}"),
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        crate::save::write_named("journal_copie.txt", text);
        "Journal écrit dans journal_copie.txt (dossier de sauvegarde).".into()
    }
}
