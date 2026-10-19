#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Applies the launcher-wide child-process policy.
///
/// Windows creates a visible console for console-subsystem executables such as
/// `java.exe`, PowerShell and `taskkill` unless the parent opts out explicitly.
/// Keep every child process hidden so background checks never interrupt the UI.
pub(crate) fn hide_std_console(command: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    #[cfg(not(windows))]
    let _ = command;
}

pub(crate) fn hide_tokio_console(command: &mut tokio::process::Command) {
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    #[cfg(not(windows))]
    let _ = command;
}
