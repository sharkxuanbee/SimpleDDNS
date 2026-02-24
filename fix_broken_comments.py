
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        new_lines = []
        for line in lines:
            stripped = line.strip()
            # Check for broken comments split by 'for'
            if stripped.startswith('for context'):
                new_lines.append(line.replace('for context', '// for context'))
            elif stripped.startswith('for consistent display order'):
                new_lines.append(line.replace('for consistent display order', '// for consistent display order'))
            else:
                new_lines.append(line)
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.writelines(new_lines)
        print(f"Fixed broken comments in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
