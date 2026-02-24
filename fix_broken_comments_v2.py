
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        new_lines = []
        for line in lines:
            # We look for lines starting with // for context or // for consistent...
            # And split them if code follows.
            
            # Since previous script replaced `for context` with `// for context`,
            # we now have `// for context ...code...`
            
            # We can use regex to split
            
            import re
            
            # Check for `// for context` followed by non-whitespace
            # Actually, previous script just did text replacement.
            # So line starts with `// for context`.
            
            if '// for context' in line and 'if let' in line:
                 line = line.replace('// for context', '// for context\n')
            
            if '// for consistent display order' in line and 'let mut' in line:
                 line = line.replace('// for consistent display order', '// for consistent display order\n')
            
            new_lines.append(line)
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.writelines(new_lines)
        print(f"Fixed broken comments with newlines in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
