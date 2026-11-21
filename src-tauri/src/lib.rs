pub mod attachments;
pub mod catalog;
mod commands;
pub mod db;
pub mod download;
mod image_commands;
pub mod imagegen;
pub mod error;
pub mod hardware;
pub mod intent;
pub mod ollama;
pub mod recommend;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let db = db::Db::open(&dir.join("baseio.db"))?;
            app.manage(state::AppState {
                ollama: ollama::Ollama::new(),
                http: reqwest::Client::new(),
                db,
                installs: Default::default(),
                chats: Default::default(),
                image_gen: Default::default(),
                image_paths: imagegen::Paths { root: dir.clone() },
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_hardware_info,
            commands::get_catalog,
            commands::assess_catalog,
            commands::get_recommendations,
            commands::runtime_status,
            commands::start_runtime,
            commands::list_installed,
            commands::list_running,
            commands::load_model,
            commands::unload_model,
            commands::delete_model,
            commands::prepare_install,
            commands::install_model,
            commands::cancel_install,
            commands::active_installs,
            commands::list_conversations,
            commands::create_conversation,
            commands::get_messages,
            commands::delete_conversation,
            commands::send_message,
            commands::cancel_chat,
            commands::interpret_request,
            commands::set_conversation_model,
            commands::model_info,
            commands::read_attachment,
            commands::attachment_extensions,
            image_commands::image_overview,
            image_commands::image_license_accepted,
            image_commands::install_image_model,
            image_commands::cancel_image_install,
            image_commands::delete_image_model,
            image_commands::generate_image,
            image_commands::cancel_image_generation,
            image_commands::list_generated_images,
            image_commands::image_data,
            image_commands::export_image,
            image_commands::delete_generated_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
