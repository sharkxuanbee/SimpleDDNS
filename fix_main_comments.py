
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Aggressive replacement of '// ' with '\n// '
        # This assumes '// ' (with space) is always a comment start in this file
        # and not part of a string (except maybe in unlikely cases).
        # We also need to handle cases where it's already on a new line (harmless extra newline)
        
        new_content = content.replace('// ', '\n// ')
        
        # Also fix specific cases where space might be missing or different
        new_content = new_content.replace('//Global', '\n// Global')
        
        if content != new_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(new_content)
            print(f"Aggressively fixed comments in {filepath}")
        else:
            print(f"No changes in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
