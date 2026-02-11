#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};

fn main() {
  let quit = CustomMenuItem::new("quit".to_string(), "Quit FlashSpeech");
  let toggle = CustomMenuItem::new("toggle".to_string(), "Show/Hide HUD");
  
  let tray_menu = SystemTrayMenu::new()
    .add_item(toggle)
    .add_native_item(SystemTrayMenuItem::Separator)
    .add_item(quit);

  let system_tray = SystemTray::new()
    .with_menu(tray_menu);

  tauri::Builder::default()
    .system_tray(system_tray)
    .on_system_tray_event(|app, event| match event {
      SystemTrayEvent::LeftClick {
        position: _,
        size: _,
        ..
      } => {
        let window = app.get_window("main").unwrap();
        if window.is_visible().unwrap() {
            window.hide().unwrap();
        } else {
            window.show().unwrap();
            window.set_focus().unwrap();
        }
      }
      SystemTrayEvent::MenuItemClick { id, .. } => {
        match id.as_str() {
          "quit" => {
            std::process::exit(0);
          }
          "toggle" => {
            let window = app.get_window("main").unwrap();
            if window.is_visible().unwrap() {
                window.hide().unwrap();
            } else {
                window.show().unwrap();
                window.set_focus().unwrap();
            }
          }
          _ => {}
        }
      }
      _ => {}
    })
    .setup(|app| {
        // Set activation policy to Accessory to hide from Dock on macOS
        #[cfg(target_os = "macos")]
        app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        
        let window = app.get_window("main").unwrap();
        // Ensure transparency is enforced
        #[cfg(target_os = "macos")]
        {
            use cocoa::appkit::{NSWindow, NSColor};
            use cocoa::base::{id, nil, NO};
            
            let ns_window = window.ns_window().unwrap() as id;
            unsafe {
                ns_window.setHasShadow_(NO);
                ns_window.setOpaque_(NO);
                ns_window.setBackgroundColor_(NSColor::clearColor(nil)); 
            }
        }
        
        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
