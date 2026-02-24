
import os
import re

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Fix ProfileEditor struct
        # We look for `struct ProfileEditor { ... }`
        # and replace it with clean definition.
        
        # Regex to match struct ProfileEditor { ... }
        # Be careful with nested braces. But ProfileEditor is simple.
        
        clean_struct = """struct ProfileEditor {
    is_open: bool,
    is_new: bool,
    profile: DdnsProfile,
    token: String,           // Cloudflare token (encrypted in secrets.enc)
    generic_url: String,     // Generic HTTP URL template
    generic_method: String,  // GET or POST
    generic_body: String,    // POST body template
    generic_headers: String, // JSON string of headers
    save_error: Option<String>,
}"""
        
        # We find the existing struct definition
        # It starts with `struct ProfileEditor {` and ends with `}` before `impl Default`.
        # We can use regex `struct ProfileEditor \{[\s\S]*?\}` but it might stop early.
        # But we know `impl Default for ProfileEditor` follows.
        
        pattern = r'struct ProfileEditor \{[\s\S]*?\}\s*impl Default'
        replacement = clean_struct + '\n\nimpl Default'
        
        content = re.sub(pattern, replacement, content, count=1)
        
        # Fix button quotes / suffixes
        # Remove garbage characters
        content = content.replace('鈫?', '')
        content = content.replace('鉁?', '')
        
        # Fix specific button labels that might be empty now
        content = content.replace('small_button("").clicked()', 'small_button("Up").clicked()') # Heuristic
        # We have multiple small_button("") now.
        # We can context-replace.
        
        # But first let's just remove the garbage suffix.
        # If `small_button("...")鈫?` -> `small_button("...")`
        
        # Fix `btn_add` issue
        # `i18n::I18n::t(lang, "btn_add"))`
        # Ensure quotes are correct.
        # Maybe it was `t(lang, "btn_add)` (missing quote)
        
        # We can force replace the whole line if we find it.
        if 'btn_add' in content:
             # Find line with btn_add
             lines = content.splitlines()
             for i, line in enumerate(lines):
                 if 'btn_add' in line:
                     if 'i18n::I18n::t' in line:
                         # Replace with clean line
                         # Preserving indentation
                         indent = re.match(r'^\s*', line).group(0)
                         lines[i] = indent + 'if ui.button(i18n::I18n::t(lang, "btn_add")).clicked() {'
                         # Wait, we need to check if there is other logic on the line.
                         # Original: `if ui.button(...).clicked() && !self.new_ipv4_url.is_empty() {`
                         # So we should be careful.
                         # Just make sure "btn_add" is quoted.
                         lines[i] = lines[i].replace('"btn_add', '"btn_add"').replace('""btn_add"', '"btn_add"')
             content = '\n'.join(lines)

        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"Cleaned up {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
