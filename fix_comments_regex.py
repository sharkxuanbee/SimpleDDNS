
import re
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Regex to find comments that might have code after them
        # We look for // followed by anything, then a keyword that suggests code start
        # Common keywords in this file: let, if, match, egui, self, ui, ctx, return, Ok, Err, for, while, loop, unsafe
        
        keywords = ['let', 'if', 'match', 'egui', 'self', 'ui', 'ctx', 'return', 'Ok', 'Err', 'for', 'while', 'loop', 'unsafe', 'pub', 'fn', 'struct', 'impl', 'use', 'mod']
        
        # Pattern: // (content) (keyword)
        # We use a positive lookahead or just capture groups
        
        # Note: This is risky if the keyword is part of the comment text.
        # But given the minified nature, it's likely code.
        
        for kw in keywords:
            # Replace `// ... kw` with `// ...\nkw`
            # We match `//` then non-newlines, then space then kw
            # But we must be careful not to match `https://`
            
            # Using a loop is inefficient but safer to control.
            # Let's use a single regex for all keywords
            
            pattern = r'(//.*?)\s+(' + '|'.join(keywords) + r')\b'
            
            # We need to exclude `https://` and `http://`
            # So we ensure the character before `//` is not `:`? 
            # Or just check if `//` is at start of line or preceded by space/semicolon/brace
            
            # Since I already inserted newlines before `//` in previous steps, 
            # `//` should be at start of line or after newline.
            
            # So pattern: `(?m)^//(.*?)\s+(let|if|...)`
            
            content = re.sub(r'(?m)^(\s*//.*?)\s+(' + '|'.join(keywords) + r')\b', r'\1\n\2', content)
            
            # Also handle `//` that might not be at start of line (if my previous fix missed some)
            # content = re.sub(r'(?<!:)(\s*//.*?)\s+(' + '|'.join(keywords) + r')\b', r'\1\n\2', content)
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"Regex fixed comments in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
