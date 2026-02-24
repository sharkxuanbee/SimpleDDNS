
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        new_lines = []
        found = False
        for line in lines:
            # Check for 'for rendering' and 'cached_statuses'
            if 'for rendering' in line and 'cached_statuses:' in line:
                # We split
                parts = line.split('cached_statuses:')
                if len(parts) > 1:
                    comment_part = parts[0].strip() # "for rendering ..."
                    code_part = 'cached_statuses:' + parts[1] # "cached_statuses: ..."
                    
                    # Ensure comment starts with //
                    if not comment_part.startswith('//'):
                         comment_part = '// ' + comment_part
                    
                    new_lines.append(f"{comment_part}\n")
                    new_lines.append(f"    {code_part}") # Preserve newline from original line (part[1] has it?)
                    found = True
                    print(f"Fixed: {line.strip()}")
                else:
                    new_lines.append(line)
            else:
                new_lines.append(line)
        
        if found:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.writelines(new_lines)
            print(f"Fixed broken comments v3 (robust) in {filepath}")
        else:
            print("No 'for rendering' line found to fix")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
