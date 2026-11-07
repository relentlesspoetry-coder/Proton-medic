//! A native close confirmation remains available even if the web view is busy.
use crate::{app_update, windows_games::update_stop::{self, Session}};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

static REQUESTED: AtomicBool = AtomicBool::new(false);
struct Request<'a>(&'a AtomicBool);
impl<'a> Request<'a> {
    fn acquire(flag: &'a AtomicBool) -> Option<Self> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).ok().map(|_| Self(flag))
    }
}
impl Drop for Request<'_> { fn drop(&mut self) { self.0.store(false, Ordering::Release); } }

fn confirmation(sessions: &[Session]) -> String {
    // Keep buttons reachable on small displays even with many running profiles.
    let mut names = sessions.iter().take(6).map(|s| {
        let name: String = s.name.chars().take(70).collect();
        format!("• {}{} ({})", name, if s.name.chars().count()>70 { "…" } else { "" }, s.kind)
    }).collect::<Vec<_>>().join("\n");
    if sessions.len()>6 { names.push_str(&format!("\n• And {} more active Proton Medic sessions", sessions.len()-6)); }
    format!("Force quit Proton Medic and stop all {} active games and installers?\n\n{names}\n\nUnsaved game progress will be lost. An interrupted installation may need to be repaired or run again. Processes that do not close will be forcibly stopped.\n\nOnly these Proton Medic sessions and their child processes will be stopped. Steam and unrelated apps will stay open.", sessions.len())
}
fn show_error(window: &tauri::Window, reason: &str) {
    window.dialog().message(format!("Proton Medic stayed open.\n\n{reason}\n\nFinish or cancel that operation, then try closing again."))
        .title("Could not close Proton Medic").kind(MessageDialogKind::Warning).show(|_| {});
}

pub(crate) fn request(window: tauri::Window, original_reason: String) {
    let Some(request) = Request::acquire(&REQUESTED) else { return; };
    tauri::async_runtime::spawn_blocking(move || {
        let prepared = (|| {
            let reservation = app_update::reserve_force_close()?;
            let sessions = update_stop::force_active()?;
            if sessions.iter().any(|session| !session.can_stop) {
                return Err("An active game was started by an older Proton Medic version. Close it manually first; its processes cannot be safely force-stopped by this version.".to_owned());
            }
            Ok((reservation, sessions))
        })();
        let (reservation, sessions) = match prepared {
            Ok(prepared) => prepared,
            Err(reason) => { show_error(&window, &reason); return; }
        };
        if sessions.is_empty() {
            // A session may have finished between the original click and review.
            match reservation.finish() {
                Ok(()) => window.app_handle().exit(0),
                Err(_) => show_error(&window, &original_reason),
            }
            return;
        }
        let message = confirmation(&sessions);
        window.dialog().message(message).title("Force quit Proton Medic?")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom("Force Quit".into(), "Keep Open".into()))
            .show(move |confirmed| {
                if !confirmed { drop(reservation); drop(request); return; }
                tauri::async_runtime::spawn_blocking(move || {
                    // Errors (including changed sessions) keep the app open. The
                    // reservation drops on every failure so the user can retry.
                    let result = update_stop::force_stop(sessions, true).and_then(|_| reservation.finish());
                    if let Err(reason) = result { show_error(&window, &reason); }
                    else { window.app_handle().exit(0); }
                    drop(request);
                });
            });
    });
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn warning_names_reviewed_sessions_and_explains_loss_and_scope() {
        let sessions = vec![Session { id: "1-2".into(), run_id: "1-3".into(), name: "Fixture game".into(), kind: "game".into(), can_stop: true }];
        let text = confirmation(&sessions);
        for required in ["Fixture game (game)", "Unsaved game progress", "interrupted installation", "forcibly stopped", "Steam and unrelated apps will stay open"] {
            assert!(text.contains(required), "{required}");
        }
    }
    #[test] fn large_session_list_keeps_confirmation_bounded() {
        let session=Session { id: "1-2".into(), run_id: "1-3".into(), name: "Long game title ".repeat(100), kind: "game".into(), can_stop: true };
        let text=confirmation(&vec![session;17]);
        assert!(text.contains("all 17 active"));
        assert!(text.contains("11 more active Proton Medic sessions"));
        assert!(text.chars().count()<1200);
    }
    #[test] fn repeated_close_requests_do_not_open_duplicate_confirmations() {
        let flag = AtomicBool::new(false);
        let first = Request::acquire(&flag).unwrap();
        assert!(Request::acquire(&flag).is_none());
        drop(first);
        assert!(Request::acquire(&flag).is_some());
    }
}
