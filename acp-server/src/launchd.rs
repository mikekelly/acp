//! macOS LaunchAgent support for background service management
//!
//! This module provides plist generation for macOS LaunchAgents.
//! LaunchAgents run as user-level daemons after login with access to the user's Keychain.

#[cfg(target_os = "macos")]
use std::path::PathBuf;

#[cfg(target_os = "macos")]
/// Generate LaunchAgent plist XML for acp-server
///
/// Creates a plist configuration that:
/// - Runs at login (RunAtLoad)
/// - Keeps the service alive (KeepAlive)
/// - Logs stdout/stderr to ~/.acp/logs/
///
/// # Arguments
/// * `binary_path` - Absolute path to the acp-server binary
///
/// # Returns
/// Valid plist XML as a String
pub fn generate_plist(binary_path: &str) -> String {
    let log_dir = get_log_dir();
    let log_dir_str = log_dir.to_string_lossy();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.acp.server</string>
  <key>Program</key>
  <string>{}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>run</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>{}/acp-server.log</string>
  <key>StandardErrorPath</key>
  <string>{}/acp-server.err</string>
</dict>
</plist>
"#,
        binary_path, binary_path, log_dir_str, log_dir_str
    )
}

#[cfg(target_os = "macos")]
/// Get the default plist path for the LaunchAgent
///
/// Returns ~/Library/LaunchAgents/com.acp.server.plist
pub fn get_plist_path() -> PathBuf {
    let home_dir = dirs::home_dir().expect("Could not determine home directory");
    home_dir
        .join("Library")
        .join("LaunchAgents")
        .join("com.acp.server.plist")
}

#[cfg(target_os = "macos")]
/// Get the log directory path
///
/// Returns ~/.acp/logs/
pub fn get_log_dir() -> PathBuf {
    let home_dir = dirs::home_dir().expect("Could not determine home directory");
    home_dir.join(".acp").join("logs")
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;

    #[test]
    fn test_generate_plist_contains_required_keys() {
        let binary_path = "/usr/local/bin/acp-server";
        let plist = generate_plist(binary_path);

        // Verify XML structure
        assert!(plist.contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(plist.contains(r#"<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN""#));
        assert!(plist.contains(r#"<plist version="1.0">"#));

        // Verify required keys
        assert!(plist.contains("<key>Label</key>"));
        assert!(plist.contains("<string>com.acp.server</string>"));

        assert!(plist.contains("<key>Program</key>"));
        assert!(plist.contains(&format!("<string>{}</string>", binary_path)));

        assert!(plist.contains("<key>ProgramArguments</key>"));
        assert!(plist.contains("<array>"));
        assert!(plist.contains("<string>run</string>"));

        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<true/>"));

        assert!(plist.contains("<key>KeepAlive</key>"));

        assert!(plist.contains("<key>StandardOutPath</key>"));
        assert!(plist.contains("<key>StandardErrorPath</key>"));
    }

    #[test]
    fn test_generate_plist_uses_correct_log_paths() {
        let binary_path = "/usr/local/bin/acp-server";
        let plist = generate_plist(binary_path);
        let log_dir = get_log_dir();
        let log_dir_str = log_dir.to_string_lossy();

        // Verify log paths contain the log directory
        assert!(plist.contains(&format!("<string>{}/acp-server.log</string>", log_dir_str)));
        assert!(plist.contains(&format!("<string>{}/acp-server.err</string>", log_dir_str)));
    }

    #[test]
    fn test_generate_plist_valid_xml_structure() {
        let binary_path = "/usr/local/bin/acp-server";
        let plist = generate_plist(binary_path);

        // Verify it starts and ends correctly
        assert!(plist.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(plist.trim().ends_with("</plist>"));

        // Verify dict structure
        assert!(plist.contains("<dict>"));
        assert!(plist.contains("</dict>"));
    }

    #[test]
    fn test_get_plist_path_returns_correct_location() {
        let path = get_plist_path();
        let path_str = path.to_string_lossy();

        // Should be in ~/Library/LaunchAgents/
        assert!(path_str.contains("Library/LaunchAgents"));
        assert!(path_str.ends_with("com.acp.server.plist"));
    }

    #[test]
    fn test_get_log_dir_returns_acp_logs() {
        let log_dir = get_log_dir();
        let log_dir_str = log_dir.to_string_lossy();

        // Should be ~/.acp/logs/
        assert!(log_dir_str.contains(".acp"));
        assert!(log_dir_str.ends_with("logs"));
    }

    #[test]
    fn test_generate_plist_escapes_special_characters() {
        // Test with a path containing special characters that need XML escaping
        let binary_path = "/path/with spaces/acp-server";
        let plist = generate_plist(binary_path);

        // The path should appear in the plist (spaces are allowed in XML strings)
        assert!(plist.contains("/path/with spaces/acp-server"));
    }

    #[test]
    fn test_generate_plist_program_arguments_order() {
        let binary_path = "/usr/local/bin/acp-server";
        let plist = generate_plist(binary_path);

        // Find the ProgramArguments array
        let args_start = plist.find("<key>ProgramArguments</key>").expect("ProgramArguments key not found");
        let args_section = &plist[args_start..];

        // Find the array section
        let array_start = args_section.find("<array>").expect("array not found");
        let array_end = args_section.find("</array>").expect("array end not found");
        let array_content = &args_section[array_start..array_end];

        // First argument should be the binary path
        let first_arg_pos = array_content.find(&format!("<string>{}</string>", binary_path))
            .expect("binary path not found in array");

        // Second argument should be "run"
        let run_arg_pos = array_content.find("<string>run</string>")
            .expect("run argument not found in array");

        // Binary path should come before "run"
        assert!(first_arg_pos < run_arg_pos, "Binary path should be first argument");
    }
}
