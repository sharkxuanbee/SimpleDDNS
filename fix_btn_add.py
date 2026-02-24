
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Replace "btn_add"" with "btn_add"
        if '"btn_add""' in content:
            content = content.replace('"btn_add""', '"btn_add"')
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print("Fixed btn_add double quote")
        else:
            print("btn_add double quote not found")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
