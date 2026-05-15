import os
import re

def process_file(path):
    with open(path, "r") as f:
        content = f.read()

    content = re.sub(r'let _ = app.emit\(([^)]*)\);', r'// emit(\1)', content)
    content = re.sub(r'if let Err\(e\) = app.emit\(([^;]*)\);?', r'// emit(\1)', content)

    # For epic_webview_login.rs, there are some AppHandle references
    content = content.replace("app: &tauri::AppHandle", "")
    content = content.replace("close_window();", "close_window();")
    
    with open(path, "w") as f:
        f.write(content)

for root, _, files in os.walk("src/backend"):
    for file in files:
        if file.endswith(".rs"):
            process_file(os.path.join(root, file))
