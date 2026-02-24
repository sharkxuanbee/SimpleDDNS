
import os
import re

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Look for the specific pattern of unclosed match block in CentralPanel
        # match self.active_tab { ... _ => {} });
        
        # We look for _ => {} followed by }); (possibly with whitespace)
        # Regex: _ => \{\}\s*\}\);
        
        # We want to insert a } before });
        
        new_content = re.sub(r'(_ => \{\})\s*(\}\);)', r'\1\n        }\n        \2', content)
        
        if content != new_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(new_content)
            print(f"Fixed match brace in {filepath}")
        else:
            print(f"No match brace fix needed in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
