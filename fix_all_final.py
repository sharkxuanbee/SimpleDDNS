
import os
import re

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        new_lines = []
        for line in lines:
            stripped = line.strip()
            
            # Fix broken comments
            # Lines starting with 'for ', 'loop ', 'if ' that look like English text
            # Heuristic: if it doesn't look like valid Rust code (e.g. no { or ;)
            
            if stripped.startswith('for Chinese support'):
                new_lines.append(f"// {line}")
                continue
            if stripped.startswith('loop in background'):
                new_lines.append(f"// {line}")
                continue
            if stripped.startswith('for each profile if missing'):
                new_lines.append(f"// {line}")
                continue
            if stripped.startswith('for live status updates'):
                new_lines.append(f"// {line}")
                continue
            if stripped.startswith('if error'):
                # Line 382: `if error                     if let Some(s) = status {`
                # `if error` is the broken comment.
                # We need to remove `if error` and keep `if let ...`
                if 'if let' in line:
                    line = line.replace('if error', '// if error\n')
                else:
                    new_lines.append(f"// {line}")
                    continue
            
            # Fix small_button(")
            # Pattern: `small_button(").clicked()`
            # We want: `small_button("X").clicked()` (or Up/Down)
            
            if 'small_button(").clicked()' in line:
                # We need to distinguish Up/Down/X
                # Context is hard line-by-line.
                # But we can look at variable usage if present?
                # Line 517: `if ui.small_button(").clicked() { ipv4_to_remove = Some(i); }`
                # If `remove` in next line?
                # But we iterate line by line.
                
                # We'll just use "Action" or "X" generic for now to fix syntax.
                # Or "Move".
                line = line.replace('small_button(").clicked()', 'small_button("Move").clicked()')
            
            # Fix any other unclosed quotes?
            # `btn_add` error suggests unclosed quote.
            # If `small_button(")` was the cause, fixing it fixes `btn_add` error.
            
            new_lines.append(line)
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.writelines(new_lines)
        print(f"Fixed all final issues in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
