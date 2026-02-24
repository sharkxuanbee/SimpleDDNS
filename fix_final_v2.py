
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        new_lines = []
        for line in lines:
            stripped = line.strip()
            
            # Helper to split comment from code
            def split_comment(line, comment_start, code_keyword):
                if comment_start in line and code_keyword in line:
                    parts = line.split(code_keyword, 1) # split on first occurrence of keyword? 
                    # No, keyword might be part of comment?
                    # But here keywords are `let`, `fonts`, `statuses`.
                    
                    # Safer: replace `comment_start` with `// comment_start\n` 
                    # AND ensure code follows.
                    
                    # But we already have `// loop in background` (from previous script?)
                    # No, previous script `new_lines.append(f"// {line}")`
                    # So line became `// loop in background     let statuses_clone ...`
                    # So the whole line is commented out!
                    
                    # We need to uncomment the code part.
                    # The line starts with `// `.
                    # We need to find the code part and move it to new line.
                    pass
            
            # Check if line is commented out by previous script
            if stripped.startswith('// loop in background'):
                # It contains `let statuses_clone`?
                if 'let statuses_clone' in line:
                    # Split
                    line = line.replace('let statuses_clone', '\n    let statuses_clone')
            
            if stripped.startswith('// for proportional fonts'):
                if 'fonts' in line:
                    # `fonts.families...` starts code?
                    # Or `fonts` is variable?
                    # Line 23: `fonts.families...`
                    # Wait, error says `found fonts`.
                    # So `fonts` is code.
                    if 'fonts.families' in line:
                         line = line.replace('fonts.families', '\n    fonts.families')
                    elif 'fonts' in line and not line.strip().endswith('fonts'):
                         # Maybe `fonts` is start of statement
                         line = line.replace('fonts', '\n    fonts')

            if stripped.startswith('// for headless mode'):
                if 'let args' in line:
                    line = line.replace('let args', '\n    let args')
            
            # Also fix `for proportional fonts` if not commented out yet (if I reverted file or something)
            # But previous script wrote to file.
            
            # Check for `for proportional fonts` (not commented)
            if stripped.startswith('for proportional fonts'):
                # Split
                if 'fonts' in line:
                     parts = line.split('fonts', 1) # Split on first 'fonts'
                     # But 'fonts' is in comment text!
                     # "for proportional fonts"
                     # So we split on SECOND fonts?
                     # Or split on `fonts.`?
                     if 'fonts.' in line:
                         line = line.replace('fonts.', '\n    fonts.')
                         line = '// ' + line
            
            if stripped.startswith('for headless mode'):
                if 'let args' in line:
                    line = line.replace('let args', '\n    let args')
                    line = '// ' + line

            new_lines.append(line)
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.writelines(new_lines)
        print(f"Fixed final v2 issues in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
