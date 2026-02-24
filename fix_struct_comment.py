
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        new_lines = []
        for line in lines:
            if '// Settings editor' in line and 'new_ipv4_url' in line:
                # Split comment
                parts = line.split('new_ipv4_url')
                comment_part = parts[0].strip()
                code_part = 'new_ipv4_url' + parts[1]
                
                if not comment_part.startswith('//'):
                     comment_part = '// ' + comment_part
                
                new_lines.append(f"{comment_part}\n")
                new_lines.append(f"    {code_part}")
            else:
                new_lines.append(line)
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.writelines(new_lines)
        print(f"Fixed struct comment in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
