mod git;
mod github;
mod system;
mod terminal;
mod update;

/// The repository passed on the command line, as in `fox-git .`.
#[tauri::command]
fn launch_path() -> Option<String> {
    std::env::args().nth(1)
}

/// Says where to get WebView2, since without it the app would close without a word.
#[cfg(windows)]
fn offer_webview2() {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    use windows_sys::Win32::UI::WindowsAndMessaging::{IDYES, MB_ICONERROR, MB_YESNO, MessageBoxW};

    const PAGE: &str = "https://developer.microsoft.com/microsoft-edge/webview2/";
    let russian = unsafe { GetUserDefaultUILanguage() } & 0xff == 0x19;
    let message = if russian {
        format!(
            "Не удалось открыть окно Fox Git. Обычно это значит, что компонент Microsoft Edge WebView2 не установлен или повреждён.

Установите WebView2 Runtime со страницы Microsoft и запустите Fox Git снова:
{PAGE}

Открыть страницу?"
        )
    } else {
        format!(
            "Fox Git could not open its window. This usually means that Microsoft Edge WebView2 is missing or damaged.

Install WebView2 Runtime from the Microsoft page and start Fox Git again:
{PAGE}

Open the page?"
        )
    };
    let wide = |text: &str| text.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let answer = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide(&message).as_ptr(),
            wide("Fox Git").as_ptr(),
            MB_YESNO | MB_ICONERROR,
        )
    };
    if answer == IDYES {
        let _ = tauri_plugin_opener::open_url(PAGE, None::<&str>);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    if tauri::webview_version().is_err() {
        offer_webview2();
        return;
    }

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(update::Pending::default())
        .manage(terminal::Terminals::default())
        .invoke_handler(tauri::generate_handler![
            launch_path,
            git::repo_root,
            git::overview,
            git::log,
            git::status,
            git::file_diff,
            git::conflict_diff,
            git::commit_show,
            git::file_log,
            git::commit_file_diff,
            git::lfs_info,
            git::git_action,
            git::clone_repo,
            git::repos_in,
            git::trust_repo,
            git::ignore_paths,
            github::github_built_in,
            github::github_login,
            github::github_login_cancel,
            github::github_user,
            github::github_logout,
            github::github_repos,
            github::github_add_ssh_key,
            system::system_accent,
            system::ssh_keys,
            system::ssh_generate,
            system::ssh_test,
            terminal::terminal_open,
            terminal::terminal_write,
            terminal::terminal_resize,
            terminal::terminal_close,
            update::update_check,
            update::update_install,
            update::update_restart,
        ])
        .build(tauri::generate_context!());

    #[cfg(windows)]
    if result.is_err() {
        offer_webview2();
        return;
    }
    update::clean_up();
    result.expect("error while running Fox Git").run(|app, event| {
        if let tauri::RunEvent::Exit = event {
            update::on_exit(tauri::Manager::state(app));
        }
    });
}
