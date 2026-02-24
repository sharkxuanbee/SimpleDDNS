
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        new_lines = []
        for line in lines:
            # Fix double quotes for btn_add
            if '"btn_add""' in line:
                line = line.replace('"btn_add""', '"btn_add"')
            
            # Fix unknown prefix S
            # t.format("%H:%M:%S")
            # If it became `t.format("%H:%M:"S)` or similar
            if 't.format(' in line and '%H:%M:%S' in line:
                # Ensure it is quoted correctly
                # Replace with clean version
                # preserve indentation
                import re
                indent = re.match(r'^\s*', line).group(0)
                line = indent + 'ui.label(format!("Last: {}", t.format("%H:%M:%S")));\n'
            
            # Fix unknown prefix IPv6
            # `if ui.button("+ IPv6").clicked() {`
            if '+ IPv6' in line:
                # Ensure quotes
                if '"+ IPv6"' not in line: # if broken
                     line = line.replace('+ IPv6', '"+ IPv6"') # heuristic
                     # But be careful not to double quote if partially quoted
                     # Better: replace the whole button call if possible
                     pass
                # Check if it has extra quotes
                # `"+ IPv6"` is correct.
                # Error says `unknown prefix IPv6`.
                # This implies `identifier "string"`.
                # Maybe `+ "IPv6"`?
                # Or `"+ "IPv6`.
                # If `fix_quotes.py` or `cleanup_main.py` did something.
                
                # I'll just replace the whole line if it matches the pattern
                if 'if ui.button' in line and 'IPv6' in line:
                     indent = re.match(r'^\s*', line).group(0)
                     line = indent + 'if ui.button("+ IPv6").clicked() {\n'

            # Fix unknown prefix IPv4
            if 'if ui.button' in line and 'IPv4' in line and '+' in line:
                 indent = re.match(r'^\s*', line).group(0)
                 line = indent + 'if ui.button("+ IPv4").clicked() {\n'

            new_lines.append(line)
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.writelines(new_lines)
        print(f"Fixed quotes v2 in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
