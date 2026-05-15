import os
import re

def process_file(path):
    with open(path, "r") as f:
        content = f.read()

    # Remove tauri imports
    content = re.sub(r'use tauri::[^\n]*\n', '', content)
    content = re.sub(r'use tauri_plugin_store::[^\n]*\n', '', content)
    
    # Remove <R: Runtime>
    content = re.sub(r'<R:\s*Runtime>', '', content)
    
    # Remove app: &AppHandle<R>, and app: AppHandle<R>,
    content = re.sub(r'app:\s*&?AppHandle(?:<R>)?,\s*', '', content)
    content = re.sub(r'app:\s*&?AppHandle(?:<R>)?\s*', '', content)
    
    # Replace tauri::async_runtime::spawn_blocking with tokio::task::spawn_blocking
    content = content.replace("tauri::async_runtime::spawn_blocking", "tokio::task::spawn_blocking")
    
    # Replace tauri::async_runtime::spawn with tokio::spawn
    content = content.replace("tauri::async_runtime::spawn", "tokio::spawn")
    
    # Fix leftover calls that pass app
    content = re.sub(r'\(\s*app,\s*', '(', content)
    content = re.sub(r'\(\s*&app,\s*', '(', content)
    content = re.sub(r'\(\s*app\s*\)', '()', content)
    content = re.sub(r'\(\s*&app\s*\)', '()', content)
    
    # Fix Some specific function calls inside profile_service like get_profiles_dir(app)?
    content = content.replace("get_profiles_dir(app)", "get_profiles_dir()")
    content = content.replace("get_profiles_dir(&app)", "get_profiles_dir()")

    with open(path, "w") as f:
        f.write(content)

for root, _, files in os.walk("src/backend"):
    for file in files:
        if file.endswith(".rs"):
            process_file(os.path.join(root, file))
