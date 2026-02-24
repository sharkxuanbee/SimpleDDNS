
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Replace specific broken comment lines
        # We use replace on content directly to avoid line splitting issues
        
        # Line 22: for proportional fonts (UI text)
        if 'for proportional fonts (UI text)' in content:
             content = content.replace('for proportional fonts (UI text)', '// for proportional fonts (UI text)\n')
        
        # Line 83: for headless mode
        if 'for headless mode' in content:
             content = content.replace('for headless mode', '// for headless mode\n')
             
        # Also fix indentation if needed (the newline makes next line start at col 0)
        # But Rustfmt or compiler doesn't care about indentation for correctness usually.
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"Fixed final v3 issues in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
